use std::{
    collections::HashSet,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};

pub const MAX_HEADLINES_PER_FEED: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NewsFeed {
    HackerNews,
    SimonWillison,
    GithubBlog,
}

impl NewsFeed {
    pub const ALL: [Self; 3] = [Self::HackerNews, Self::SimonWillison, Self::GithubBlog];

    pub fn label(self) -> &'static str {
        match self {
            Self::HackerNews => "HACKER NEWS",
            Self::SimonWillison => "SIMON WILLISON // LLMs",
            Self::GithubBlog => "GITHUB BLOG",
        }
    }

    pub fn url(self) -> &'static str {
        match self {
            Self::HackerNews => "https://hnrss.org/frontpage",
            Self::SimonWillison => "https://simonwillison.net/tags/llms.atom",
            Self::GithubBlog => "https://github.com/blog.atom",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::HackerNews => 0,
            Self::SimonWillison => 1,
            Self::GithubBlog => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headline {
    pub source: NewsFeed,
    pub title: String,
    pub url: String,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedView {
    Loading {
        last_good: Option<Vec<Headline>>,
    },
    Ready(Vec<Headline>),
    Error {
        message: String,
        last_good: Option<Vec<Headline>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsView {
    feeds: [FeedView; 3],
}

impl NewsView {
    pub fn loading() -> Self {
        Self {
            feeds: std::array::from_fn(|_| FeedView::Loading { last_good: None }),
        }
    }

    pub fn feed(&self, feed: NewsFeed) -> &FeedView {
        &self.feeds[feed.index()]
    }

    pub fn ready(feeds: [Vec<Headline>; 3]) -> Self {
        Self {
            feeds: feeds.map(FeedView::Ready),
        }
    }

    pub fn from_views(feeds: [FeedView; 3]) -> Self {
        Self { feeds }
    }

    pub fn headlines(&self, feed: NewsFeed) -> &[Headline] {
        match self.feed(feed) {
            FeedView::Loading {
                last_good: Some(items),
            }
            | FeedView::Ready(items)
            | FeedView::Error {
                last_good: Some(items),
                ..
            } => items,
            FeedView::Loading { last_good: None }
            | FeedView::Error {
                last_good: None, ..
            } => &[],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NewsSelection {
    feed_index: usize,
    headline_index: usize,
}

impl NewsSelection {
    pub fn left(&mut self, view: &NewsView) {
        self.feed_index = self.feed_index.saturating_sub(1);
        self.clamp(view);
    }

    pub fn right(&mut self, view: &NewsView) {
        self.feed_index = (self.feed_index + 1).min(NewsFeed::ALL.len() - 1);
        self.clamp(view);
    }

    pub fn up(&mut self, view: &NewsView) {
        self.headline_index = self.headline_index.saturating_sub(1);
        self.clamp(view);
    }

    pub fn down(&mut self, view: &NewsView) {
        self.headline_index = self.headline_index.saturating_add(1);
        self.clamp(view);
    }

    pub fn is_selected(self, feed: NewsFeed, headline_index: usize) -> bool {
        self.feed_index == feed.index() && self.headline_index == headline_index
    }

    pub fn selected_url(self, view: &NewsView) -> Option<&str> {
        let feed = NewsFeed::ALL[self.feed_index];
        view.headlines(feed)
            .get(self.headline_index)
            .map(|headline| headline.url.as_str())
    }

    fn clamp(&mut self, view: &NewsView) {
        let feed = NewsFeed::ALL[self.feed_index];
        self.headline_index = self
            .headline_index
            .min(view.headlines(feed).len().saturating_sub(1));
    }
}

pub trait UrlOpener {
    fn open(&mut self, url: &str) -> Result<(), String>;
}

pub struct MacUrlOpener;

impl UrlOpener for MacUrlOpener {
    fn open(&mut self, url: &str) -> Result<(), String> {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("could not open headline: {error}"))
    }
}

pub fn activate_selected(
    selection: &NewsSelection,
    view: &NewsView,
    opener: &mut impl UrlOpener,
) -> Result<bool, String> {
    let Some(url) = selection.selected_url(view) else {
        return Ok(false);
    };
    opener.open(url)?;
    Ok(true)
}

pub trait NewsClient: Send + 'static {
    fn fetch(&mut self, feed: NewsFeed) -> Result<Vec<Headline>, String>;
}

pub struct FeedNewsClient {
    agent: ureq::Agent,
}

impl FeedNewsClient {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(8)))
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Default for FeedNewsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NewsClient for FeedNewsClient {
    fn fetch(&mut self, feed: NewsFeed) -> Result<Vec<Headline>, String> {
        let mut response = self
            .agent
            .get(feed.url())
            .header("User-Agent", "TermXBoard/0.1")
            .call()
            .map_err(|error| format!("{} request failed: {error}", feed.label()))?;
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|error| format!("{} response failed: {error}", feed.label()))?;
        parse_feed(feed, &body, Utc::now())
    }
}

enum WorkerCommand {
    Refresh,
    Stop,
}
type FeedResults = Vec<(NewsFeed, Result<Vec<Headline>, String>)>;

pub struct NewsMonitor {
    command_sender: Sender<WorkerCommand>,
    result_receiver: Receiver<FeedResults>,
    view: NewsView,
    interval: Duration,
    next_refresh: Instant,
    in_flight: bool,
    last_good: [Option<Vec<Headline>>; 3],
}

impl NewsMonitor {
    pub fn new<C: NewsClient>(client: C, interval: Duration, now: Instant) -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut client = client;
            while let Ok(command) = command_receiver.recv() {
                match command {
                    WorkerCommand::Refresh => {
                        let results = NewsFeed::ALL
                            .into_iter()
                            .map(|feed| (feed, client.fetch(feed)))
                            .collect();
                        if result_sender.send(results).is_err() {
                            break;
                        }
                    }
                    WorkerCommand::Stop => break,
                }
            }
        });
        Self {
            command_sender,
            result_receiver,
            view: NewsView::loading(),
            interval,
            next_refresh: now,
            in_flight: false,
            last_good: std::array::from_fn(|_| None),
        }
    }

    pub fn view(&self) -> &NewsView {
        &self.view
    }

    pub fn refresh_now(&mut self, now: Instant) {
        self.next_refresh = now;
    }

    pub fn tick(&mut self, now: Instant) {
        while let Ok(results) = self.result_receiver.try_recv() {
            self.in_flight = false;
            let mut seen_urls = HashSet::new();
            for (feed, result) in results {
                let index = feed.index();
                self.view.feeds[index] = match result {
                    Ok(mut headlines) => {
                        headlines.retain(|headline| seen_urls.insert(headline.url.clone()));
                        self.last_good[index] = Some(headlines.clone());
                        FeedView::Ready(headlines)
                    }
                    Err(message) => FeedView::Error {
                        message,
                        last_good: self.last_good[index].clone(),
                    },
                };
            }
        }
        if !self.in_flight && now >= self.next_refresh {
            if self.command_sender.send(WorkerCommand::Refresh).is_ok() {
                self.in_flight = true;
                self.next_refresh = now + self.interval;
                self.view.feeds = std::array::from_fn(|index| FeedView::Loading {
                    last_good: self.last_good[index].clone(),
                });
            } else {
                self.view.feeds = std::array::from_fn(|index| FeedView::Error {
                    message: "news worker stopped".into(),
                    last_good: self.last_good[index].clone(),
                });
            }
        }
    }
}

