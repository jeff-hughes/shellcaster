use anyhow::{anyhow, Result};
use chrono::Utc;
use opml::{Body, Head, Outline, OPML};

use crate::feeds::PodcastFeed;
use crate::types::*;

/// Import a list of podcast feeds from an OPML file. Supports
/// v1.0, v1.1, and v2.0 OPML files.
pub fn import(xml: String) -> Result<Vec<PodcastFeed>> {
    return match OPML::new(&xml) {
        Err(err) => Err(anyhow!(err)),
        Ok(opml) => {
            let mut feeds = Vec::new();
            for pod in opml.body.outlines.into_iter() {
                if pod.xml_url.is_some() {
                    // match against title attribute first -- if this is
                    // not set or empty, then match against the text
                    // attribute; this must be set, but can be empty
                    let temp_title = pod.title.filter(|t| !t.is_empty());
                    let title = match temp_title {
                        Some(t) => Some(t),
                        None => {
                            if pod.text.is_empty() {
                                None
                            } else {
                                Some(pod.text)
                            }
                        }
                    };
                    feeds.push(PodcastFeed::new(None, pod.xml_url.unwrap(), title));
                }
            }
            Ok(feeds)
        }
    };
}

/// Converts the current set of podcast feeds to the OPML format
pub fn export(podcasts: Vec<Podcast>) -> OPML {
    let date = Utc::now();
    let mut opml = OPML {
        head: Some(Head {
            title: Some("Shellcaster Podcast Feeds".to_string()),
            date_created: Some(date.to_rfc2822()),
            ..Head::default()
        }),
        ..Default::default()
    };

    let mut outlines = Vec::new();

    for pod in podcasts.iter() {
        // opml.add_feed(&pod.title, &pod.url);
        outlines.push(Outline {
            text: pod.title.clone(),
            r#type: Some("rss".to_string()),
            xml_url: Some(pod.url.clone()),
            title: Some(pod.title.clone()),
            ..Outline::default()
        });
    }

    opml.body = Body {
        outlines: outlines,
    };
    return opml;
}
