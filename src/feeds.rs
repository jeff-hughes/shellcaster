use rss::Channel;

use crate::types::{Podcast};

pub fn get_feed_data(url: String) -> Result<Podcast, Box<dyn std::error::Error>> {
    let channel = Channel::from_url(&url)?;

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
            },
        };
    }

    Ok(Podcast {
        id: None,
        title: channel.title().to_string(),
        url: url,
        description: Some(channel.description().to_string()),
        author: author,
        explicit: explicit,
    })
}