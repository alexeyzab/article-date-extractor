extern crate article_date_extractor;
extern crate chrono;

#[test]
fn integration_test() {
    use article_date_extractor::extract_date::extract_article_published_date;
    use chrono::NaiveDate;

    let link_1 = "http://edition.cnn.\
                  com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html";
    let body_1 = include_str!("fixtures/cnn.html");

    let link_2 = "https://www.nytimes.\
                  com/2017/03/15/style/meditation-studio-sound-baths-mndfl-new-york.html";
    let body_2 = include_str!("fixtures/nytimes.html");

    let link_3 = "http://www.bbc.com/news/world-middle-east-39298218";
    let body_3 = include_str!("fixtures/bbc.html");

    let link_4 = "https://research.googleblog.com/2017/03/announcing-guetzli-new-open-source-jpeg.\
                  html";
    let body_4 = include_str!("fixtures/google_blog.html");

    let link_5 = "http://theklog.co/type-of-water-to-wash-face-with/";
    let body_5 = include_str!("fixtures/klog.html");

    assert_eq!(NaiveDate::from_ymd(2015, 11, 28),
               extract_article_published_date(&link_1, body_1).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017, 03, 15),
               extract_article_published_date(&link_2, body_2).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017, 03, 16),
               extract_article_published_date(&link_3, body_3).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017, 03, 16),
               extract_article_published_date(&link_4, body_4).unwrap());

    assert_eq!(NaiveDate::from_ymd(2017, 03, 16),
               extract_article_published_date(&link_5, body_5).unwrap());

    assert!((extract_article_published_date("", "")).is_err());
}
