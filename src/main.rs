use std::process;
use std::path::PathBuf;
use std::sync::mpsc;

mod config;
mod keymap;
mod db;
mod ui;
mod types;
mod feeds;
mod downloads;
mod play_file;

use crate::types::*;
use crate::config::Config;
use crate::ui::{UI, UiMsg};
use crate::db::Database;
use crate::feeds::FeedMsg;
use crate::downloads::DownloadMsg;

// Specifies how long, in milliseconds, to display messages at the
// bottom of the screen in the UI.
const MESSAGE_TIME: u64 = 5000;

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
    // SETUP -----------------------------------------------------------

    // figure out where config file is located -- either specified from
    // command line args, or using default config location for OS
    let args: Vec<String> = std::env::args().collect();
    let config_path = get_config_path(&args)
        .unwrap_or_else(|| {
            println!("Could not identify your operating system's default directory to store configuration files. Please specify paths manually using config.toml and use `-c` or `--config` flag to specify where config.toml is located when launching the program.");
            process::exit(1);
        });
    let config = Config::new(&config_path);

    let mut main_ctrl = MainController::new(config);


    // MAIN LOOP --------------------------------------------------------

    // wait for messages from the UI and process
    let mut message_iter = main_ctrl.rx_to_main.iter();
    while let Some(message) = message_iter.next() {
        match message {
            Message::Ui(UiMsg::Quit) => break,

            Message::Ui(UiMsg::AddFeed(url)) => {
                let tx_feeds_to_main = mpsc::Sender::clone(&main_ctrl.tx_to_main);
                let _ = feeds::spawn_feed_checker(tx_feeds_to_main, url, None);
            },

            Message::Feed(FeedMsg::NewData(pod)) => main_ctrl.add_or_sync_data(pod, false),

            Message::Feed(FeedMsg::Error) => main_ctrl.msg_to_ui("Error retrieving RSS feed.".to_string()),

            Message::Ui(UiMsg::Sync(pod_index)) => {
                let pod = main_ctrl.podcasts.clone_podcast(pod_index).unwrap();
                let tx_feeds_to_main = mpsc::Sender::clone(&main_ctrl.tx_to_main);
                let _ = feeds::spawn_feed_checker(
                    tx_feeds_to_main,
                    pod.url,
                    pod.id);
            },

            Message::Feed(FeedMsg::SyncData(pod)) => main_ctrl.add_or_sync_data(pod, true),

            Message::Ui(UiMsg::SyncAll) => {
                // We pull out the data we need here first, so we can
                // stop borrowing the podcast list as quickly as possible.
                // Slightly less efficient (two loops instead of
                // one), but then it won't block other tasks that
                // need to access the list.
                let mut pod_data = Vec::new();
                {
                    let borrowed_pod_list = main_ctrl.podcasts
                        .borrow();
                    for podcast in borrowed_pod_list.iter() {
                        pod_data.push((podcast.url.clone(), podcast.id));
                    }
                }
                for data in pod_data.into_iter() {
                    let url = data.0;
                    let id = data.1;

                    let tx_feeds_to_main = mpsc::Sender::clone(&main_ctrl.tx_to_main);
                    let _ = feeds::spawn_feed_checker(tx_feeds_to_main, url, id);
                }
            },

            Message::Ui(UiMsg::Play(pod_index, ep_index)) => main_ctrl.play_file(pod_index, ep_index),

            Message::Ui(UiMsg::MarkPlayed(pod_index, ep_index, played)) => {
                let mut podcast = main_ctrl.podcasts.clone_podcast(pod_index).unwrap();
                let mut any_unplayed = false;

                // TODO: Try to find a way to do this without having
                // to clone the episode...
                let mut episode = podcast.episodes.clone_episode(ep_index).unwrap();
                episode.played = played;
                
                main_ctrl.db.set_played_status(episode.id.unwrap(), played);
                podcast.episodes.replace(ep_index, episode).unwrap();

                {
                    // recheck if there are any unplayed episodes for the
                    // selected podcast
                    let borrowed_ep_list = podcast
                        .episodes.borrow();
                    for ep in borrowed_ep_list.iter() {
                        if !ep.played {
                            any_unplayed = true;
                            break;
                        }
                    }
                }
                if any_unplayed != podcast.any_unplayed {
                    podcast.any_unplayed = any_unplayed;
                    main_ctrl.podcasts.replace(pod_index, podcast).unwrap();
                }
            },

            Message::Ui(UiMsg::MarkAllPlayed(pod_index, played)) => {
                let mut podcast = main_ctrl.podcasts.clone_podcast(pod_index).unwrap();
                {
                    let mut borrowed_ep_list = podcast
                        .episodes.borrow();

                    for ep in borrowed_ep_list.iter() {
                        main_ctrl.db.set_played_status(ep.id.unwrap(), played);
                    }

                    *borrowed_ep_list = main_ctrl.db.get_episodes(podcast.id.unwrap());
                }

                podcast.any_unplayed = !played;
                main_ctrl.podcasts.replace(pod_index, podcast).unwrap();
                main_ctrl.tx_to_ui.send(MainMessage::UiUpdateMenus).unwrap();
            },

            Message::Ui(UiMsg::Download(pod_index, ep_index)) => {
                let pod_title;
                {
                    let borrowed_podcast_list = main_ctrl.podcasts.borrow();
                    pod_title = borrowed_podcast_list[pod_index as usize].title.clone();
                }
                let episode = main_ctrl.podcasts
                    .clone_episode(pod_index, ep_index).unwrap();

                // add directory for podcast, create if it does not exist
                let mut download_path = main_ctrl.config.download_path.clone();
                download_path.push(pod_title.clone());
                if std::fs::create_dir_all(&download_path).is_err() {
                    main_ctrl.msg_to_ui(
                        format!("Could not create dir: {}", pod_title.clone()));
                }

                main_ctrl.download_manager.download_list(
                    &[&episode], &download_path);
            },

            Message::Dl(DownloadMsg::Complete(ep_data)) => {
                let _ = main_ctrl.db.insert_file(ep_data.id, &ep_data.file_path);
                {
                    let pod_index = main_ctrl.podcasts
                        .id_to_index(ep_data.pod_id).unwrap();
                    // TODO: Try to do this without cloning the podcast...
                    let podcast = main_ctrl.podcasts
                        .clone_podcast(pod_index).unwrap();

                    let ep_index = podcast.episodes
                        .id_to_index(ep_data.id).unwrap();
                    let mut episode = podcast.episodes.clone_episode(ep_index).unwrap();
                    episode.path = Some(ep_data.file_path.clone());
                    podcast.episodes.replace(ep_index, episode).unwrap();
                }

                main_ctrl.tx_to_ui.send(MainMessage::UiUpdateMenus).unwrap();
            },

            Message::Dl(DownloadMsg::ResponseError(_)) => {
                main_ctrl.msg_to_ui("Error sending download request.".to_string());
            },
            Message::Dl(DownloadMsg::ResponseDataError(_)) => {
                main_ctrl.msg_to_ui("Error downloading episode.".to_string()); 
            },
            Message::Dl(DownloadMsg::FileCreateError(_)) => {
                main_ctrl.msg_to_ui("Error creating file.".to_string()); 
            },
            Message::Dl(DownloadMsg::FileWriteError(_)) => {
                main_ctrl.msg_to_ui("Error writing file to disk.".to_string()); 
            },

            Message::Ui(UiMsg::DownloadAll(pod_index)) => {
                // TODO: Try to do this without cloning the podcast...
                let podcast = main_ctrl.podcasts
                    .clone_podcast(pod_index).unwrap();
                let pod_title = podcast.title.clone();
                let borrowed_ep_list = podcast
                    .episodes.borrow();

                let mut episodes = Vec::new();
                for e in borrowed_ep_list.iter() {
                    episodes.push(e);
                }

                // add directory for podcast, create if it does not exist
                let mut download_path = main_ctrl.config.download_path.clone();
                download_path.push(pod_title.clone());
                if std::fs::create_dir_all(&download_path).is_err() {
                    main_ctrl.msg_to_ui(
                        format!("Could not create dir: {}", pod_title));
                }

                main_ctrl.download_manager.download_list(
                    &episodes, &download_path);
            },

            Message::Ui(UiMsg::Noop) => (),
        }
    }

    main_ctrl.tx_to_ui.send(MainMessage::UiTearDown).unwrap();
    main_ctrl.ui_thread.join().unwrap();  // wait for UI thread to finish teardown
}


