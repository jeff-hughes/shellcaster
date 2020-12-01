use std::io::Read;
use std::sync::mpsc;

use crate::sanitizer::parse_from_rfc2822_with_fallback;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::{Match, Regex};
use rss::{Channel, Item};

use crate::threadpool::Threadpool;
use crate::types::*;

lazy_static! {
    /// Regex for parsing an episode "duration", which could take the form
    /// of HH:MM:SS, MM:SS, or SS.
    static ref RE_DURATION: Regex = Regex::new(r"(\d+)(?::(\d+))?(?::(\d+))?").unwrap();
}

/// Enum for communicating back to the main thread after feed data has
/// been retrieved.
#[derive(Debug)]
pub enum FeedMsg {
    NewData(PodcastNoId),
    SyncData((i64, PodcastNoId)),
    Error(PodcastFeed),
}

/// Struct holding data about a podcast feed (subset of info about a
/// podcast) for the purpose of passing back and forth between threads.
#[derive(Debug, Clone)]
pub struct PodcastFeed {
    pub id: Option<i64>,
    pub url: String,
    pub title: Option<String>,
}

impl PodcastFeed {
    pub fn new(id: Option<i64>, url: String, title: Option<String>) -> Self {
        return Self {
            id: id,
            url: url,
            title: title,
        };
    }
}

/// Spawns a new thread to check a feed and retrieve podcast data.
pub fn check_feed(
    feed: PodcastFeed,
    max_retries: usize,
    threadpool: &Threadpool,
    tx_to_main: mpsc::Sender<Message>,
)
{
    threadpool.execute(move || match get_feed_data(feed.url.clone(), max_retries) {
        Ok(pod) => match feed.id {
            Some(id) => {
                tx_to_main
                    .send(Message::Feed(FeedMsg::SyncData((id, pod))))
                    .unwrap();
            }
            None => tx_to_main
                .send(Message::Feed(FeedMsg::NewData(pod)))
                .unwrap(),
        },
        Err(_err) => tx_to_main
            .send(Message::Feed(FeedMsg::Error(feed)))
            .unwrap(),
    });
}

/// Given a URL, this attempts to pull the data about a podcast and its
/// episodes from an RSS feed.
fn get_feed_data(
    url: String,
    mut max_retries: usize,
) -> Result<PodcastNoId, Box<dyn std::error::Error>>
{
    let request: Result<ureq::Response, Box<dyn std::error::Error>> = loop {
        let response = ureq::get(&url)
            .timeout_connect(5000)
            .timeout_read(15000)
            .call();
        if response.error() {
            max_retries -= 1;
            if max_retries == 0 {
                break Err(String::from("TODO: Better error handling here.").into());
            }
        } else {
            break Ok(response);
        }
    };

    return match request {
        Ok(resp) => {
            let mut reader = resp.into_reader();
            let mut resp_data = Vec::new();
            reader.read_to_end(&mut resp_data)?;

            let channel = Channel::read_from(&resp_data[..])?;
            Ok(parse_feed_data(channel, &url))
        }
        Err(err) => Err(err),
    };
}


/// Given a Channel with the RSS feed data, this parses the data about a
/// podcast and its episodes and returns a Podcast. There are existing
/// specifications for podcast RSS feeds that a feed should adhere to, but
/// this does try to make some attempt to account for the possibility that
/// a feed might not be valid according to the spec.
fn parse_feed_data(channel: Channel, url: &str) -> PodcastNoId {
    let title = channel.title().to_string();
    let url = url.to_string();
    let description = Some(channel.description().to_string());
    let last_checked = Utc::now();

    let mut author = None;
    let mut explicit = None;
    if let Some(itunes) = channel.itunes_ext() {
        author = match itunes.author() {
            None => None,
            Some(a) => Some(a.to_string()),
        };
        explicit = match itunes.explicit() {
            None => None,
            Some(s) => {
                let ss = s.to_lowercase();
                match &ss[..] {
                    "yes" | "explicit" | "true" => Some(true),
                    "no" | "clean" | "false" => Some(false),
                    _ => None,
                }
            }
        };
    }

    let mut episodes = Vec::new();
    let items = channel.into_items();
    if !items.is_empty() {
        for item in &items {
            episodes.push(parse_episode_data(item));
        }
    }

    return PodcastNoId {
        title: title,
        url: url,
        description: description,
        author: author,
        explicit: explicit,
        last_checked: last_checked,
        episodes: episodes,
    };
}

/// For an item (episode) in an RSS feed, this pulls data about the item
/// and converts it to an Episode. There are existing specifications for
/// podcast RSS feeds that a feed should adhere to, but this does try to
/// make some attempt to account for the possibility that a feed might
/// not be valid according to the spec.
fn parse_episode_data(item: &Item) -> EpisodeNoId {
    let title = match item.title() {
        Some(s) => s.to_string(),
        None => "".to_string(),
    };
    let url = match item.enclosure() {
        Some(enc) => enc.url().to_string(),
        None => "".to_string(),
    };
    let description = match item.description() {
        Some(dsc) => dsc.to_string(),
        None => "".to_string(),
    };
    let pubdate = match item.pub_date() {
        Some(pd) => match parse_from_rfc2822_with_fallback(pd) {
            Ok(date) => {
                // this is a bit ridiculous, but it seems like
                // you have to convert from a DateTime<FixedOffset>
                // to a NaiveDateTime, and then from there create
                // a DateTime<Utc>; see
                // https://github.com/chronotope/chrono/issues/169#issue-239433186
                Some(DateTime::from_utc(date.naive_utc(), Utc))
            }
            Err(_) => None,
        },
        None => None,
    };

    let mut duration = None;
    if let Some(itunes) = item.itunes_ext() {
        duration = match duration_to_int(itunes.duration()) {
            Some(dur) => Some(dur as i64),
            None => None,
        };
    }

    return EpisodeNoId {
        title: title,
        url: url,
        description: description,
        pubdate: pubdate,
        duration: duration,
    };
}

