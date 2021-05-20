use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use unicode_segmentation::UnicodeSegmentation;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use nohash_hasher::BuildNoHashHasher;
use regex::Regex;

use crate::downloads::DownloadMsg;
use crate::feeds::FeedMsg;
use crate::ui::UiMsg;

lazy_static! {
    /// Regex for removing "A", "An", and "The" from the beginning of
    /// podcast titles
    static ref RE_ARTICLES: Regex = Regex::new(r"^(a|an|the) ").expect("Regex error");
}

/// Defines interface used for both podcasts and episodes, to be
/// used and displayed in menus.
pub trait Menuable {
    fn get_id(&self) -> i64;
    fn get_title(&self, length: usize) -> String;
    fn is_played(&self) -> bool;
}

/// Struct holding data about an individual podcast feed. This includes a
/// (possibly empty) vector of episodes.
#[derive(Debug, Clone)]
pub struct Podcast {
    pub id: i64,
    pub title: String,
    pub sort_title: String,
    pub url: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub explicit: Option<bool>,
    pub last_checked: DateTime<Utc>,
    pub episodes: LockVec<Episode>,
}

impl Podcast {
    /// Counts and returns the number of unplayed episodes in the podcast.
    fn num_unplayed(&self) -> usize {
        return self
            .episodes
            .map(|ep| !ep.is_played() as usize)
            .iter()
            .sum();
    }
}

impl Menuable for Podcast {
    /// Returns the database ID for the podcast.
    fn get_id(&self) -> i64 {
        return self.id;
    }

    /// Returns the title for the podcast, up to length characters.
    fn get_title(&self, length: usize) -> String {
        let mut title_length = length;

        // if the size available is big enough, we add the unplayed data
        // to the end
        if length > crate::config::PODCAST_UNPLAYED_TOTALS_LENGTH {
            let meta_str = format!("({}/{})", self.num_unplayed(), self.episodes.len());
            title_length = length - meta_str.chars().count() - 2;

            let out = self.title.substr(0, title_length);

            return format!(
                " {} {:>width$} ",
                out,
                meta_str,
                width = length - out.grapheme_len() - 2
            ); // this pads spaces between title and totals
        } else {
            return format!(" {} ", self.title.substr(0, title_length - 2));
        }
    }

    fn is_played(&self) -> bool {
        return self.num_unplayed() == 0;
    }
}

impl PartialEq for Podcast {
    fn eq(&self, other: &Self) -> bool {
        return self.sort_title == other.sort_title;
    }
}
impl Eq for Podcast {}

impl PartialOrd for Podcast {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return Some(self.cmp(other));
    }
}

impl Ord for Podcast {
    fn cmp(&self, other: &Self) -> Ordering {
        return self.sort_title.cmp(&other.sort_title);
    }
}


/// Struct holding data about an individual podcast episode. Most of this
/// is metadata, but if the episode has been downloaded to the local
/// machine, the filepath will be included here as well. `played`
/// indicates whether the podcast has been marked as played or unplayed.
#[derive(Debug, Clone)]
pub struct Episode {
    pub id: i64,
    pub pod_id: i64,
    pub title: String,
    pub url: String,
    pub description: String,
    pub pubdate: Option<DateTime<Utc>>,
    pub duration: Option<i64>,
    pub path: Option<PathBuf>,
    pub played: bool,
}

impl Episode {
    /// Formats the duration in seconds into an HH:MM:SS format.
    pub fn format_duration(&self) -> String {
        return match self.duration {
            Some(dur) => {
                let mut seconds = dur;
                let hours = seconds / 3600;
                seconds -= hours * 3600;
                let minutes = seconds / 60;
                seconds -= minutes * 60;
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            }
            None => "--:--:--".to_string(),
        };
    }
}

impl Menuable for Episode {
    /// Returns the database ID for the episode.
    fn get_id(&self) -> i64 {
        return self.id;
    }

    /// Returns the title for the episode, up to length characters.
    fn get_title(&self, length: usize) -> String {
        let out = match self.path {
            Some(_) => {
                let title = self.title.substr(0, length - 4);
                format!("[D] {}", title)
            }
            None => self.title.substr(0, length),
        };
        if length > crate::config::EPISODE_PUBDATE_LENGTH {
            let dur = self.format_duration();
            let meta_dur = format!("[{}]", dur);

            if let Some(pubdate) = self.pubdate {
                // print pubdate and duration
                let pd = pubdate.format("%F");
                let meta_str = format!("({}) {}", pd, meta_dur);
                let added_len = meta_str.chars().count();

                let out_added = out.substr(0, length - added_len - 2);
                return format!(
                    " {} {:>width$} ",
                    out_added,
                    meta_str,
                    width = length - out_added.grapheme_len() - 2
                );
            } else {
                // just print duration
                let out_added = out.substr(0, length - meta_dur.chars().count() - 2);
                return format!(
                    " {} {:>width$} ",
                    out_added,
                    meta_dur,
                    width = length - out_added.grapheme_len() - 2
                );
            }
        } else if length > crate::config::EPISODE_DURATION_LENGTH {
            let dur = self.format_duration();
            let meta_dur = format!("[{}]", dur);
            let out_added = out.substr(0, length - meta_dur.chars().count() - 2);
            return format!(
                " {} {:>width$} ",
                out_added,
                meta_dur,
                width = length - out_added.grapheme_len() - 2
            );
        } else {
            return format!(" {} ", out.substr(0, length - 2));
        }
    }