impl Drop for NewsMonitor {
    fn drop(&mut self) {
        let _ = self.command_sender.send(WorkerCommand::Stop);
    }
}

pub fn parse_feed(
    source: NewsFeed,
    xml: &str,
    _now: DateTime<Utc>,
) -> Result<Vec<Headline>, String> {
    let feed = feed_rs::parser::parse(xml.as_bytes())
        .map_err(|error| format!("invalid {} feed: {error}", source.label()))?;
    let mut seen_urls = HashSet::new();
    let mut headlines = Vec::new();
    for entry in feed.entries {
        let Some(title) = entry
            .title
            .map(|title| title.content.trim().to_string())
            .filter(|title| !title.is_empty())
        else {
            continue;
        };
        let Some(url) = entry
            .links
            .first()
            .map(|link| link.href.trim().to_string())
            .filter(|url| !url.is_empty())
        else {
            continue;
        };
        let Some(published_at) = entry.published.or(entry.updated) else {
            continue;
        };
        if seen_urls.insert(url.clone()) {
            headlines.push(Headline {
                source,
                title,
                url,
                published_at,
            });
        }
        if headlines.len() == MAX_HEADLINES_PER_FEED {
            break;
        }
    }
    if headlines.is_empty() {
        return Err(format!(
            "{} feed returned no usable headlines",
            source.label()
        ));
    }
    Ok(headlines)
}