/// Gets the path to the config file if one is specified in the command-
/// line arguments, or else returns the default config path for the
/// user's operating system.
/// Returns None if default OS config directory cannot be determined.
/// 
/// Note: Right now we only have one possible command-line argument,
/// specifying a config path. If the command-line API is
/// extended in the future, this will have to be refactored.
fn get_config_path(args: &[String]) -> Option<PathBuf> {
    return match args.len() {
        3 => Some(PathBuf::from(&args[2])),
        _ => {
            let default_config = dirs::config_dir();
            match default_config {
                Some(mut path) => {
                    path.push("shellcaster");
                    path.push("config.toml");
                    Some(path)
                },
                None => None,
            } 
        },
    };
}


/// Main application controller, holding all of the main application
/// state and mechanisms for communicatingg with the rest of the app.
struct MainController {
    config: Config,
    db: Database,
    download_manager: downloads::DownloadManager,
    podcasts: LockVec<Podcast>,
    ui_thread: std::thread::JoinHandle<()>,
    tx_to_ui: mpsc::Sender<MainMessage>,
    tx_to_main: mpsc::Sender<Message>,
    rx_to_main: mpsc::Receiver<Message>,
}

impl MainController {
    /// Instantiates the main controller (used during app startup), which
    /// sets up the connection to the database, download manager, and UI
    /// thread, and reads the list of podcasts from the database.
    fn new(config: Config) -> MainController {
        // create transmitters and receivers for passing messages between threads
        let (tx_to_ui, rx_from_main) = mpsc::channel();
        let (tx_to_main, rx_to_main) = mpsc::channel();
        
        // get connection to the database
        let db_inst = Database::connect(&config.config_path);
    
        // set up download manager
        let tx_dl_to_main = tx_to_main.clone();
        let download_manager = downloads::DownloadManager::new(
            config.simultaneous_downloads,
            tx_dl_to_main);
    
        // create vector of podcasts, where references are checked at
        // runtime; this is necessary because we want main.rs to hold the
        // "ground truth" list of podcasts, and it must be mutable, but
        // UI needs to check this list and update the screen when
        // necessary
        let podcast_list = LockVec::new(db_inst.get_podcasts());
    
        // set up UI in new thread
        let tx_ui_to_main = mpsc::Sender::clone(&tx_to_main);
        let ui_thread = UI::spawn(config.clone(), podcast_list.clone(), rx_from_main, tx_ui_to_main);
            // TODO: Can we do this without cloning the config?

        return MainController {
            config: config,
            db: db_inst,
            download_manager: download_manager,
            podcasts: podcast_list,
            ui_thread: ui_thread,
            tx_to_ui: tx_to_ui,
            tx_to_main: tx_to_main,
            rx_to_main: rx_to_main,
        };
    }

