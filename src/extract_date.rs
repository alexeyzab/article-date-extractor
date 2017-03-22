use regex::Regex;
use chrono::NaiveDate;
use reqwest;
use std::io::Read;
use select::document::Document;
use select::predicate::{Name, Attr};
use rustc_serialize::json::Json;
use errors::*;

// Some formats borrowed from https://github.com/amir/article-date-extractor
static FMTS: &'static [&str] = &["%A, %B %e, %Y",
                                 "%Y-%m-%dT%H:%M:%S%:z",
                                 "/%Y/%m/%d/",
                                 "/%Y/%d/%m/",
                                 "%Y-%m-%d",
                                 "%B %e, %Y",
                                 "%Y-%m-%d %H:%M:%S",
                                 "%Y-%m-%dT%H:%M:%SZ",
                                 "%B %k, %Y, %H:%M %p",
                                 "%Y-%m-%d %H:%M:%S.000000"];

// Use lazy_static to ensure we only compile the regex once
lazy_static! {
    // Regex by Newspaper3k  - https://github.com/codelucas/newspaper/blob/master/newspaper/urls.py
    static ref RE: Regex =
        Regex::new(r"([\./\-_]{0,1}(19|20)\d{2})[\./\-_]{0,1}(([0-3]{0,1}[0-9][\./\-_])|(\w{3,5}[\./\-_]))([0-3]{0,1}[0-9][\./\-]{0,1})").unwrap();
}

// Parse the date, trying out each format
fn parse_date(input: &str) -> Result<NaiveDate> {
    let mut result: Result<NaiveDate> = Err("None of the formats matched the date".into());

    'outer: for fmt in FMTS {
        if let Ok(v) = NaiveDate::parse_from_str(input, fmt) {
            {
                result = Ok(v);
                break 'outer;
            }
        }
    }

    result
}

// Extract date from a URL
fn extract_from_url(url: &str) -> Option<String> {
    if let Some(val) = RE.find(url) {
        return Some(val.as_str().to_string());
    } else {
        return None;
    }
}

// Extract date from JSON-LD
fn extract_from_ldjson<'a>(html: &'a Document) -> Option<String> {
    let mut json_date: Option<String> = None;
    let mut _ldjson: String = String::new();
    if let Some(ldj) = html.find(Attr("type", "application/ld+json")).next() {
        _ldjson = ldj.text();
    } else {
        return None;
    }

    let mut _decoded_ldjson: Json = Json::from_str("{}").unwrap();

    match Json::from_str(&_ldjson) {
        Ok(v) => _decoded_ldjson = v,
        _ => return None,
    }

    if let Some(date_published) = _decoded_ldjson.search("datePublished") {
        if let Some(date) = date_published.as_string() {
            json_date = Some(date.to_string())
        }
    } else if let Some(date_created) = _decoded_ldjson.search("dateCreated") {
        if let Some(date) = date_created.as_string() {
            json_date = Some(date.to_string())
        }
    }

    json_date
}

