use std::rc::Rc;
use core::cell::RefCell;

mod config;
mod keymap;
mod db;
mod ui;
mod types;
mod feeds;
mod downloads;
mod play_file;

use crate::ui::{UI, UiMessage};
use crate::db::Database;
use crate::types::{Podcast, MutableVec};

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
    let db_inst = Database::connect();
    let config = config::parse_config_file("./config.toml");
    let download_manager = downloads::DownloadManager::new();

    // create vector of podcasts, where references are checked at runtime;
    // this is necessary because we want main.rs to hold the "ground truth"
    // list of podcasts, and it must be mutable, but UI needs to check
    // this list and update the screen when necessary
    let podcast_list: MutableVec<Podcast> = Rc::new(
        RefCell::new(db_inst.get_podcasts()));
    let mut ui = UI::new(&config, &podcast_list);

    loop {
        let mess = ui.getch();
        match mess {
            UiMessage::Quit => break,

            UiMessage::AddFeed(url) => {
                match feeds::get_feed_data(url) {
                    Ok(pod) => {
                        match db_inst.insert_podcast(pod) {
                            Ok(num_ep) => {
                                *podcast_list.borrow_mut() = db_inst.get_podcasts();
                                ui.update_menus();
                                ui.spawn_msg_win(
                                &format!("Successfully added {} episodes.", num_ep), 5000);
                            },
                            Err(_err) => ui.spawn_msg_win("Error adding podcast to database.", 5000),
                        }
                    },
                    Err(_err) => ui.spawn_msg_win("Error retrieving RSS feed.", 5000),
                }
            },

            UiMessage::Play(pod_index, ep_index) => {
                let borrowed_pod_list = podcast_list.borrow();
                let borrowed_podcast = borrowed_pod_list
                    .get(pod_index as usize).unwrap();
                let borrowed_ep_list = borrowed_podcast
                    .episodes.borrow();
                // TODO: Try to find a way to do this without having
                // to clone the episode...
                let episode = borrowed_ep_list
                    .get(ep_index as usize).unwrap().clone();

                match episode.path {
                    Some(path) => {
                        match path.to_str() {
                            Some(p) => {
                                if let Err(_) = play_file::execute(&config.play_command, &p) {
                                    ui.spawn_msg_win("Error: Could not play file. Check configuration.", 5000);
                                }
                            },
                            None => ui.spawn_msg_win("Error: Filepath is not valid Unicode.", 5000),
                        }
                    },
                    None => {
                        if let Err(_) = play_file::execute(&config.play_command, &episode.url) {
                            ui.spawn_msg_win("Error: Could not stream URL.", 5000);
                        }
                    }
                }
            },

            UiMessage::Download(pod_index, ep_index) => {
                let mut success = false;

                // limit scope so that we drop the mutable borrow;
                // otherwise, will panic once we try to update the UI
                {
                    let borrowed_pod_list = podcast_list.borrow();
                    let borrowed_podcast = borrowed_pod_list
                        .get(pod_index as usize).unwrap();
                    let mut borrowed_ep_list = borrowed_podcast
                        .episodes.borrow_mut();
                    // TODO: Try to find a way to do this without having
                    // to clone the episode...
                    let mut episode = borrowed_ep_list
                        .get(ep_index as usize).unwrap().clone();

                    // add directory for podcast, create if it does not exist
                    let mut download_path = config.download_path.clone();
                    download_path.push(borrowed_podcast.title.clone());
                    if let Err(_) = std::fs::create_dir_all(&download_path) {
                        ui.spawn_msg_win(
                            &format!("Could not create dir: {}", borrowed_podcast.title.clone())[..],
                            5000);
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
                    ui.update_menus();
                }
            },

            UiMessage::DownloadAll(pod_index) => {
                let mut success = false;

                // limit scope so that we drop the mutable borrow;
                // otherwise, will panic once we try to update the UI
                {
                    let borrowed_pod_list = podcast_list.borrow();
                    let borrowed_podcast = borrowed_pod_list
                        .get(pod_index as usize).unwrap();
                    let mut borrowed_ep_list = borrowed_podcast
                        .episodes.borrow_mut();

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
                        ui.spawn_msg_win(
                            &format!("Could not create dir: {}", borrowed_podcast.title.clone())[..],
                            5000);
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
                    ui.update_menus();
                }
            },

            UiMessage::Noop => (),
        }
    }

    ui.tear_down();
}