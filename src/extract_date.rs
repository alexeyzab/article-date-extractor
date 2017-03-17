use regex::Regex;
use chrono::NaiveDate;
use reqwest;
use std::io::Read;
use select::document::Document;
use select::predicate::{Name, Attr};
use rustc_serialize::json::Json;
use errors::*;

// Some formats borrowed from https://github.com/amir/article-date-extractor
static FMTS: &'static [&str] = &["%A, %B %e, %Y", "%Y-%m-%dT%H:%M:%S%:z", "/%Y/%m/%d/", "/%Y/%d/%m/", "%Y-%m-%d", "%B %e, %Y", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%SZ", "%B %k, %Y, %H:%M %p", "%Y-%m-%d %H:%M:%S.000000"];

// Use lazy_static to ensure we only compile the regex once
lazy_static! {
    // Regex by Newspaper3k  - https://github.com/codelucas/newspaper/blob/master/newspaper/urls.py
    static ref RE: Regex = Regex::new(r"([\./\-_]{0,1}(19|20)\d{2})[\./\-_]{0,1}(([0-3]{0,1}[0-9][\./\-_])|(\w{3,5}[\./\-_]))([0-3]{0,1}[0-9][\./\-]{0,1})").unwrap();
}

// Parse the date, trying out each format
fn parse_date(input: &str) -> Result<NaiveDate> {
    let mut result: Result<NaiveDate> = Err("None of the formats matched the date".into());

    'outer: for fmt in FMTS {
        if let Ok(v) = NaiveDate::parse_from_str(input, fmt) {
            { result = Ok(v);
              break 'outer; }
        }
    }

    result
}

// Extract date from a URL
fn extract_from_url(url: &str) -> Option<String> {
    if let Some(val) = RE.find(url) {
        return Some(val.as_str().to_string())
    } else {
        return None
    }
}