// Extract date from meta tags
fn extract_from_meta<'a>(html: &'a Document) -> Option<String> {
    let mut meta_date: Option<String> = None;

    'outer: for meta in html.find(Name("meta")) {
        let meta_name: Option<&str> = meta.attr("name");
        let item_prop: Option<&str> = meta.attr("itemprop");
        let http_equiv: Option<&str> = meta.attr("http-equiv");
        let meta_property: Option<&str> = meta.attr("property");

        if let Some(v) = meta_name {
            match v.to_lowercase().as_ref() {
                "pubdate" |
                "publishdate" |
                "timestamp" |
                "dc.date.issued" |
                "date" |
                "sailthru.date" |
                "article.published" |
                "published-date" |
                "article.created" |
                "article_date_original" |
                "cxenseparse:recs:publishtime" |
                "date_published" => {
                    if let Some(ct) = meta.attr("content") {
                        {
                            meta_date = Some(ct.trim().to_string());
                            break 'outer;
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(v) = item_prop {
            match v.to_lowercase().as_ref() {
                "datepublished" | "datecreated" => {
                    if let Some(ct) = meta.attr("content") {
                        {
                            meta_date = Some(ct.trim().to_string());
                            break 'outer;
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(v) = http_equiv {
            match v.to_lowercase().as_ref() {
                "date" => {
                    if let Some(ct) = meta.attr("content") {
                        {
                            meta_date = Some(ct.trim().to_string());
                            break 'outer;
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(v) = meta_property {
            match v.as_ref() {
                "article:published_time" |
                "bt:pubdate" => {
                    if let Some(ct) = meta.attr("content") {
                        {
                            meta_date = Some(ct.trim().to_string());
                            break 'outer;
                        }
                    }
                }
                "og:image" => {
                    if let Some(url) = meta.attr("content") {
                        {
                            meta_date = extract_from_url(url.trim());
                            break 'outer;
                        }
                    }
                }

                _ => {}
            }
        }


    }

    meta_date
}

// Extract from html tags
fn extract_from_html_tag<'a>(html: &'a Document) -> Option<String> {
    lazy_static! {
        static ref TAG_RE: Regex =
            Regex::new(r"(?i)publishdate|pubdate|timestamp|article_date|articledate|date").unwrap();
    }

    let mut date: Option<String> = None;

    'initial: for time in html.find(Name("time")) {
        if let Some(dt) = time.attr("datetime") {
            {
                date = Some(dt.to_string());
                break 'initial;
            }
        } else if let Some("timestamp") = time.attr("class") {
            {
                date = Some(time.text().trim_matches('\n').to_string());
                break 'initial;
            }
        }
    }

    if date.is_none() {
        'outer: for tag in html.find(Name("span")) {
            if let Some("datePublished") = tag.attr("itemprop") {
                if let Some(v) = tag.attr("content") {
                    {
                        date = Some(v.to_string());
                        break 'outer;
                    }
                } else if !tag.text().is_empty() {
                    {
                        date = Some(tag.text().trim_matches('\n').to_string());
                        break 'outer;
                    }
                }
            }
        }
    }

    // These next three loops are due to the lack of `find_all` method for select.rs library
    if date.is_none() {
        'outer_first: for tag in html.find(Name("span")) {
            if TAG_RE.is_match(tag.attr("class").unwrap_or("")) {
                {
                    date = Some(tag.text().trim_matches('\n').to_string());
                    break 'outer_first;
                }
            }
        }
    }

    if date.is_none() {
        'outer_second: for tag in html.find(Name("p")) {
            if TAG_RE.is_match(tag.attr("class").unwrap_or("")) {
                {
                    date = Some(tag.text().trim_matches('\n').to_string());
                    break 'outer_second;
                }
            }
        }
    }

    if date.is_none() {
        'outer_third: for tag in html.find(Name("div")) {
            if TAG_RE.is_match(tag.attr("class").unwrap_or("")) {
                {
                    date = Some(tag.text().trim_matches('\n').to_string());
                    break 'outer_third;
                }
            }
        }
    }

    date
}

// Try to extract the date by using each function one by one
/// This function attempts to extract the article date by using several different methods in a row.
/// The following methods are used: extracting the date from url, JSON-LD, meta tags, additional html tags.
///
/// Supported date formats:
///
///
///"%A, %B %e, %Y"
///
///"%Y-%m-%dT%H:%M:%S%:z"
///
///"/%Y/%m/%d/"
///
///"/%Y/%d/%m/"
///
///"%Y-%m-%d"
///
///"%B %e, %Y"
///
///"%Y-%m-%d %H:%M:%S"
///
///"%Y-%m-%dT%H:%M:%SZ"
///
///"%B %k, %Y, %H:%M %p"
///
///"%Y-%m-%d %H:%M:%S.000000"
///
pub fn extract_article_published_date(link: &str, html: Option<String>) -> Result<NaiveDate> {
    let mut body: String = String::new();
    let mut _parsed_body: Option<Document> = None;

    if let Some(v) = extract_from_url(link) {
        return parse_date(&v);
    }

    if html.is_none() {
        if let Ok(mut response) = reqwest::get(link) {
            response.read_to_string(&mut body).unwrap();
            let doc = Document::from(body.as_str());
            _parsed_body = Some(doc);
        } else {
            return Err("Couldn't open the link".into());
        }
    } else {
        _parsed_body = Some(Document::from(html.unwrap().as_str()))
    }

    if let Some(v) = extract_from_url(link) {
        return parse_date(&v);
    } else if let Some(v) = extract_from_ldjson(_parsed_body.as_ref().unwrap()) {
        return parse_date(&v);
    } else if let Some(v) = extract_from_meta(_parsed_body.as_ref().unwrap()) {
        return parse_date(&v);
    } else if let Some(v) = extract_from_html_tag(_parsed_body.as_ref().unwrap()) {
        return parse_date(&v);
    } else {
        return Err("Couldn't find the date to parse".into());
    }
}

// Unit tests
#[cfg(test)]
mod test {
    use super::extract_from_url;
    use super::parse_date;
    use super::extract_from_meta;
    use super::extract_from_ldjson;
    use super::extract_from_html_tag;
    use chrono::NaiveDate;
    use reqwest;
    use reqwest::Response;
    use std::io::Read;
    use select::document::Document;

    #[test]
    fn parsing_date() {
        assert_eq!(NaiveDate::from_ymd(2015, 11, 30),
                   parse_date("/2015/11/30/").unwrap());
        assert_eq!(NaiveDate::from_ymd(2015, 11, 30),
                   parse_date("/2015/30/11/").unwrap());

        assert!(parse_date("bad_format").is_err());
    }

    #[test]
    fn extracting_from_url() {
        let link: &str = "http://edition.cnn.\
                          com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.\
                          html";
        assert_eq!(Some("/2015/11/28/".to_string()), extract_from_url(link));

        let link: &str = "";
        assert_eq!(None, extract_from_url(link));
    }

    #[test]
    fn extracting_from_ldjson() {
        let mut response: Response =
            reqwest::get("https://techcrunch.com/2015/11/30/atlassian-share-price/").unwrap();
        let mut body: String = String::new();
        response.read_to_string(&mut body).unwrap();
        let document: Document = Document::from(body.as_str());

        assert_eq!(Some("2015-12-01T07:50:48Z".to_string()),
                   extract_from_ldjson(&document));
    }

    #[test]
    fn extracting_from_meta() {
        let mut response: Response =
            reqwest::get("https://techcrunch.com/2015/11/30/atlassian-share-price/").unwrap();
        let mut body: String = String::new();
        response.read_to_string(&mut body).unwrap();
        let document: Document = Document::from(body.as_str());

        assert_eq!(Some(("2015-11-30 23:50:48".to_string())),
                   extract_from_meta(&document));
    }

    #[test]
    fn extracting_from_html_tag() {
        let mut response: Response =
            reqwest::get("https://research.googleblog.\
                          com/2017/03/announcing-guetzli-new-open-source-jpeg.html")
                .unwrap();
        let mut body: String = String::new();
        response.read_to_string(&mut body).unwrap();
        let document: Document = Document::from(body.as_str());

        assert_eq!(Some("Thursday, March 16, 2017".to_string()),
                   extract_from_html_tag(&document));
    }
}
