use feed::{Feed, State};
use select::document::Document;
use select::node::Node;
use select::predicate::{Class, Name, Predicate};

type ElementName = &'static str;

#[derive(Fail, Debug)]
pub enum ScrapeError {
    #[fail(display = "unable to find element that contains {} information", _0)]
    NoElement(ElementName),

    #[fail(display = "unable to parse {} information", _1)]
    FailedIntParse(#[cause] ::std::num::ParseIntError, ElementName),

    #[fail(display = "no feeds found")]
    NoneFound,
}

pub fn scrape_top<'a>(body: &str) -> Result<Vec<Feed<'a>>, ScrapeError> {
    let doc = Document::from(body);

    let feed_data = doc.find(Class("btable").descendant(Name("tr"))).skip(1);
    let mut feeds = Vec::new();

    for row in feed_data {
        let (id, name) = parse_id_and_name(&row, "w100")?;

        // The top 50 feed list allows multiple states and/or counties to appear,
        // so we can't assume their location
        let location_info = row.find(Name("td"))
            .nth(1)
            .ok_or_else(|| ScrapeError::NoElement("location"))?;

        let mut hyperlinks = location_info
            .find(Name("a"))
            .filter_map(|link| link.attr("href").map(|url| (url, link.text())));

        let (state_link, state_abbrev) = hyperlinks
            .next()
            .ok_or_else(|| ScrapeError::NoElement("state data"))?;

        let state_id = parse_link_id(state_link)
            .ok_or_else(|| ScrapeError::NoElement("state id"))?
            .parse::<u32>()
            .map_err(|e| ScrapeError::FailedIntParse(e, "state id"))?;

        let county = match hyperlinks.next() {
            Some((link, ref text)) if link.starts_with("/listen/ctid") => text.clone(),
            _ => "Numerous".to_string(),
        };

        feeds.push(Feed {
            id,
            state: State::new(state_id, state_abbrev),
            county,
            name,
            listeners: parse_listeners(&row)?,
            alert: row.find(Class("messageBox"))
                .next()
                .map(|alert| alert.text()),
        });
    }

    if feeds.is_empty() {
        return Err(ScrapeError::NoneFound);
    }

    Ok(feeds)
}

pub fn scrape_state<'a>(state: &State<'a>, body: &str) -> Result<Vec<Feed<'a>>, ScrapeError> {
    let doc = Document::from(body);

    // TODO: add support for areawide feeds
    let table = {
        // State feed pages may contain a section for areawide feeds that appears
        // before the main feed data. Since the parsing logic for that hasn't been
        // implemented yet, we simply skip over that table
        let tables = doc.find(Class("btable")).take(2).collect::<Vec<_>>();

        if tables.is_empty() {
            return Err(ScrapeError::NoElement("feed data"));
        } else if tables.len() >= 2 {
            tables[1]
        } else {
            tables[0]
        }
    };

    let feed_data = table.find(Class("btable").descendant(Name("tr")));

    let mut feeds = Vec::new();

    for feed in feed_data.skip(1) {
        let (id, name) = parse_id_and_name(&feed, "w1p")?;

        let county = feed.find(Name("a"))
            .next()
            .map(|node| node.text())
            .unwrap_or_else(|| "Numerous".to_string());

        let alert = feed.find(Name("font").and(Class("fontRed")))
            .next()
            .map(|alert| alert.text());

        feeds.push(Feed {
            id,
            state: state.clone(),
            county,
            name,
            listeners: parse_listeners(&feed)?,
            alert,
        });
    }

    if feeds.is_empty() {
        return Err(ScrapeError::NoneFound);
    }

    Ok(feeds)
}

fn parse_id_and_name(node: &Node, class_name: &str) -> Result<(u32, String), ScrapeError> {
    let base = node.find(Class(class_name).descendant(Name("a")))
        .next()
        .ok_or_else(|| ScrapeError::NoElement("id and name"))?;

    let id = base.attr("href")
        .and_then(parse_link_id)
        .ok_or_else(|| ScrapeError::NoElement("feed id"))?
        .parse::<u32>()
        .map_err(|e| ScrapeError::FailedIntParse(e, "state id"))?;

    Ok((id, base.text()))
}

fn parse_listeners(node: &Node) -> Result<u32, ScrapeError> {
    let text = node.find(Class("c").and(Class("m")))
        .next()
        .map(|node| node.text())
        .ok_or_else(|| ScrapeError::NoElement("feed listeners"))?;

    let result = text.trim_right()
        .parse::<u32>()
        .map_err(|e| ScrapeError::FailedIntParse(e, "feed listeners"))?;

    Ok(result)
}

fn parse_link_id(url: &str) -> Option<String> {
    let pos = url.rfind('/')?;

    if pos + 1 >= url.len() {
        None
    } else {
        Some(url[pos + 1..].to_string())
    }
}