// Extract date from JSON-LD
fn extract_from_ldjson<'a>(html: &'a Document) -> Option<String> {
    let mut json_date: Option<String> = None;
    let mut ldjson = String::new();
    if let Some(ldj) = html.find(Attr("type", "application/ld+json")).next() {
        ldjson = ldj.text();
    }

    let decoded_ldjson = Json::from_str(ldjson.as_str()).unwrap();

    if let Some(date_published) = decoded_ldjson.search("datePublished") {
        if let Some(date) = date_published.as_string() {
            json_date = Some(date.to_string())
        }
    } else if let Some(date_created) = decoded_ldjson.search("dateCreated") {
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
        let meta_name     = meta.attr("name").unwrap_or("").to_lowercase();
        let item_prop     = meta.attr("itemprop").unwrap_or("").to_lowercase();
        let http_equiv    = meta.attr("http-equiv").unwrap_or("").to_lowercase();
        let meta_property = meta.attr("property").unwrap_or("").to_lowercase();

        match meta_name.as_ref() {
            "pubdate"               | "publishdate"                  | "timestamp"       |
            "dc.date.issued"        | "date"                         | "sailthru.date"   |
            "article.published"     | "published-date"               | "article.created" |
            "article_date_original" | "cxenseparse:recs:publishtime" | "date_published"  => { if let Some(ct) = meta.attr("content") {
                                                                                                  meta_date = Some(ct.trim().to_string())
                                                                                              }
                                                                                              break 'outer; },
            _ => {},
        }

        match item_prop.as_ref() {
            "datepublished" | "datecreated" => { if let Some(ct) = meta.attr("content") {
                                                   meta_date = Some(ct.trim().to_string())
                                                 }
                                                 break 'outer; },
            _ => {},
        }

        match http_equiv.as_ref() {
            "date" =>  { if let Some(ct) = meta.attr("content") {
                           meta_date = Some(ct.trim().to_string())
                         }
                         break 'outer; },
            _ => {},
        }

        match meta_property.as_ref() {
            "article:published_time" | "bt:pubdate" => { if let Some(ct) = meta.attr("content") {
                                                           meta_date = Some(ct.trim().to_string())
                                                         }
                                                         break 'outer; },
            "og:image"                              => { if let Some(url) = meta.attr("content") {
                                                           meta_date = extract_from_url(url.trim())
                                                         }
                                                         break 'outer; },

            _ => {},
        }


    }

    meta_date
}

// Extract from html tags
fn extract_from_html_tag<'a>(html: &'a Document) -> Option<String> {
    let mut date: Option<String> = None;

    for time in html.find(Name("time")) {
        if let Some(dt) = time.attr("datetime") {
            date = Some(dt.to_string())
        } else if let Some("timestamp") = time.attr("class") {
            date = Some(time.text())
        }
    }

    if date.is_none() {
        for tag in html.find(Name("meta")) {
            if let Some("datePublished") = tag.attr("itemprop") {
                if let Some(v) = tag.attr("content") {
                    date = Some(v.to_string())
                } else {
                    date = Some(tag.text())
                }
            }
        }
    }

    date
}

// Try to extract the date by using each function one by one
pub fn extract_article_published_date(link: &str, html: Option<String>) -> Result<NaiveDate> {
    let mut body: String = String::new();
    let mut parsed_body: Option<Document> = None;

    if let Some(v) = extract_from_url(link) {
        return parse_date(v.as_str())
    }

    if html.is_none() {
        if let Ok(mut response) = reqwest::get(link) {
            response.read_to_string(&mut body).unwrap();
            let doc = Document::from(body.as_str());
            parsed_body = Some(doc);
        } else {
            return Err("Couldn't open the link".into())
        }
    } else {
        parsed_body = Some(Document::from(html.unwrap().as_str()))
    }

    if let Some(v) = extract_from_url(link) {
        return parse_date(v.as_str())
    } else if let Some(v) = extract_from_ldjson(parsed_body.as_ref().unwrap()) {
        return parse_date(v.as_str())
    } else if let Some(v) = extract_from_meta(parsed_body.as_ref().unwrap()) {
        return parse_date(v.as_str())
    } else if let Some(v) = extract_from_html_tag(parsed_body.as_ref().unwrap()) {
        return parse_date(v.as_str())
    } else {
        return Err("Couldn't find the date to parse".into())
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
    use super::extract_article_published_date;
    use chrono::NaiveDate;
    use reqwest;
    use std::string::String;
    use std::io::Read;
    use select::document::Document;

    #[test]
    fn parsing_date() {
        assert_eq!(NaiveDate::from_ymd(2015,11,30), parse_date("/2015/11/30/").unwrap());
        assert_eq!(NaiveDate::from_ymd(2015,11,30), parse_date("/2015/30/11/").unwrap());

        assert!(parse_date("bad_format").is_err());
    }

    #[test]
    fn extracting_from_url() {
        let link = "http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html";
        assert_eq!(Some("/2015/11/28/".to_string()), extract_from_url(link));

        let link = "";
        assert_eq!(None, extract_from_url(link));
    }

    #[test]
    fn extracting_from_meta() {
        let mut response = reqwest::get("https://techcrunch.com/2015/11/30/atlassian-share-price/").unwrap();
        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();
        let document = Document::from(body.as_str());

        assert_eq!(Some(("2015-11-30 23:50:48".to_string())), extract_from_meta(&document));
    }

    #[test]
    fn extracting_from_ldjson() {
        let mut response = reqwest::get("https://techcrunch.com/2015/11/30/atlassian-share-price/").unwrap();
        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();
        let document = Document::from(body.as_str());

        assert_eq!(Some("2015-12-01T07:50:48Z".to_string()), extract_from_ldjson(&document));
    }

    #[test]
    fn extracting_from_html_tag() {
        let mut response = reqwest::get("http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html").unwrap();
        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();
        let document = Document::from(body.as_str());

        assert_eq!(Some("2015-11-29T00:44:59Z".to_string()), extract_from_html_tag(&document));
    }

    #[test]
    fn extracting_article_published_date() {
        let link = "http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html";
        let mut response = reqwest::get("http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html").unwrap();
        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();

        assert_eq!(NaiveDate::from_ymd(2015,11,28), extract_article_published_date(&link, None).unwrap());
        assert_eq!(NaiveDate::from_ymd(2015,11,28), extract_article_published_date(&link, Some(body)).unwrap());

        assert!((extract_article_published_date("", None)).is_err());
    }
}
