extern crate article_date_extractor;
extern crate chrono;
extern crate reqwest;

#[test]
fn integration_test() {
    use article_date_extractor::extract_date::extract_article_published_date;
    use chrono::NaiveDate;
    use reqwest;
    use std::io::Read;

    let link_1= "http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html";
    let link_2 = "https://www.nytimes.com/2017/03/15/style/meditation-studio-sound-baths-mndfl-new-york.html";
    let link_3 = "http://www.bbc.com/news/world-middle-east-39298218";
    let link_4 = "https://research.googleblog.com/2017/03/announcing-guetzli-new-open-source-jpeg.html";
    let link_5 = "http://theklog.co/type-of-water-to-wash-face-with/";

    let mut response = reqwest::get("http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html").unwrap();
    let mut body = String::new();
    response.read_to_string(&mut body).unwrap();

    assert_eq!(NaiveDate::from_ymd(2015,11,28), extract_article_published_date(&link_1, None).unwrap());
    assert_eq!(NaiveDate::from_ymd(2015,11,28), extract_article_published_date(&link_1, Some(body)).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017,03,15), extract_article_published_date(&link_2, None).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017,03,16), extract_article_published_date(&link_3, None).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017,03,16), extract_article_published_date(&link_4, None).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017,03,16), extract_article_published_date(&link_5, None).unwrap());

    assert!((extract_article_published_date("", None)).is_err());
}
