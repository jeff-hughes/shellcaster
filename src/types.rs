use std::fmt;
use std::convert;
use std::rc::Rc;
use core::cell::RefCell;
use chrono::{DateTime, Utc};

/// Struct holding data about an individual podcast feed. This includes a
/// (possibly empty) vector of episodes.
#[derive(Debug)]
pub struct Podcast {
    pub id: Option<i32>,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub explicit: Option<bool>,
    pub last_checked: DateTime<Utc>,
    pub episodes: MutableVec<Episode>,
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

/// Struct holding data about an individual podcast episode. Most of this
/// is metadata, but if the episode has been downloaded to the local
/// machine, the filepath will be included here as well. `played` indicates
/// whether the podcast has been marked as played or unplayed.
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


pub type MutableVec<T> = Rc<RefCell<Vec<T>>>;