    fn is_played(&self) -> bool {
        return self.played;
    }
}


/// Struct holding data about an individual podcast feed, before it has
/// been inserted into the database. This includes a
/// (possibly empty) vector of episodes.
#[derive(Debug, Clone)]
pub struct PodcastNoId {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub explicit: Option<bool>,
    pub last_checked: DateTime<Utc>,
    pub episodes: Vec<EpisodeNoId>,
}

/// Struct holding data about an individual podcast episode, before it
/// has been inserted into the database.
#[derive(Debug, Clone)]
pub struct EpisodeNoId {
    pub title: String,
    pub url: String,
    pub description: String,
    pub pubdate: Option<DateTime<Utc>>,
    pub duration: Option<i64>,
}

/// Struct holding data about an individual podcast episode, specifically
/// for the popup window that asks users which new episodes they wish to
/// download.
#[derive(Debug, Clone)]
pub struct NewEpisode {
    pub id: i64,
    pub pod_id: i64,
    pub title: String,
    pub pod_title: String,
    pub selected: bool,
}

impl Menuable for NewEpisode {
    /// Returns the database ID for the episode.
    fn get_id(&self) -> i64 {
        return self.id;
    }

    /// Returns the title for the episode, up to length characters.
    fn get_title(&self, length: usize) -> String {
        let selected = if self.selected { "✓" } else { " " };
        let full_string = format!("[{}] {} ({})", selected, self.title, self.pod_title);
        return full_string.substr(0, length);
    }

    fn is_played(&self) -> bool {
        return true;
    }
}

/// Struct used to hold a vector of data inside a reference-counted
/// mutex, to allow for multiple owners of mutable data.
/// Primarily, the LockVec is used to provide methods that abstract
/// away some of the logic necessary for borrowing and locking the
/// Arc<Mutex<_>>.
#[derive(Debug)]
pub struct LockVec<T>
where T: Clone + Menuable
{
    data: Arc<Mutex<HashMap<i64, T, BuildNoHashHasher<i64>>>>,
    order: Arc<Mutex<Vec<i64>>>,
}

impl<T: Clone + Menuable> LockVec<T> {
    /// Create a new LockVec.
    pub fn new(data: Vec<T>) -> LockVec<T> {
        let mut hm = HashMap::with_hasher(BuildNoHashHasher::default());
        let mut order = Vec::new();
        for i in data.into_iter() {
            let id = i.get_id();
            hm.insert(i.get_id(), i);
            order.push(id);
        }

        return LockVec {
            data: Arc::new(Mutex::new(hm)),
            order: Arc::new(Mutex::new(order)),
        };
    }

    /// Lock the LockVec hashmap for reading/writing.
    pub fn borrow_map(&self) -> MutexGuard<HashMap<i64, T, BuildNoHashHasher<i64>>> {
        return self.data.lock().expect("Mutex error");
    }

    /// Lock the LockVec order vector for reading/writing.
    pub fn borrow_order(&self) -> MutexGuard<Vec<i64>> {
        return self.order.lock().expect("Mutex error");
    }

    /// Lock the LockVec hashmap for reading/writing.
    #[allow(clippy::type_complexity)]
    pub fn borrow(
        &self,
    ) -> (
        MutexGuard<HashMap<i64, T, BuildNoHashHasher<i64>>>,
        MutexGuard<Vec<i64>>,
    ) {
        return (
            self.data.lock().expect("Mutex error"),
            self.order.lock().expect("Mutex error"),
        );
    }

    /// Given an id, this takes a new T and replaces the old T with that
    /// id.
    pub fn replace(&self, id: i64, t: T) {
        let mut borrowed = self.borrow_map();
        borrowed.insert(id, t);
    }

    /// Empty out and replace all the data in the LockVec.
    pub fn replace_all(&self, data: Vec<T>) {
        let (mut map, mut order) = self.borrow();
        map.clear();
        order.clear();
        for i in data.into_iter() {
            let id = i.get_id();
            map.insert(i.get_id(), i);
            order.push(id);
        }
    }

