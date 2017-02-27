use regex::Regex;
use chrono::{NaiveDate, ParseError};

// Some formats borrowed from https://github.com/amir/article-date-extractor
static FMTS: &'static [&str] = &["/%Y/%d/%m/", "/%Y/%m/%d/", "%Y-%m-%d", "%B %e, %Y", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S%Z", "%B %k, %Y, %H:%M %p", "%Y-%m-%d %H:%M:%S.000000"];

// Parse the date, trying out each format
// TODO: It's probably better to stop iterating once a parse is successful
fn parse_date(input: &str) -> Result<NaiveDate, ParseError> {
    let parse_attempt: Vec<Result<NaiveDate,ParseError>> = FMTS.iter().map(|&fmt| NaiveDate::parse_from_str(input, fmt)).collect::<Vec<Result<NaiveDate, ParseError>>>();
    try_filter_dates(parse_attempt)
}

// Attempt to find a successful parse, if there are none, return the first error encountered
fn try_filter_dates(vec: Vec<Result<NaiveDate, ParseError>>) -> Result<NaiveDate, ParseError> {
    let orig_vec: Vec<Result<NaiveDate, ParseError>> = vec.clone();
    let filtered: Vec<Result<NaiveDate, ParseError>> = vec.into_iter().filter(|&item| item.is_ok()).collect::<Vec<Result<NaiveDate, ParseError>>>();
    match filtered.first() {
        Some(v) => v.to_owned(),
        None => orig_vec.first().unwrap().to_owned(),
    }
}

// Extract date from a URL
fn extract_from_url(url: &str) -> Result<String, String> {
    // Use lazy_static to ensure we only compile the regex once
    lazy_static! {
       // Regex by Newspaper3k  - https://github.com/codelucas/newspaper/blob/master/newspaper/urls.py
       static ref RE: Regex = Regex::new(r"([\./\-_]{0,1}(19|20)\d{2})[\./\-_]{0,1}(([0-3]{0,1}[0-9][\./\-_])|(\w{3,5}[\./\-_]))([0-3]{0,1}[0-9][\./\-]{0,1})?").unwrap();
    }
    match RE.find(url) {
        Some(v) => Ok(String::from(v.as_str())),
        None => Err(String::from("Couldn't extract a url")),
    }
}

#[cfg(test)]
mod test {
    use super::extract_from_url;
    use super::parse_date;
    use chrono::{NaiveDate};
    #[test]
    fn extracting_date() {
        let link = "http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html";
        assert_eq!(extract_from_url(link), Ok(String::from("/2015/11/28/")));

        let link = "";
        assert_eq!(extract_from_url(link), Err(String::from("Couldn't extract a url")));
    }

    #[test]
    fn parsing() {
        assert_eq!(parse_date("/2015/11/30/"), Ok(NaiveDate::from_ymd(2015,11,30)));
        assert_eq!(parse_date("/2015/30/11/"), Ok(NaiveDate::from_ymd(2015,11,30)));
    }
}
