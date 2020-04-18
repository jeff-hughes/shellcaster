use std::fs::File;
use std::io::Read;
use serde::Deserialize;

use crate::keymap::{Keybindings, UserAction};

/// Holds information about user configuration of program. 
#[derive(Debug)]
pub struct Config {
    pub keybindings: Keybindings,
}

/// A temporary struct used to deserialize data from the TOML configuration
/// file. Will be converted into Config struct.
#[derive(Debug)]
#[derive(Deserialize)]
struct ConfigFromToml {
    keybindings: KeybindingsFromToml,
}

/// A temporary struct used to deserialize keybinding data from the TOML
/// configuration file.
#[derive(Debug)]
#[derive(Deserialize)]
struct KeybindingsFromToml {
    left: Option<Vec<String>>,
    right: Option<Vec<String>>,
    up: Option<Vec<String>>,
    down: Option<Vec<String>>,
    add_feed: Option<Vec<String>>,
    sync: Option<Vec<String>>,
    sync_all: Option<Vec<String>>,
    play: Option<Vec<String>>,
    mark_played: Option<Vec<String>>,
    mark_all_played: Option<Vec<String>>,
    download: Option<Vec<String>>,
    download_all: Option<Vec<String>>,
    delete: Option<Vec<String>>,
    delete_all: Option<Vec<String>>,
    remove: Option<Vec<String>>,
    remove_all: Option<Vec<String>>,
    search: Option<Vec<String>>,
    quit: Option<Vec<String>>,
}


/// Given a file path, this reads a TOML config file and returns a Config
/// struct with keybindings, etc. Inserts defaults if config file does
/// not exist, or if specific values are not set.
pub fn parse_config_file(path: &str) -> Config {
    let mut config_string = String::new();
    let config_toml: ConfigFromToml;

    match File::open(&path) {
        Ok(mut file) => {
            file.read_to_string(&mut config_string)
                .expect("Error reading config.toml. Please ensure file is readable.");
            config_toml = toml::from_str(&config_string)
                .expect("Error parsing config.toml. Please check file syntax.");
        },
        Err(_) => {
            // if we can't find the file, set everything to empty
            // so we it will use the defaults for everything
            let keybindings = KeybindingsFromToml {
                left: None,
                right: None,
                up: None,
                down: None,
                add_feed: None,
                sync: None,
                sync_all: None,
                play: None,
                mark_played: None,
                mark_all_played: None,
                download: None,
                download_all: None,
                delete: None,
                delete_all: None,
                remove: None,
                remove_all: None,
                search: None,
                quit: None,
            };
            config_toml = ConfigFromToml {
                keybindings: keybindings,
            };
        }
    }

    return set_keymap(&config_toml);
}

/// Takes the deserialized TOML configuration, and creates a Config struct
/// that specifies user settings where indicated, and defaults for any
/// settings that were not specified by the user.
fn set_keymap(config_toml: &ConfigFromToml) -> Config {
    let action_map: Vec<(&Option<Vec<String>>, UserAction, Vec<String>)> = vec![
        (&config_toml.keybindings.left, UserAction::Left, vec!["Left".to_string(), "h".to_string()]),
        (&config_toml.keybindings.right, UserAction::Right, vec!["Right".to_string(), "l".to_string()]),
        (&config_toml.keybindings.up, UserAction::Up, vec!["Up".to_string(), "k".to_string()]),
        (&config_toml.keybindings.down, UserAction::Down, vec!["Down".to_string(), "j".to_string()]),

        (&config_toml.keybindings.add_feed, UserAction::AddFeed, vec!["a".to_string()]),
        (&config_toml.keybindings.sync, UserAction::Sync, vec!["s".to_string()]),
        (&config_toml.keybindings.sync_all, UserAction::SyncAll, vec!["S".to_string()]),

        (&config_toml.keybindings.play, UserAction::Play, vec!["Enter".to_string(), "p".to_string()]),
        (&config_toml.keybindings.mark_played, UserAction::MarkPlayed, vec!["m".to_string()]),
        (&config_toml.keybindings.mark_all_played, UserAction::MarkAllPlayed, vec!["M".to_string()]),

        (&config_toml.keybindings.download, UserAction::Download, vec!["d".to_string()]),
        (&config_toml.keybindings.download_all, UserAction::DownloadAll, vec!["D".to_string()]),
        (&config_toml.keybindings.delete, UserAction::Delete, vec!["x".to_string()]),
        (&config_toml.keybindings.delete_all, UserAction::DeleteAll, vec!["X".to_string()]),
        (&config_toml.keybindings.remove, UserAction::Remove, vec!["r".to_string()]),
        (&config_toml.keybindings.remove_all, UserAction::RemoveAll, vec!["R".to_string()]),

        (&config_toml.keybindings.search, UserAction::Search, vec!["/".to_string()]),
        (&config_toml.keybindings.quit, UserAction::Quit, vec!["q".to_string()]),
    ];

    let mut keymap = Keybindings::new();
    for (config, action, defaults) in action_map.iter() {
        match config {
            Some(v) => keymap.insert_from_vec(v, *action),
            None => keymap.insert_from_vec(&defaults, *action),
        }
    }

    return Config {
        keybindings: keymap,
    };
}