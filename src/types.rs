use std::fmt;
use std::convert;
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Podcast {
    pub id: Option<i32>,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub explicit: Option<bool>,
    pub last_checked: DateTime<Utc>,
    pub episodes: Vec<Episode>,
}

impl fmt::Display for Podcast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}

impl convert::AsRef<str> for Podcast {
    fn as_ref(&self) -> &str {
        return &self.title[..];
    }
}

#[derive(Debug)]
pub struct Episode {
    pub id: Option<i32>,
    pub title: String,
    pub url: String,
    pub description: String,
    pub pubdate: Option<DateTime<Utc>>,
    pub duration: Option<i32>,
    pub path: String,
    pub played: bool,
}

impl fmt::Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}

impl convert::AsRef<str> for Episode {
    fn as_ref(&self) -> &str {
        return &self.title[..];
    }
}