use regex::Regex;
use chrono::{NaiveDate, ParseError};
use reqwest;
use reqwest::Response;
use std::io::Read;
use select::document::Document;
use select::predicate::{Name, Attr};
use rustc_serialize::json::Json;
use errors::*;

// Some formats borrowed from https://github.com/amir/article-date-extractor
static FMTS: &'static [&str] = &["/%Y/%m/%d/", "/%Y/%d/%m/", "%Y-%m-%d", "%B %e, %Y", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%SZ", "%B %k, %Y, %H:%M %p", "%Y-%m-%d %H:%M:%S.000000"];

// Use lazy_static to ensure we only compile the regex once
lazy_static! {
    // Regex by Newspaper3k  - https://github.com/codelucas/newspaper/blob/master/newspaper/urls.py
    static ref RE: Regex = Regex::new(r"([\./\-_]{0,1}(19|20)\d{2})[\./\-_]{0,1}(([0-3]{0,1}[0-9][\./\-_])|(\w{3,5}[\./\-_]))([0-3]{0,1}[0-9][\./\-]{0,1})?").unwrap();
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
    let mut date: Option<String> = None;

    if let Some(val) = RE.find(url) {
        date = Some(val.as_str().to_string());
    }

    date
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
}
// Unit tests
#[cfg(test)]
mod test {
    use super::extract_from_url;
    use super::parse_date;
    use super::extract_from_meta;
    use super::extract_from_ldjson;
    use chrono::{NaiveDate};
    use reqwest;
    use std::string::String;
    use std::io::Read;
    use select::document::Document;

    #[test]
    fn parsing_date() {
        assert_eq!(parse_date("/2015/11/30/"), Ok(NaiveDate::from_ymd(2015,11,30)));
        assert_eq!(parse_date("/2015/30/11/"), Ok(NaiveDate::from_ymd(2015,11,30)));
    }

    #[test]
    fn extracting_from_url() {
        let link = "http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html";
        assert_eq!(extract_from_url(link), parse_date("/2015/11/28/"));

        let link = "";
        assert_eq!(extract_from_url(link), parse_date(""));
    }

    #[test]
    fn extracting_from_meta() {
        let mut response = reqwest::get("https://techcrunch.com/2015/11/30/atlassian-share-price/").unwrap();
        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();
        let document = Document::from(body.as_str());

        assert_eq!(Ok(NaiveDate::from_ymd(2015,11,30)), extract_from_meta(&document));
    }

    #[test]
    fn extracting_from_ldjson() {
        let mut response = reqwest::get("https://techcrunch.com/2015/11/30/atlassian-share-price/").unwrap();
        let mut body = String::new();
        response.read_to_string(&mut body).unwrap();
        let document = Document::from(body.as_str());

        assert_eq!(Ok(NaiveDate::from_ymd(2015,12,01)), extract_from_ldjson(&document));
        // println!("{:?}", extract_from_ldjson(&document));
        // println!("{:?}", parse_date("2015-12-01T07:50:48Z"));
        // println!("{:?}", parse_date(extract_from_ldjson(&document).trim()));
    }
}
