use chrono::{TimeZone, Utc};
use termxboard::news::{Headline, NewsFeed, NewsSelection, NewsView, UrlOpener, activate_selected};

fn headline(feed: NewsFeed, name: &str) -> Headline {
    Headline {
        source: feed,
        title: name.into(),
        url: format!("https://example.com/{name}"),
        published_at: Utc.with_ymd_and_hms(2026, 8, 6, 9, 0, 0).unwrap(),
    }
}

fn view() -> NewsView {
    NewsView::ready([
        vec![
            headline(NewsFeed::HackerNews, "hn-1"),
            headline(NewsFeed::HackerNews, "hn-2"),
        ],
        vec![headline(NewsFeed::SimonWillison, "simon-1")],
        vec![headline(NewsFeed::GithubBlog, "github-1")],
    ])
}

#[test]
fn arrows_move_selection_between_feeds_and_headlines() {
    let view = view();
    let mut selection = NewsSelection::default();

    selection.down(&view);
    assert_eq!(
        selection.selected_url(&view),
        Some("https://example.com/hn-2")
    );
    selection.right(&view);
    assert_eq!(
        selection.selected_url(&view),
        Some("https://example.com/simon-1")
    );
    selection.left(&view);
    assert_eq!(
        selection.selected_url(&view),
        Some("https://example.com/hn-1")
    );
    selection.up(&view);
    assert_eq!(
        selection.selected_url(&view),
        Some("https://example.com/hn-1")
    );
}

#[derive(Default)]
struct RecordingOpener(Vec<String>);

impl UrlOpener for RecordingOpener {
    fn open(&mut self, url: &str) -> Result<(), String> {
        self.0.push(url.into());
        Ok(())
    }
}

#[test]
fn activating_selection_opens_the_feed_provided_url() {
    let view = view();
    let selection = NewsSelection::default();
    let mut opener = RecordingOpener::default();

    assert!(activate_selected(&selection, &view, &mut opener).expect("open succeeds"));
    assert_eq!(opener.0, ["https://example.com/hn-1"]);
}
