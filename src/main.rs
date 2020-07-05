use std::process;
use std::path::PathBuf;

mod main_controller;
mod config;
mod keymap;
mod db;
mod ui;
mod types;
mod threadpool;
mod feeds;
mod downloads;
mod play_file;

use crate::main_controller::{MainController, MainMessage};
use crate::types::*;
use crate::config::Config;
use crate::ui::UiMsg;
use crate::feeds::FeedMsg;
use crate::downloads::DownloadMsg;

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
#[allow(clippy::while_let_on_iterator)]
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

    let mut db_path = config_path;
    if !db_path.pop() {
        println!("Could not correctly parse the config file location. Please specify a valid path to the config file.");
        process::exit(1);
    }

    let main_ctrl = MainController::new(config, &db_path);


    // MAIN LOOP --------------------------------------------------------

    // wait for messages from the UI and other threads, and then process
    let mut message_iter = main_ctrl.rx_to_main.iter();
    while let Some(message) = message_iter.next() {
        match message {
            Message::Ui(UiMsg::Quit) => break,

            Message::Ui(UiMsg::AddFeed(url)) =>
                main_ctrl.add_podcast(url),

            Message::Feed(FeedMsg::NewData(pod)) =>
                main_ctrl.add_or_sync_data(pod, false),

            Message::Feed(FeedMsg::Error) =>
                main_ctrl.msg_to_ui("Error retrieving RSS feed.".to_string(), true),

            Message::Ui(UiMsg::Sync(pod_index)) =>
                main_ctrl.sync(Some(pod_index)),

            Message::Feed(FeedMsg::SyncData(pod)) =>
                main_ctrl.add_or_sync_data(pod, true),

            Message::Ui(UiMsg::SyncAll) =>
                main_ctrl.sync(None),

            Message::Ui(UiMsg::Play(pod_index, ep_index)) =>
                main_ctrl.play_file(pod_index, ep_index),

            Message::Ui(UiMsg::MarkPlayed(pod_index, ep_index, played)) =>
                main_ctrl.mark_played(pod_index, ep_index, played),

            Message::Ui(UiMsg::MarkAllPlayed(pod_index, played)) =>
                main_ctrl.mark_all_played(pod_index, played),

            Message::Ui(UiMsg::Download(pod_index, ep_index)) =>
                main_ctrl.download(pod_index, Some(ep_index)),

            Message::Ui(UiMsg::DownloadAll(pod_index)) =>
                main_ctrl.download(pod_index, None),

            // downloading can produce any one of these responses
            Message::Dl(DownloadMsg::Complete(ep_data)) =>
                main_ctrl.download_complete(ep_data),
            Message::Dl(DownloadMsg::ResponseError(_)) =>
                main_ctrl.msg_to_ui("Error sending download request.".to_string(), true),
            Message::Dl(DownloadMsg::FileCreateError(_)) =>
                main_ctrl.msg_to_ui("Error creating file.".to_string(), true),
            Message::Dl(DownloadMsg::FileWriteError(_)) =>
                main_ctrl.msg_to_ui("Error downloading episode.".to_string(), true),

            Message::Ui(UiMsg::Delete(pod_index, ep_index)) =>
                main_ctrl.delete_file(pod_index, ep_index),

            Message::Ui(UiMsg::DeleteAll(pod_index)) =>
                main_ctrl.delete_files(pod_index),

            Message::Ui(UiMsg::RemovePodcast(pod_index, delete_files)) =>
                main_ctrl.remove_podcast(pod_index, delete_files),

            Message::Ui(UiMsg::RemoveEpisode(pod_index, ep_index, delete_files)) =>
                main_ctrl.remove_episode(pod_index, ep_index, delete_files),

            Message::Ui(UiMsg::RemoveAllEpisodes(pod_index, delete_files)) =>
                main_ctrl.remove_all_episodes(pod_index, delete_files),
                    
            Message::Ui(UiMsg::Noop) => (),
        }
    }

    // CLEANUP ----------------------------------------------------------
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