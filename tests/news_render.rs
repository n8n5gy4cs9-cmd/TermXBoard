use chrono::{Duration, Utc};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use termxboard::{
    AppState,
    news::{FeedView, Headline, NewsFeed, NewsSelection, NewsView},
    telemetry::TelemetryView,
    ui,
    weather::WeatherView,
};

fn headline(feed: NewsFeed, title: &str, hours_ago: i64) -> Headline {
    Headline {
        source: feed,
        title: title.into(),
        url: format!("https://example.com/{title}"),
        published_at: Utc::now() - Duration::hours(hours_ago),
    }
}

fn rendered(news: &NewsView) -> (String, usize) {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| {
            ui::render_with_news(
                frame,
                &AppState::default(),
                &TelemetryView::loading(),
                &WeatherView::loading(),
                news,
                &NewsSelection::default(),
            )
        })
        .expect("render");
    let buffer = terminal.backend().buffer();
    let output = buffer.content().iter().map(|cell| cell.symbol()).collect();
    let red_leds = buffer
        .content()
        .iter()
        .filter(|cell| cell.symbol() == "●" && cell.fg == Color::Red)
        .count();
    (output, red_leds)
}

#[test]
fn dashboard_renders_all_news_sources_headlines_and_relative_times() {
    let news = NewsView::ready([
        vec![headline(NewsFeed::HackerNews, "Rust reaches orbit", 3)],
        vec![headline(NewsFeed::SimonWillison, "New LLM notes", 2)],
        vec![headline(NewsFeed::GithubBlog, "Actions update", 1)],
    ]);

    let (output, _) = rendered(&news);

    for expected in [
        "HACKER NEWS",
        "SIMON WILLISON",
        "GITHUB BLOG",
        "Rust reaches orbit",
        "New LLM notes",
        "Actions update",
        "3h",
    ] {
        assert!(output.contains(expected), "missing {expected:?}");
    }
}

#[test]
fn dashboard_marks_failed_feed_and_keeps_cached_headline_visible() {
    let cached = headline(NewsFeed::HackerNews, "Cached story", 1);
    let news = NewsView::from_views([
        FeedView::Error {
            message: "feed offline".into(),
            last_good: Some(vec![cached]),
        },
        FeedView::Loading { last_good: None },
        FeedView::Ready(vec![headline(NewsFeed::GithubBlog, "GitHub story", 1)]),
    ]);

    let (output, red_leds) = rendered(&news);

    assert!(output.contains("Cached story"));
    assert!(output.contains("STALE"));
    assert!(red_leds >= 1);
}

#[test]
fn dashboard_designs_loading_and_empty_news_states() {
    let loading = NewsView::loading();
    let (loading_output, _) = rendered(&loading);
    assert!(loading_output.contains("LOADING"));
    assert!(loading_output.contains('▰'));

    let empty = NewsView::ready([vec![], vec![], vec![]]);
    let (empty_output, _) = rendered(&empty);
    assert_eq!(empty_output.matches("NO HEADLINES").count(), 3);
}
