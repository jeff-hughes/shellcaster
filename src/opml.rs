use chrono::Utc;
use opml::{Body, Head, Outline, OPML};

use crate::feeds::PodcastFeed;
use crate::types::*;

/// Import a list of podcast feeds from an OPML file. Supports
/// v1.0, v1.1, and v2.0 OPML files.
pub fn import(xml: String) -> Result<Vec<PodcastFeed>, String> {
    return match OPML::new(&xml) {
        Err(err) => Err(err),
        Ok(opml) => {
            let mut feeds = Vec::new();
            for pod in opml.body.outlines.iter() {
                if let Some(url) = pod.xml_url.clone() {
                    // match against title attribute first -- if this is
                    // not set or empty, then match against the text
                    // attribute; this must be set, but can be empty
                    let temp_title = match &pod.title {
                        Some(t) => {
                            if t.is_empty() {
                                None
                            } else {
                                Some(t.clone())
                            }
                        }
                        None => None,
                    };
                    let title = match temp_title {
                        Some(t) => Some(t),
                        None => {
                            if pod.text.is_empty() {
                                None
                            } else {
                                Some(pod.text.clone())
                            }
                        }
                    };
                    feeds.push(PodcastFeed::new(None, url, title));
                }
            }
            Ok(feeds)
        }
    };
}

/// Converts the current set of podcast feeds to the OPML format
pub fn export(podcasts: Vec<Podcast>) -> OPML {
    let date = Utc::now();
    let mut opml = OPML::default();
    opml.head = Some(Head {
        title: Some("Shellcaster Podcast Feeds".to_string()),
        date_created: Some(date.to_rfc2822()),
        ..Head::default()
    });

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
