extern crate reqwest;

pub mod statistics;
mod scrape;

use config::Config;
use failure::{Error, ResultExt};
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct State {
    pub id: u32,
    pub abbrev: String,
}

impl State {
    pub fn new(id: u32, abbrev: String) -> State {
        State { id, abbrev }
    }
}

#[derive(Debug)]
pub struct Feed {
    pub id: u32,
    pub name: String,
    pub listeners: u32,
    pub state: State,
    pub county: String,
    pub alert: Option<String>,
}

impl Feed {
    pub fn download_and_scrape(config: &Config) -> Result<Vec<Feed>, Error> {
        let client = reqwest::Client::new();

        let mut feeds = FeedSource::Top.download_and_scrape(&client)?;

        if let Some(state_id) = config.misc.state_feeds_id {
            let state = State::new(state_id, "CS".into()); // CS = Config Specified
            let state_feeds = FeedSource::State(state).download_and_scrape(&client)?;

            feeds.extend(state_feeds);
        }

        filter_whitelist_blacklist(config, &mut feeds);

        feeds.sort_by_key(|feed| feed.id);
        feeds.dedup();

        Ok(feeds)
    }
}

impl PartialEq for Feed {
    fn eq(&self, other: &Feed) -> bool {
        self.id == other.id
    }
}

fn filter_whitelist_blacklist(config: &Config, feeds: &mut Vec<Feed>) {
    if !config.whitelist.is_empty() {
        feeds.retain(|feed| {
            config
                .whitelist
                .iter()
                .any(|entry| entry.matches_feed(feed))
        });
    }

    if !config.blacklist.is_empty() {
        feeds.retain(|feed| {
            config
                .blacklist
                .iter()
                .any(|entry| !entry.matches_feed(feed))
        });
    }
}

// TODO: move to scrape module (?)
#[derive(Fail, Debug)]
pub enum FeedSourceError {
    #[fail(display = "failed to parse top feeds")] FailedToParseTopFeeds,
    #[fail(display = "failed to parse state feeds")] FailedToParseStateFeeds,
}

enum FeedSource {
    Top,
    State(State),
}

impl FeedSource {
    fn get_url(&self) -> Cow<str> {
        match *self {
            FeedSource::Top => "http://www.broadcastify.com/listen/top".into(),
            FeedSource::State(ref state) => {
                format!("http://www.broadcastify.com/listen/stid/{}", state.id).into()
            }
        }
    }

    fn download_page(&self, client: &reqwest::Client) -> Result<String, Error> {
        let body = client.get(self.get_url().as_ref()).send()?.text()?;
        Ok(body)
    }

    fn scrape(&self, body: &str) -> Result<Vec<Feed>, Error> {
        match *self {
            FeedSource::Top => {
                let scraped =
                    scrape::scrape_top(body).context(FeedSourceError::FailedToParseTopFeeds)?;

                Ok(scraped)
            }
            FeedSource::State(ref state) => {
                let scraped = scrape::scrape_state(state, body)
                    .context(FeedSourceError::FailedToParseStateFeeds)?;

                Ok(scraped)
            }
        }
    }

    fn download_and_scrape(&self, client: &reqwest::Client) -> Result<Vec<Feed>, Error> {
        let body = self.download_page(client)?;
        self.scrape(&body)
    }
}