/// Given a string representing an episode duration, this attempts to
/// convert to an integer representing the duration in seconds. Covers
/// formats HH:MM:SS, MM:SS, and SS. If the duration cannot be converted
/// (covering numerous reasons), it will return None.
fn duration_to_int(duration: Option<&str>) -> Option<i32> {
    match duration {
        Some(dur) => {
            match RE_DURATION.captures(&dur) {
                Some(cap) => {
                    /*
                     * Provided that the regex succeeds, we should have
                     * 4 capture groups (with 0th being the full match).
                     * Depending on the string format, however, some of
                     * these may return None. We first loop through the
                     * capture groups and push Some results to a vector.
                     * After that, we convert from a vector of Results to
                     * a Result with a vector, using the collect() method.
                     * This will fail on the first error, so the duration
                     * is parsed only if all components of it were
                     * successfully converted to integers. Finally, we
                     * convert hours, minutes, and seconds into a total
                     * duration in seconds and return.
                     */

                    let mut times = Vec::new();
                    let mut first = true;
                    for c in cap.iter() {
                        // cap[0] is always full match
                        if first {
                            first = false;
                            continue;
                        }

                        if let Some(value) = c {
                            times.push(regex_to_int(value));
                        }
                    }

                    match times.len() {
                        // HH:MM:SS
                        3 => {
                            let result: Result<Vec<_>, _> = times.into_iter().collect();
                            match result {
                                Ok(v) => Some(v[0] * 60 * 60 + v[1] * 60 + v[2]),
                                Err(_) => None,
                            }
                        }
                        // MM:SS
                        2 => {
                            let result: Result<Vec<_>, _> = times.into_iter().collect();
                            match result {
                                Ok(v) => Some(v[0] * 60 + v[1]),
                                Err(_) => None,
                            }
                        }
                        // SS
                        1 => match times[0] {
                            Ok(i) => Some(i),
                            Err(_) => None,
                        },
                        _ => None,
                    }
                }
                None => None,
            }
        }
        None => None,
    }
}

/// Helper function converting a match from a regex capture group into an
/// integer.
fn regex_to_int(re_match: Match) -> Result<i32, std::num::ParseIntError> {
    let mstr = re_match.as_str();
    mstr.parse::<i32>()
}


// TESTS -----------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    fn open_file(path: &str) -> BufReader<File> {
        return BufReader::new(File::open(path).unwrap());
    }

    #[test]
    fn no_description() {
        let path = "./tests/test_no_description.xml";
        let channel = Channel::read_from(open_file(path)).unwrap();
        let data = parse_feed_data(channel, "dummy_url");
        assert_eq!(data.description, Some("".to_string()));
    }

    #[test]
    fn invalid_explicit() {
        let path = "./tests/test_inval_explicit.xml";
        let channel = Channel::read_from(open_file(path)).unwrap();
        let data = parse_feed_data(channel, "dummy_url");
        assert_eq!(data.explicit, None);
    }

    #[test]
    fn no_episodes() {
        let path = "./tests/test_no_episodes.xml";
        let channel = Channel::read_from(open_file(path)).unwrap();
        let data = parse_feed_data(channel, "dummy_url");
        assert_eq!(data.episodes.len(), 0);
    }

    #[test]
    fn nan_duration() {
        let duration = String::from("nan");
        assert_eq!(duration_to_int(Some(&duration)), None);
    }

    #[test]
    fn nonnumeric_duration() {
        let duration = String::from("some string");
        assert_eq!(duration_to_int(Some(&duration)), None);
    }

    #[test]
    fn duration_hhhmmss() {
        let duration = String::from("31:38:42");
        assert_eq!(duration_to_int(Some(&duration)), Some(113922));
    }

    #[test]
    fn duration_hhmmss() {
        let duration = String::from("01:38:42");
        assert_eq!(duration_to_int(Some(&duration)), Some(5922));
    }

    #[test]
    fn duration_hmmss() {
        let duration = String::from("1:38:42");
        assert_eq!(duration_to_int(Some(&duration)), Some(5922));
    }

    #[test]
    fn duration_mmmss() {
        let duration = String::from("68:42");
        assert_eq!(duration_to_int(Some(&duration)), Some(4122));
    }

    #[test]
    fn duration_mmss() {
        let duration = String::from("08:42");
        assert_eq!(duration_to_int(Some(&duration)), Some(522));
    }

    #[test]
    fn duration_mss() {
        let duration = String::from("8:42");
        assert_eq!(duration_to_int(Some(&duration)), Some(522));
    }

    #[test]
    fn duration_sss() {
        let duration = String::from("142");
        assert_eq!(duration_to_int(Some(&duration)), Some(142));
    }

    #[test]
    fn duration_ss() {
        let duration = String::from("08");
        assert_eq!(duration_to_int(Some(&duration)), Some(8));
    }

    #[test]
    fn duration_s() {
        let duration = String::from("8");
        assert_eq!(duration_to_int(Some(&duration)), Some(8));
    }
}
