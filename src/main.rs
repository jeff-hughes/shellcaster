use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

mod config;
mod keymap;
mod db;
mod ui;
mod types;
mod feeds;
mod downloads;
mod play_file;

use crate::types::*;
use crate::ui::{UI, UiMsg};
use crate::db::Database;
use crate::feeds::FeedMsg;

/// Enum used for communicating with other threads.
#[derive(Debug)]
pub enum MainMessage {
    UiUpdateMenus,
    UiSpawnMsgWin(String, u64),
    UiTearDown,
}

/// Main controller for shellcaster program.
/// 
/// Setup involves connecting to the sqlite database (creating it if 
/// necessary), then querying the list of podcasts and episodes. This
/// is then passed off to the UI, which instantiates the menus displaying
/// the podcast info.
/// 
/// After this, the program enters a loop that listens for user keyboard
/// input, and dispatches to the proper module as necessary. User input
/// to quit the program breaks the loop, tears down the UI, and ends the
/// program.
fn main() {
    // figure out where config file is located -- either specified from
    // command line args, or using default config location for OS
    let mut config_path;
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        3 => {
            config_path = PathBuf::from(&args[2]);
        },
        _ => {
            let default_config = dirs::config_dir();
            match default_config {
                Some(path) => {
                    config_path = path;
                    config_path.push("shellcaster");
                    config_path.push("config.toml");
                },
                None => panic!("Could not identify your operating system's default directory to store configuration files. Please specify paths manually using config.toml and use `-c` or `--config` flag to specify where config.toml is located when launching the program."),
            } 
        }
    }
    let config = config::parse_config_file(&config_path);
    
    let db_inst = Database::connect(&config.config_path);
    let download_manager = downloads::DownloadManager::new();

    // create vector of podcasts, where references are checked at runtime;
    // this is necessary because we want main.rs to hold the "ground truth"
    // list of podcasts, and it must be mutable, but UI needs to check
    // this list and update the screen when necessary
    let podcast_list: MutableVec<Podcast> = Arc::new(
        Mutex::new(db_inst.get_podcasts()));

    // create transmitters and receivers for passing messages between threads
    let (tx_to_ui, rx_from_main) = mpsc::channel();
    let (tx_to_main, rx_to_main) = mpsc::channel();
    let tx_ui_to_main = mpsc::Sender::clone(&tx_to_main);
    let ui_thread = UI::spawn(config.clone(), Arc::clone(&podcast_list), rx_from_main, tx_ui_to_main);
        // TODO: Can we do this without cloning the config?

    let mut message_iter = rx_to_main.iter();
    loop {
        if let Some(message) = message_iter.next() {
            match message {
                Message::Ui(UiMsg::Quit) => break,

                Message::Ui(UiMsg::AddFeed(url)) => {
                    let tx_feeds_to_main = mpsc::Sender::clone(&tx_to_main);
                    let _ = feeds::spawn_feed_checker(tx_feeds_to_main, url, None);
                },

                Message::Feed(FeedMsg::NewData(pod)) => {
                    match db_inst.insert_podcast(pod) {
                        Ok(num_ep) => {
                            *podcast_list.lock().unwrap() = db_inst.get_podcasts();
                            tx_to_ui.send(MainMessage::UiUpdateMenus).unwrap();
                            tx_to_ui.send(MainMessage::UiSpawnMsgWin(format!("Successfully added {} episodes.", num_ep), 5000)).unwrap();
                        },
                        Err(_err) => tx_to_ui.send(MainMessage::UiSpawnMsgWin("Error adding podcast to database.".to_string(), 5000)).unwrap(),
                    }
                },

                Message::Feed(FeedMsg::Error) => tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                    "Error retrieving RSS feed.".to_string(), 5000)).unwrap(),

                Message::Ui(UiMsg::Sync(pod_index)) => {
                    let url;
                    let id;
                    {
                        let borrowed_pod_list = podcast_list.lock().unwrap();
                        let borrowed_podcast = borrowed_pod_list
                            .get(pod_index as usize).unwrap();
                        url = borrowed_podcast.url.clone();
                        id = borrowed_podcast.id;
                    }
                    let tx_feeds_to_main = mpsc::Sender::clone(&tx_to_main);
                    let _ = feeds::spawn_feed_checker(tx_feeds_to_main, url, id);
                },

                Message::Feed(FeedMsg::SyncData(pod)) => {
                    let title = pod.title.clone();
                    match db_inst.update_podcast(pod) {
                        Ok(_) => {
                            *podcast_list.lock().unwrap() = db_inst.get_podcasts();
                            tx_to_ui.send(MainMessage::UiUpdateMenus).unwrap();
                            tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                                format!("Synchronized {}.", title), 5000)).unwrap();
                        },
                        Err(_err) => tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                            format!("Error synchronizing {}.", title), 5000)).unwrap(),
                    }
                },

                Message::Ui(UiMsg::SyncAll) => {
                    // We pull out the data we need here first, so we can
                    // stop borrowing the podcast list as quickly as possible.
                    // Slightly less efficient (two loops instead of
                    // one), but then it won't block other tasks that
                    // need to access the list.
                    let mut pod_data = Vec::new();
                    {
                        let borrowed_pod_list = podcast_list.lock().unwrap();
                        for podcast in borrowed_pod_list.iter() {
                            pod_data.push((podcast.url.clone(), podcast.id));
                        }
                    }
                    for data in pod_data.iter() {
                        let url = data.0.clone();
                        let id = data.1;

                        let tx_feeds_to_main = mpsc::Sender::clone(&tx_to_main);
                        let _ = feeds::spawn_feed_checker(tx_feeds_to_main, url, id);
                    }
                },

                Message::Ui(UiMsg::Play(pod_index, ep_index)) => {
                    let episode;
                    {
                        let borrowed_pod_list = podcast_list.lock().unwrap();
                        let borrowed_podcast = borrowed_pod_list
                            .get(pod_index as usize).unwrap();
                        let borrowed_ep_list = borrowed_podcast
                            .episodes.lock().unwrap();
                        // TODO: Try to find a way to do this without having
                        // to clone the episode...
                        episode = borrowed_ep_list
                            .get(ep_index as usize).unwrap().clone();
                    }

                    match episode.path {
                        Some(path) => {
                            match path.to_str() {
                                Some(p) => {
                                    if let Err(_) = play_file::execute(&config.play_command, &p) {
                                        tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                                            "Error: Could not play file. Check configuration.".to_string(), 5000)).unwrap();
                                    }
                                },
                                None => tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                                    "Error: Filepath is not valid Unicode.".to_string(), 5000)).unwrap(),
                            }
                        },
                        None => {
                            if let Err(_) = play_file::execute(&config.play_command, &episode.url) {
                                tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                                    "Error: Could not stream URL.".to_string(), 5000)).unwrap();
                            }
                        }
                    }
                },

                Message::Ui(UiMsg::Download(pod_index, ep_index)) => {
                    let mut success = false;

                    // limit scope so that we drop the mutable borrow;
                    // otherwise, will panic once we try to update the UI
                    {
                        let borrowed_pod_list = podcast_list.lock().unwrap();
                        let borrowed_podcast = borrowed_pod_list
                            .get(pod_index as usize).unwrap();
                        let mut borrowed_ep_list = borrowed_podcast
                            .episodes.lock().unwrap();
                        // TODO: Try to find a way to do this without having
                        // to clone the episode...
                        let mut episode = borrowed_ep_list
                            .get(ep_index as usize).unwrap().clone();

                        // add directory for podcast, create if it does not exist
                        let mut download_path = config.download_path.clone();
                        download_path.push(borrowed_podcast.title.clone());
                        if let Err(_) = std::fs::create_dir_all(&download_path) {
                            tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                                format!("Could not create dir: {}", borrowed_podcast.title.clone()), 5000)).unwrap();
                        }

                        let file_paths = download_manager
                            .download_list(&vec![&episode], &download_path);

                        match &file_paths[0] {
                            Ok(ff) => {
                                match ff {
                                    Some(path) => {
                                        let _ = db_inst.insert_file(episode.id.unwrap(), &path);
                                        episode.path = Some(path.clone());
                                        borrowed_ep_list[ep_index as usize] = episode;
                                        success = true;
                                    },
                                    None => (),
                                }
                            },
                            Err(_) => (),
                        }
                    }

                    if success {
                        tx_to_ui.send(MainMessage::UiUpdateMenus).unwrap();
                    }
                },

                Message::Ui(UiMsg::DownloadAll(pod_index)) => {
                    let mut success = false;

                    // limit scope so that we drop the mutable borrow;
                    // otherwise, will panic once we try to update the UI
                    {
                        let borrowed_pod_list = podcast_list.lock().unwrap();
                        let borrowed_podcast = borrowed_pod_list
                            .get(pod_index as usize).unwrap();
                        let mut borrowed_ep_list = borrowed_podcast
                            .episodes.lock().unwrap();

                        // TODO: Try to find a way to do this without having
                        // to clone the episodes...
                        let mut episodes = Vec::new();
                        let mut episode_refs = Vec::new();
                        for e in borrowed_ep_list.iter() {
                            episodes.push(e.clone());
                            episode_refs.push(e);
                        }

                        // add directory for podcast, create if it does not exist
                        let mut download_path = config.download_path.clone();
                        download_path.push(borrowed_podcast.title.clone());
                        if let Err(_) = std::fs::create_dir_all(&download_path) {
                            tx_to_ui.send(MainMessage::UiSpawnMsgWin(
                                format!("Could not create dir: {}", borrowed_podcast.title.clone()), 5000)).unwrap();
                        }

                        let file_paths = download_manager
                            .download_list(&episode_refs, &download_path);

                        for (i, f) in file_paths.iter().enumerate() {
                            match f {
                                Ok(ff) => {
                                    match ff {
                                        Some(path) => {
                                            episodes[i].path = Some(path.clone());
                                            borrowed_ep_list[i] = episodes[i].clone();
                                            success = true;
                                        },
                                        None => (),
                                    }
                                },
                                Err(_) => (),
                            }
                        }
                    }

                    // update if even one file downloaded successfully
                    if success {
                        tx_to_ui.send(MainMessage::UiUpdateMenus).unwrap();
                    }
                },

                Message::Ui(UiMsg::Noop) => (),
            }
        }
    }

    tx_to_ui.send(MainMessage::UiTearDown).unwrap();
    ui_thread.join().unwrap();  // wait for UI thread to finish teardown
}