use std::rc::Rc;
use core::cell::RefCell;

mod config;
mod keymap;
mod db;
mod ui;
mod types;
mod feeds;
mod downloads;

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
            UiMessage::Download(pod_index, ep_index) => {
                let borrowed_pod_list = podcast_list.borrow();
                let borrowed_ep_list = borrowed_pod_list
                    .get(pod_index as usize)
                    .unwrap()
                    .episodes.borrow();
                let episode = borrowed_ep_list
                    .get(ep_index as usize).unwrap();
                download_manager.download_list(&vec![episode], &config.download_path);
            },
            UiMessage::Noop => (),
        }
    }

    ui.tear_down();
}