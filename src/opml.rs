use opml::{OPML, Head, Body, Outline};
use chrono::Utc;

use crate::types::*;

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