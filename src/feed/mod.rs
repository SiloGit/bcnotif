extern crate reqwest;

pub mod statistics;
mod scrape;

use config::Config;
use std::io::Read;

error_chain! {
    links {
        Scrape(self::scrape::Error, self::scrape::ErrorKind);
    }

    foreign_links {
        Reqwest(reqwest::Error);
        Io(::std::io::Error);
    }

    errors {
        TopFeeds {
            display("failed to parse top feeds")
        }

        StateFeeds {
            display("failed to parse state feeds")
        }
    }
}

#[derive(Debug)]
pub struct Feed {
    pub id:        u32,
    pub name:      String,
    pub listeners: u32,
    pub state_id:  u32,
    pub county:    String,
    pub alert:     Option<String>,
}

impl Feed {
    pub fn download_and_scrape(config: &Config) -> Result<Vec<Feed>> {
        let client = reqwest::Client::new();

        let mut feeds = FeedSource::Top.download_and_scrape(&client)?;

        if let Some(state_id) = config.misc.state_feeds_id {
            let state_feeds = FeedSource::State(state_id)
                .download_and_scrape(&client)?;

            feeds.extend(state_feeds);
        }

        filter_whitelist_blacklist(config, &mut feeds);

        feeds.sort_by_key(|feed| feed.id);
        feeds.dedup();

        Ok(feeds)
    }

    // TODO: parse states from website (?)
    pub fn get_state_abbrev(&self) -> Option<&str> {
        macro_rules! create_matches {
            ($($id:expr => $abbrev:expr,)+) => {
                match self.state_id {
                    $($id => Some($abbrev),)+
                    _ => None,
                }
            };
        }

        create_matches!(
            1   => "AL", // Alabama
            2   => "AK", // Alaska
            4   => "AZ", // Arizona
            5   => "AR", // Arkansas
            6   => "CA", // California
            8   => "CO", // Colorado
            9   => "CT", // Connecticut
            10  => "DE", // Delaware
            11  => "DC", // District of Columbia
            12  => "FL", // Florida
            13  => "GA", // Georgia
            183 => "GU", // Guam
            15  => "HI", // Hawaii
            16  => "ID", // Idaho
            17  => "IL", // Illinois
            18  => "IN", // Indiana
            19  => "IA", // Iowa
            20  => "KS", // Kansas
            21  => "KY", // Kentucky
            22  => "LA", // Louisiana
            23  => "ME", // Maine
            24  => "MD", // Maryland
            25  => "MA", // Massachusetts
            26  => "MI", // Michigan
            27  => "MN", // Minnesota
            28  => "MS", // Mississippi
            29  => "MO", // Missouri
            30  => "MT", // Montana
            31  => "NE", // Nebraska
            32  => "NV", // Nevada
            33  => "NH", // New Hampshire
            34  => "NJ", // New Jersey
            35  => "NM", // New Mexico
            36  => "NY", // New York
            37  => "NC", // North Carolina
            38  => "ND", // North Dakota
            39  => "OH", // Ohio
            40  => "OK", // Oklahoma
            41  => "OR", // Oregon
            42  => "PA", // Pennsylvania
            57  => "PR", // Puerto Rico
            44  => "RI", // Rhode Island
            45  => "SC", // South Carolina
            46  => "SD", // South Dakota
            47  => "TN", // Tennessee
            48  => "TX", // Texas
            49  => "UT", // Utah
            50  => "VT", // Vermont
            181 => "VI", // Virgin Islands
            51  => "VA", // Virginia
            53  => "WA", // Washington
            54  => "WV", // West Virginia
            55  => "WI", // Wisconsin
            56  => "WY", // Wyoming
        )
    }
}

impl PartialEq for Feed {
    fn eq(&self, other: &Feed) -> bool {
        self.id == other.id
    }
}

fn filter_whitelist_blacklist(config: &Config, feeds: &mut Vec<Feed>) {
    if config.whitelist.len() > 0 {
        feeds.retain(|ref feed| {
            config.whitelist
                .iter()
                .any(|entry| entry.matches_feed(&feed))
        });
    }

    if config.blacklist.len() > 0 {
        feeds.retain(|ref feed| {
            config.blacklist
                .iter()
                .any(|entry| !entry.matches_feed(&feed))
        });
    }
}

enum FeedSource {
    Top,
    State(u32),
}

impl FeedSource {
    fn get_url(&self) -> String {
        match *self {
            FeedSource::Top => "http://broadcastify.com/listen/top".into(),
            FeedSource::State(id) => format!("http://broadcastify.com/listen/stid/{}", id),
        }
    }

    fn download_page(&self, client: &reqwest::Client) -> Result<String> {
        let mut resp = client.get(&self.get_url()).send()?;
        let mut body = String::new();

        resp.read_to_string(&mut body)?;
        
        Ok(body)
    }

    fn scrape(&self, body: &str) -> Result<Vec<Feed>> {
        match *self {
            FeedSource::Top => {
                let scraped = scrape::scrape_top(&body)
                    .chain_err(|| ErrorKind::TopFeeds)?;

                Ok(scraped)
            },
            FeedSource::State(id) => {
                let scraped = scrape::scrape_state(id, &body)
                    .chain_err(|| ErrorKind::StateFeeds)?;

                Ok(scraped)
            },
        }
    }

    fn download_and_scrape(&self, client: &reqwest::Client) -> Result<Vec<Feed>> {
        let body = self.download_page(client)?;
        self.scrape(&body)
    }
}