    /// Maps a closure to every element in the LockVec, in the same way
    /// as an Iterator. However, to avoid issues with keeping the borrow
    /// alive, the function returns a Vec of the collected results,
    /// rather than an iterator.
    pub fn map<B, F>(&self, mut f: F) -> Vec<B>
    where F: FnMut(&T) -> B {
        let (map, order) = self.borrow();
        return order
            .iter()
            .map(|id| f(map.get(id).expect("Index error in LockVec")))
            .collect();
    }

    /// Maps a closure to a single element in the LockVec, specified by
    /// `id`. If there is no element `id`, this returns None.
    pub fn map_single<B, F>(&self, id: i64, f: F) -> Option<B>
    where F: FnOnce(&T) -> B {
        let borrowed = self.borrow_map();
        return match borrowed.get(&id) {
            Some(item) => Some(f(item)),
            None => return None,
        };
    }

    /// Maps a closure to a single element in the LockVec, specified by
    /// `index` (position order). If there is no element at that index,
    /// this returns None.
    pub fn map_single_by_index<B, F>(&self, index: usize, f: F) -> Option<B>
    where F: FnOnce(&T) -> B {
        let order = self.borrow_order();
        return match order.get(index) {
            Some(id) => self.map_single(*id, f),
            None => None,
        };
    }

    /// Maps a closure to every element in the LockVec, in the same way
    /// as the `filter_map()` does on an Iterator, both mapping and
    /// filtering, over a specified range.
    /// Does not check if the range is valid!
    /// However, to avoid issues with keeping the borrow
    /// alive, the function returns a Vec of the collected results,
    /// rather than an iterator.
    pub fn map_by_range<B, F>(&self, start: usize, end: usize, mut f: F) -> Vec<B>
    where F: FnMut(&T) -> Option<B> {
        let (map, order) = self.borrow();
        return (start..end)
            .into_iter()
            .filter_map(|id| {
                f(map
                    .get(order.get(id).expect("Index error in LockVec"))
                    .expect("Index error in LockVec"))
            })
            .collect();
    }

    /// Maps a closure to every element in the LockVec, in the same way
    /// as the `filter_map()` does on an Iterator, both mapping and
    /// filtering. However, to avoid issues with keeping the borrow
    /// alive, the function returns a Vec of the collected results,
    /// rather than an iterator.
    pub fn filter_map<B, F>(&self, mut f: F) -> Vec<B>
    where F: FnMut(&T) -> Option<B> {
        let (map, order) = self.borrow();
        return order
            .iter()
            .filter_map(|id| f(map.get(id).expect("Index error in LockVec")))
            .collect();
    }

    /// Returns the number of items in the LockVec.
    pub fn len(&self) -> usize {
        return self.borrow_order().len();
    }

    /// Returns whether or not there are any items in the LockVec.
    pub fn is_empty(&self) -> bool {
        return self.borrow_order().is_empty();
    }
}

impl<T: Clone + Menuable> Clone for LockVec<T> {
    fn clone(&self) -> Self {
        return LockVec {
            data: Arc::clone(&self.data),
            order: Arc::clone(&self.order),
        };
    }
}

impl LockVec<Podcast> {
    /// This clones the podcast with the given id.
    pub fn clone_podcast(&self, id: i64) -> Option<Podcast> {
        let pod_map = self.borrow_map();
        return match pod_map.get(&id) {
            Some(pod) => Some(pod.clone()),
            None => None,
        };
    }

    /// This clones the episode with the given id (`ep_id`), from
    /// the podcast with the given id (`pod_id`). Note that if you
    /// are already borrowing the episode list for a podcast, you can
    /// also use `clone_episode()` directly on that list.
    pub fn clone_episode(&self, pod_id: i64, ep_id: i64) -> Option<Episode> {
        let pod_map = self.borrow_map();
        if let Some(pod) = pod_map.get(&pod_id) {
            return pod.episodes.clone_episode(ep_id);
        }
        return None;
    }
}

impl LockVec<Episode> {
    /// This clones the episode with the given id (`ep_id`). Note
    /// that `clone_episode()` is also implemented for LockVec<Podcast>,
    /// and can be used at that level as well if given a podcast id.
    pub fn clone_episode(&self, ep_id: i64) -> Option<Episode> {
        let ep_map = self.borrow_map();
        return match ep_map.get(&ep_id) {
            Some(ep) => Some(ep.clone()),
            None => None,
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


/// Some helper functions for dealing with Unicode strings.
pub trait StringUtils {
    fn substr(&self, start: usize, length: usize) -> String;
    fn grapheme_len(&self) -> usize;
}

impl StringUtils for String {
    /// Takes a slice of the String, properly separated at Unicode
    /// grapheme boundaries. Returns a new String.
    fn substr(&self, start: usize, length: usize) -> String {
        return self
            .graphemes(true)
            .skip(start)
            .take(length)
            .collect::<String>();
    }

    /// Counts the total number of Unicode graphemes in the String.
    fn grapheme_len(&self) -> usize {
        return self.graphemes(true).count();
    }
}
