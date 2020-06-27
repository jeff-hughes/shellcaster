use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::ops::{Bound, RangeBounds};
use chrono::{DateTime, Utc};

use crate::ui::UiMsg;
use crate::feeds::FeedMsg;
use crate::downloads::DownloadMsg;

/// Defines interface used for both podcasts and episodes, to be
/// used and displayed in menus.
pub trait Menuable {
    fn get_title(&self, length: usize) -> String;
    fn is_played(&self) -> bool;
}

/// Struct holding data about an individual podcast feed. This includes a
/// (possibly empty) vector of episodes.
#[derive(Debug, Clone)]
pub struct Podcast {
    pub id: Option<i32>,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub explicit: Option<bool>,
    pub last_checked: DateTime<Utc>,
    pub episodes: LockVec<Episode>,
    pub any_unplayed: bool,
}

impl Menuable for Podcast {
    fn get_title(&self, length: usize) -> String {
        return self.title[..].substring(0, length).to_string();
    }

    fn is_played(&self) -> bool {
        return !self.any_unplayed;
    }
}

/// Struct holding data about an individual podcast episode. Most of this
/// is metadata, but if the episode has been downloaded to the local
/// machine, the filepath will be included here as well. `played` indicates
/// whether the podcast has been marked as played or unplayed.
#[derive(Debug, Clone)]
pub struct Episode {
    pub id: Option<i32>,
    pub pod_id: Option<i32>,
    pub title: String,
    pub url: String,
    pub description: String,
    pub pubdate: Option<DateTime<Utc>>,
    pub duration: Option<i32>,
    pub path: Option<PathBuf>,
    pub played: bool,
}

impl Menuable for Episode {
    fn get_title(&self, length: usize) -> String {
        return match self.path {
            Some(_) => format!("[D] {}", self.title[..].substring(0, length-4)),
            None => self.title[..].substring(0, length).to_string(),
        };
    }

    fn is_played(&self) -> bool {
        return self.played;
    }
}


/// Struct used to hold a vector of data inside a reference-counted
/// mutex, to allow for multiple owners of mutable data.
/// Primarily, the LockVec is used to provide methods that abstract
/// away some of the logic necessary for borrowing and locking the
/// Arc<Mutex<_>>.
#[derive(Debug)]
pub struct LockVec<T>
    where T: Clone {
    data: Arc<Mutex<Vec<T>>>,
}

impl<T: Clone> LockVec<T> {
    /// Create a new LockVec.
    pub fn new(data: Vec<T>) -> LockVec<T> {
        return LockVec {
            data: Arc::new(Mutex::new(data)),
        }
    }

    /// Lock the LockVec for reading/writing.
    pub fn borrow(&self) -> MutexGuard<Vec<T>> {
        return self.data.lock().unwrap();
    }

    /// Given an index in the vector, this takes a new T and replaces
    /// the old T at that position in the vector.
    pub fn replace(&self, index: i32, t: T) -> Result<(), &'static str> {
        let mut borrowed = self.borrow();
        if index > 0 && (index as usize) < borrowed.len() {
            borrowed[index as usize] = t;
            return Ok(());
        } else {
            return Err("Invalid index");
        }
    }

}

impl<T: Clone> Clone for LockVec<T> {
    fn clone(&self) -> Self {
        return LockVec {
            data: Arc::clone(&self.data),
        }
    }
}

impl LockVec<Podcast> {
    /// This clones the podcast at the given index.
    pub fn clone_podcast(&self, index: i32) -> Option<Podcast> {
        let pod_list = self.borrow();
        return match pod_list.get(index as usize) {
            Some(pod) => Some(pod.clone()),
            None => None,
        };
    }

    /// This clones the episode at the given index (`ep_index`), from
    /// the podcast at the given index (`pod_index`). Note that if you
    /// are already borrowing the episode list for a podcast, you can
    /// also use `clone_episode()` directly on that list.
    pub fn clone_episode(&self, pod_index: i32, ep_index: i32) -> Option<Episode> {
        let pod_list = self.borrow();
        if let Some(pod) = pod_list.get(pod_index as usize) {
            return pod.episodes.clone_episode(ep_index);
        }
        return None;
    }

    /// Given a podcast ID (from the database), this provides the vector
    /// index where that podcast is located.
    pub fn id_to_index(&self, id: i32) -> Option<i32> {
        let borrowed = self.borrow();
        let item = borrowed.iter().position(|val| val.id == Some(id));
        return match item {
            Some(index) => Some(index as i32),
            None => None
        };
    }
}

impl LockVec<Episode> {
    /// This clones the episode at the given index (`ep_index`). Note
    /// that `clone_episode()` is also implemented for LockVec<Podcast>,
    /// and can be used at that level as well if given a podcast index.
    pub fn clone_episode(&self, index: i32) -> Option<Episode> {
        let ep_list = self.borrow();
        return match ep_list.get(index as usize) {
            Some(ep) => Some(ep.clone()),
            None => None,
        };
    }

    /// Given an episode ID (from the database), this provides the vector
    /// index where that episode is located.
    pub fn id_to_index(&self, id: i32) -> Option<i32> {
        let borrowed = self.borrow();
        let item = borrowed.iter().position(|val| val.id == Some(id));
        return match item {
            Some(index) => Some(index as i32),
            None => None
        };
    }
}


/// Overarching Message enum that allows multiple threads to communicate
/// back to the main thread with a single enum type.
#[derive(Debug)]
pub enum Message {
    Ui(UiMsg),
    Feed(FeedMsg),
    Dl(DownloadMsg),
}


// some utilities for dealing with UTF-8 substrings that split properly
// on character boundaries. From:
// https://users.rust-lang.org/t/how-to-get-a-substring-of-a-string/1351/11
// Note that using UnicodeSegmentation::graphemes() from the
// `unicode-segmentation` crate might still end up being preferable...
pub trait StringUtils {
    fn substring(&self, start: usize, len: usize) -> &str;
    fn slice(&self, range: impl RangeBounds<usize>) -> &str;
}

impl StringUtils for str {
    fn substring(&self, start: usize, len: usize) -> &str {
        let mut char_pos = 0;
        let mut byte_start = 0;
        let mut it = self.chars();
        loop {
            if char_pos == start { break; }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_start += c.len_utf8();
            }
            else { break; }
        }
        char_pos = 0;
        let mut byte_end = byte_start;
        loop {
            if char_pos == len { break; }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_end += c.len_utf8();
            }
            else { break; }
        }
        &self[byte_start..byte_end]
    }
    fn slice(&self, range: impl RangeBounds<usize>) -> &str {
        let start = match range.start_bound() {
            Bound::Included(bound) | Bound::Excluded(bound) => *bound,
            Bound::Unbounded => 0,
        };
        let len = match range.end_bound() {
            Bound::Included(bound) => *bound + 1,
            Bound::Excluded(bound) => *bound,
            Bound::Unbounded => self.len(),
        } - start;
        self.substring(start, len)
    }
}