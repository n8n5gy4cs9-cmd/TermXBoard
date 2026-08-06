use chrono::{TimeZone, Utc};
use termxboard::news::{NewsFeed, parse_feed};

#[test]
fn rss_headlines_keep_feed_titles_urls_and_publication_times() {
    let xml = r#"<?xml version="1.0"?>
      <rss version="2.0"><channel><title>HN</title>
        <item><title>Rust reaches orbit</title><link>https://example.com/rust</link><pubDate>Wed, 06 Aug 2026 09:00:00 GMT</pubDate></item>
      </channel></rss>"#;
    let now = Utc.with_ymd_and_hms(2026, 8, 6, 12, 0, 0).unwrap();

    let headlines = parse_feed(NewsFeed::HackerNews, xml, now).expect("valid RSS");

    assert_eq!(headlines.len(), 1);
    assert_eq!(headlines[0].source, NewsFeed::HackerNews);
    assert_eq!(headlines[0].title, "Rust reaches orbit");
    assert_eq!(headlines[0].url, "https://example.com/rust");
    assert_eq!(
        headlines[0].published_at,
        Utc.with_ymd_and_hms(2026, 8, 6, 9, 0, 0).unwrap()
    );
}

#[test]
fn atom_headlines_are_deduplicated_and_limited_to_five() {
    let mut entries = String::new();
    for index in 0..6 {
        entries.push_str(&format!(r#"<entry><title>Story {index}</title><link href="https://example.com/{index}"/><published>2026-08-06T0{index}:00:00Z</published></entry>"#));
    }
    entries.push_str(r#"<entry><title>Duplicate URL</title><link href="https://example.com/0"/><published>2026-08-06T10:00:00Z</published></entry>"#);
    let xml = format!(
        r#"<?xml version="1.0"?><feed xmlns="http://www.w3.org/2005/Atom"><title>Feed</title>{entries}</feed>"#
    );
    let now = Utc.with_ymd_and_hms(2026, 8, 6, 12, 0, 0).unwrap();

    let headlines = parse_feed(NewsFeed::SimonWillison, &xml, now).expect("valid Atom");

    assert_eq!(headlines.len(), 5);
    assert_eq!(
        headlines
            .iter()
            .filter(|item| item.url == "https://example.com/0")
            .count(),
        1
    );
}

#[test]
fn malformed_feeds_and_entries_without_required_feed_data_are_rejected() {
    let now = Utc.with_ymd_and_hms(2026, 8, 6, 12, 0, 0).unwrap();
    assert!(parse_feed(NewsFeed::GithubBlog, "not XML", now).is_err());

    let missing_time = r#"<rss version="2.0"><channel><title>x</title><item><title>No time</title><link>https://example.com</link></item></channel></rss>"#;
    assert!(parse_feed(NewsFeed::GithubBlog, missing_time, now).is_err());
}
