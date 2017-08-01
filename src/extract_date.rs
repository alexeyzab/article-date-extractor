use regex::Regex;
use chrono::NaiveDate;
use select::document::Document;
use select::predicate::{Name, Attr};
use serde_json;
use serde_json::Value;
use errors::*;

// Some formats borrowed from https://github.com/amir/article-date-extractor
static FMTS: &[&str] = &["%A, %B %e, %Y",
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

fn parse_date(input: &str) -> Result<NaiveDate> {
    FMTS.iter()
        .flat_map(|fmt| NaiveDate::parse_from_str(input, fmt))
        .next()
        .ok_or("None of the formats matched the date".into())
}

fn extract_from_url(url: &str) -> Option<String> {
    RE.find(url).map(|val| val.as_str().to_string())
}

fn extract_from_ldjson<'a>(html: &'a Document) -> Option<String> {
    html.find(Attr("type", "application/ld+json"))
        .next()
        .map(|ldj| ldj.text())
        .and_then(|ldjson| serde_json::from_str(&ldjson).ok())
        .and_then(|decoded_ldjson: Value| {
            let published = decoded_ldjson
                .get("datePublished")
                .and_then(|date| date.as_str())
                .map(|date| date.to_string());

            let created = decoded_ldjson
                .get("dateCreated")
                .and_then(|date| date.as_str())
                .map(|date| date.to_string());

            published.or(created)
        })        
}

fn meta_name_denotes_date(meta_name: &str) -> bool {
    match meta_name.to_lowercase().as_str() {
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
        "date_published" => true,
        _ => false,
    }
}

fn meta_itemprop_denotes_date(item_prop: &str) -> bool {
    match item_prop.to_lowercase().as_str() {
        "datepublished" | "datecreated" => true,
        _ => false,
    }
}

fn meta_http_equiv_denotes_date(http_equiv: &str) -> bool {
    match http_equiv.to_lowercase().as_str() {
        "date" => true,
        _ => false,
    }
}

fn meta_property_denotes_date(meta_property: &str) -> bool {
    match meta_property {
        "article:published_time" |
        "bt:pubdate" => true,
        _ => false,
    }
}

fn extract_from_meta<'a>(html: &'a Document) -> Option<String> {
    html.find(Name("meta")).flat_map(|meta| {
        let content = match meta.attr("content") {
            Some(c) => c,
            None => return None,
        };
        let content = content.trim();

        let meta_name = meta.attr("name");
        let item_prop = meta.attr("itemprop");
        let http_equiv = meta.attr("http-equiv");
        let meta_property = meta.attr("property");

        let content_has_date = meta_name.map(meta_name_denotes_date)
            .or_else(|| item_prop.map(meta_itemprop_denotes_date))
            .or_else(|| http_equiv.map(meta_http_equiv_denotes_date))
            .or_else(|| meta_property.map(meta_property_denotes_date))
            .unwrap_or(false);

        if content_has_date {
            Some(content.to_string())
        } else if Some("og:image") == meta_property {
            extract_from_url(content)
        } else {
            None
        }
    }).next()
}

fn extract_time_tag<'a>(html: &'a Document) -> Option<String> {
    html.find(Name("time")).flat_map(|time| {
        if time.attr("class") == Some("timestamp") {
            Some(time.text().trim_matches('\n').to_string())
        } else {
            time.attr("datetime")
                .and_then(|dt| Some(dt.to_string()))
        }
    }).next()
}

fn extract_span_date_published<'a>(html: &'a Document) -> Option<String> {
    html.find(Name("span")).flat_map(|tag| {
        if tag.attr("itemprop") == Some("datePublished") {
            tag.attr("content").map(|v| v.to_string())
        } else if !tag.text().is_empty() && tag.attr("itemprop") == Some("datePublished") {
            Some(tag.text().trim_matches('\n').to_string())
        } else {
            None
        }
    }).next()
}

fn extract_from_tag_with_regex<'a>(html: &'a Document, reg: &Regex, tag: &str) -> Option<String> {
    html.find(Name(tag)).flat_map(|t| {
        t.attr("class").and_then(|v| {
            if reg.is_match(v) {
                Some(t.text().trim_matches('\n').to_string())
            } else {
                None
            }
        })
    }).next()
}

fn extract_from_html_tag<'a>(html: &'a Document) -> Option<String> {
    lazy_static! {
        static ref TAG_RE: Regex =
            Regex::new(r"(?i)publishdate|pubdate|timestamp|article_date|articledate|date").unwrap();
    }

    extract_time_tag(html)
        .or_else(|| extract_span_date_published(html))
        .or_else(|| extract_from_tag_with_regex(html, &TAG_RE, "span"))
        .or_else(|| extract_from_tag_with_regex(html, &TAG_RE, "p"))
        .or_else(|| extract_from_tag_with_regex(html, &TAG_RE, "div"))
        .or(None)
}

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
pub fn extract_article_published_date(link: &str, html: &str) -> Result<NaiveDate> {
    let doc = Document::from(html);

    extract_from_url(link)
        .or_else(|| extract_from_ldjson(&doc))
        .or_else(|| extract_from_meta(&doc))
        .or_else(|| extract_from_html_tag(&doc))
        .ok_or("Couldn't find the date to parse".into())
        .and_then(|v| parse_date(&v))
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
        let link = "http://edition.cnn.\
                          com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.\
                          html";
        assert_eq!(Some("/2015/11/28/".to_string()), extract_from_url(link));

        let link = "";
        assert_eq!(None, extract_from_url(link));
    }

    #[test]
    fn extracting_from_ldjson() {
        let document = Document::from(include_str!("../tests/fixtures/techcrunch.html"));

        assert_eq!(Some("2015-12-01T07:50:48Z".to_string()),
                   extract_from_ldjson(&document));
    }

    #[test]
    fn extracting_from_meta() {
        let document = Document::from(include_str!("../tests/fixtures/techcrunch.html"));

        assert_eq!(Some(("2015-11-30 23:50:48".to_string())),
                   extract_from_meta(&document));
    }

    #[test]
    fn extracting_from_html_tag() {
        let document = Document::from(include_str!("../tests/fixtures/google_blog.html"));

        assert_eq!(Some("Thursday, March 16, 2017".to_string()),
                   extract_from_html_tag(&document));
    }
}