    /// Sends the specified message to the UI, which will display at
    /// the bottom of the screen.
    fn msg_to_ui(&self, message: String) {
        self.tx_to_ui.send(MainMessage::UiSpawnMsgWin(
            message, MESSAGE_TIME)).unwrap();
    }

    /// Handles the application logic for adding a new podcast, or
    /// synchronizing data from the RSS feed of an existing podcast.
    fn add_or_sync_data(&self, pod: Podcast, update: bool) {
        let title = pod.title.clone();
        let db_result;
        let failure;

        if update {
            db_result = self.db.update_podcast(pod);
            failure = format!("Error synchronizing {}.", title);
        } else {
            db_result = self.db.insert_podcast(pod);
            failure = "Error adding podcast to database.".to_string();
        }
        match db_result {
            Ok(num_ep) => {
                *self.podcasts.borrow() = self.db.get_podcasts();
                self.tx_to_ui.send(MainMessage::UiUpdateMenus).unwrap();

                if update {
                    self.msg_to_ui(format!("Synchronized {}.", title));
                } else {
                    self.msg_to_ui(format!("Successfully added {} episodes.", num_ep));
                }
            },
            Err(_err) => self.msg_to_ui(failure),
        }
    }

    /// Attempts to execute the play command on the given podcast
    /// episode.
    fn play_file(&self, pod_index: i32, ep_index: i32) {
        let episode = self.podcasts.clone_episode(
            pod_index, ep_index).unwrap();

        match episode.path {
            // if there is a local file, try to play that
            Some(path) => {
                match path.to_str() {
                    Some(p) => {
                        if play_file::execute(&self.config.play_command, &p).is_err() {
                            self.msg_to_ui(
                                "Error: Could not play file. Check configuration.".to_string());
                        }
                    },
                    None => self.msg_to_ui(
                        "Error: Filepath is not valid Unicode.".to_string()),
                }
            },
            // otherwise, try to stream the URL
            None => {
                if play_file::execute(&self.config.play_command, &episode.url).is_err() {
                    self.msg_to_ui(
                        "Error: Could not stream URL.".to_string());
                }
            }
        }
    }
}