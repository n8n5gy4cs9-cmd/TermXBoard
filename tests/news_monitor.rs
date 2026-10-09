use std::{
    collections::VecDeque,
    thread,
    time::{Duration, Instant},
};

use chrono::{TimeZone, Utc};
use termxboard::news::{FeedView, Headline, NewsClient, NewsFeed, NewsMonitor};

struct ScriptedClient {
    results: VecDeque<Result<Vec<Headline>, String>>,
    delay: Duration,
}

impl NewsClient for ScriptedClient {
    fn fetch(&mut self, _feed: NewsFeed) -> Result<Vec<Headline>, String> {
        thread::sleep(self.delay);
        self.results.pop_front().expect("scripted feed result")
    }
}

fn headline(feed: NewsFeed, suffix: &str) -> Headline {
    Headline {
        source: feed,
        title: format!("Story {suffix}"),
        url: format!("https://example.com/{suffix}"),
        published_at: Utc.with_ymd_and_hms(2026, 8, 6, 9, 0, 0).unwrap(),
    }
}

fn cycle(suffix: &str) -> [Result<Vec<Headline>, String>; 3] {
    [
        Ok(vec![headline(
            NewsFeed::HackerNews,
            &format!("hn-{suffix}"),
        )]),
        Ok(vec![headline(
            NewsFeed::SimonWillison,
            &format!("simon-{suffix}"),
        )]),
        Ok(vec![headline(
            NewsFeed::GithubBlog,
            &format!("github-{suffix}"),
        )]),
    ]
}

fn wait_for_title(monitor: &mut NewsMonitor, now: Instant, expected: &str) {
    for _ in 0..100 {
        monitor.tick(now);
        if matches!(monitor.view().feed(NewsFeed::HackerNews), FeedView::Ready(items) if items[0].title == expected)
        {
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("news did not become ready");
}

#[test]
fn news_refresh_is_nonblocking_scheduled_and_manual() {
    let now = Instant::now();
    let results = cycle("one")
        .into_iter()
        .chain(cycle("two"))
        .chain(cycle("three"))
        .collect();
    let client = ScriptedClient {
        results,
        delay: Duration::from_millis(20),
    };
    let mut monitor = NewsMonitor::new(client, Duration::from_secs(900), now);

    let started = Instant::now();
    monitor.tick(now);
    assert!(started.elapsed() < Duration::from_millis(20));
    assert!(matches!(
        monitor.view().feed(NewsFeed::HackerNews),
        FeedView::Loading { last_good: None }
    ));
    wait_for_title(&mut monitor, now, "Story hn-one");

    monitor.tick(now + Duration::from_secs(899));
    assert!(
        matches!(monitor.view().feed(NewsFeed::HackerNews), FeedView::Ready(items) if items[0].title == "Story hn-one")
    );
    wait_for_title(&mut monitor, now + Duration::from_secs(900), "Story hn-two");

    monitor.refresh_now(now + Duration::from_secs(901));
    monitor.tick(now + Duration::from_secs(901));
    assert!(matches!(
        monitor.view().feed(NewsFeed::HackerNews),
        FeedView::Loading { last_good: Some(_) }
    ));
    wait_for_title(
        &mut monitor,
        now + Duration::from_secs(901),
        "Story hn-three",
    );
}

#[test]
fn failed_feed_retains_its_last_successful_session_headlines() {
    let now = Instant::now();
    let results = cycle("cached")
        .into_iter()
        .chain([
            Err("HN offline".into()),
            Ok(vec![headline(NewsFeed::SimonWillison, "simon-new")]),
            Ok(vec![headline(NewsFeed::GithubBlog, "github-new")]),
        ])
        .collect();
    let client = ScriptedClient {
        results,
        delay: Duration::ZERO,
    };
    let mut monitor = NewsMonitor::new(client, Duration::from_secs(900), now);
    wait_for_title(&mut monitor, now, "Story hn-cached");

    monitor.refresh_now(now + Duration::from_secs(1));
    for _ in 0..100 {
        monitor.tick(now + Duration::from_secs(1));
        if let FeedView::Error {
            message,
            last_good: Some(items),
        } = monitor.view().feed(NewsFeed::HackerNews)
        {
            assert_eq!(message, "HN offline");
            assert_eq!(items[0].title, "Story hn-cached");
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("feed failure was not visible");
}

#[test]
fn refresh_deduplicates_urls_across_news_feeds() {
    let now = Instant::now();
    let duplicate_url = "https://example.com/shared";
    let mut hn = headline(NewsFeed::HackerNews, "hn");
    hn.url = duplicate_url.into();
    let mut simon = headline(NewsFeed::SimonWillison, "simon");
    simon.url = duplicate_url.into();
    let results = [
        Ok(vec![hn]),
        Ok(vec![simon]),
        Ok(vec![headline(NewsFeed::GithubBlog, "github")]),
    ]
    .into();
    let client = ScriptedClient {
        results,
        delay: Duration::ZERO,
    };
    let mut monitor = NewsMonitor::new(client, Duration::from_secs(900), now);
    wait_for_title(&mut monitor, now, "Story hn");

    assert!(monitor.view().headlines(NewsFeed::SimonWillison).is_empty());
}
