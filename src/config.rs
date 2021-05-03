use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::keymap::{Keybindings, UserAction};

// Specifies how long, in milliseconds, to display messages at the
// bottom of the screen in the UI.
pub const MESSAGE_TIME: u64 = 5000;

// How many columns we need, minimum, before we display the
// (unplayed/total) after the podcast title
pub const PODCAST_UNPLAYED_TOTALS_LENGTH: usize = 25;

// How many columns we need, minimum, before we display the duration of
// the episode
pub const EPISODE_DURATION_LENGTH: usize = 45;

// How many columns we need, minimum, before we display the pubdate
// of the episode
pub const EPISODE_PUBDATE_LENGTH: usize = 60;

// How many columns we need (total terminal window width) before we
// display the details panel
pub const DETAILS_PANEL_LENGTH: i32 = 135;

// How many lines will be scrolled by the big scroll,
// in relation to the rows eg: 4 = 1/4 of the screen
pub const BIG_SCROLL_AMOUNT: i32 = 4;


/// Identifies the user's selection for what to do with new episodes
/// when syncing.
#[derive(Debug, Clone)]
pub enum DownloadNewEpisodes {
    Always,
    AskSelected,
    AskUnselected,
    Never,
}

/// Holds information about user configuration of program.
#[derive(Debug, Clone)]
pub struct Config {
    pub download_path: PathBuf,
    pub play_command: String,
    pub download_new_episodes: DownloadNewEpisodes,
    pub simultaneous_downloads: usize,
    pub max_retries: usize,
    pub keybindings: Keybindings,
}

/// A temporary struct used to deserialize data from the TOML configuration
/// file. Will be converted into Config struct.
#[derive(Debug, Deserialize)]
struct ConfigFromToml {
    download_path: Option<String>,
    play_command: Option<String>,
    download_new_episodes: Option<String>,
    simultaneous_downloads: Option<usize>,
    max_retries: Option<usize>,
    keybindings: KeybindingsFromToml,
}

/// A temporary struct used to deserialize keybinding data from the TOML
/// configuration file.
#[derive(Debug, Deserialize)]
struct KeybindingsFromToml {
    left: Option<Vec<String>>,
    right: Option<Vec<String>>,
    up: Option<Vec<String>>,
    down: Option<Vec<String>>,
    big_up: Option<Vec<String>>,
    big_down: Option<Vec<String>>,
    go_top: Option<Vec<String>>,
    go_bot: Option<Vec<String>>,
    page_up: Option<Vec<String>>,
    page_down: Option<Vec<String>>,
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
    help: Option<Vec<String>>,
    quit: Option<Vec<String>>,
}


impl Config {
    /// Given a file path, this reads a TOML config file and returns a
    /// Config struct with keybindings, etc. Inserts defaults if config
    /// file does not exist, or if specific values are not set.
    pub fn new(path: &Path) -> Result<Config> {
        let mut config_string = String::new();
        let config_toml: ConfigFromToml;

        match File::open(path) {
            Ok(mut file) => {
                file.read_to_string(&mut config_string).with_context(|| {
                    "Could not read config.toml. Please ensure file is readable."
                })?;
                config_toml = toml::from_str(&config_string)
                    .with_context(|| "Could not parse config.toml. Please check file syntax.")?;
            }
            Err(_) => {
                // if we can't find the file, set everything to empty
                // so we it will use the defaults for everything
                let keybindings = KeybindingsFromToml {
                    left: None,
                    right: None,
                    up: None,
                    down: None,
                    big_up: None,
                    big_down: None,
                    go_top: None,
                    go_bot: None,
                    page_up: None,
                    page_down: None,
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
                    help: None,
                    quit: None,
                };
                config_toml = ConfigFromToml {
                    download_path: None,
                    play_command: None,
                    download_new_episodes: None,
                    simultaneous_downloads: None,
                    max_retries: None,
                    keybindings: keybindings,
                };
            }
        }

        return config_with_defaults(&config_toml);
    }
}

/// Takes the deserialized TOML configuration, and creates a Config struct
/// that specifies user settings where indicated, and defaults for any
/// settings that were not specified by the user.
#[allow(clippy::type_complexity)]
fn config_with_defaults(config_toml: &ConfigFromToml) -> Result<Config> {
    // specify all default keybindings for actions
    #[rustfmt::skip]
    let action_map: Vec<(&Option<Vec<String>>, UserAction, Vec<String>)> = vec![
        (&config_toml.keybindings.left, UserAction::Left, vec!["Left".to_string(), "h".to_string()]),
        (&config_toml.keybindings.right, UserAction::Right, vec!["Right".to_string(), "l".to_string()]),
        (&config_toml.keybindings.up, UserAction::Up, vec!["Up".to_string(), "k".to_string()]),
        (&config_toml.keybindings.down, UserAction::Down, vec!["Down".to_string(), "j".to_string()]),
        (&config_toml.keybindings.big_up, UserAction::BigUp, vec!["K".to_string()]),
        (&config_toml.keybindings.big_down, UserAction::BigDown, vec!["J".to_string()]),
        (&config_toml.keybindings.page_up, UserAction::PageUp, vec!["PgUp".to_string()]),
        (&config_toml.keybindings.page_down, UserAction::PageDown, vec!["PgDn".to_string()]),
        (&config_toml.keybindings.go_top, UserAction::GoTop, vec!["g".to_string()]),
        (&config_toml.keybindings.go_bot, UserAction::GoBot, vec!["G".to_string()]),

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

        (&config_toml.keybindings.help, UserAction::Help, vec!["?".to_string()]),
        (&config_toml.keybindings.quit, UserAction::Quit, vec!["q".to_string()]),
    ];

    // for each action, if user preference is set, use that, otherwise,
    // use the default
    let mut keymap = Keybindings::new();
    for (config, action, defaults) in action_map.iter() {
        match config {
            Some(v) => keymap.insert_from_vec(v, *action),
            None => keymap.insert_from_vec(&defaults, *action),
        }
    }

    // paths are set by user, or they resolve to OS-specific path as
    // provided by dirs crate
    let download_path =
        parse_create_dir(config_toml.download_path.as_deref(), dirs::data_local_dir())?;

    let play_command = match config_toml.play_command.as_deref() {
        Some(cmd) => cmd.to_string(),
        None => "vlc %s".to_string(),
    };

    let download_new_episodes = match config_toml.download_new_episodes.as_deref() {
        Some("always") => DownloadNewEpisodes::Always,
        Some("ask-selected") => DownloadNewEpisodes::AskSelected,
        Some("ask-unselected") => DownloadNewEpisodes::AskUnselected,
        Some("never") => DownloadNewEpisodes::Never,
        Some(_) | None => DownloadNewEpisodes::AskUnselected,
    };

    let simultaneous_downloads = match config_toml.simultaneous_downloads {
        Some(num) if num > 0 => num,
        Some(_) => 3,
        None => 3,
    };

    let max_retries = match config_toml.max_retries {
        Some(num) if num > 0 => num,
        Some(_) => 3,
        None => 3,
    };

    return Ok(Config {
        download_path: download_path,
        play_command: play_command,
        download_new_episodes: download_new_episodes,
        simultaneous_downloads: simultaneous_downloads,
        max_retries: max_retries,
        keybindings: keymap,
    });
}


/// Helper function that takes an (optionally specified) user directory
/// and an (OS-dependent) default directory, expands any environment
/// variables, ~ alias, etc. Returns a PathBuf. Panics if environment
/// variables cannot be found, if OS could not produce the appropriate
/// default directory, or if the specified directories in the path could
/// not be created.
fn parse_create_dir(user_dir: Option<&str>, default: Option<PathBuf>) -> Result<PathBuf> {
    let final_path = match user_dir {
        Some(path) => match shellexpand::full(path) {
            Ok(realpath) => PathBuf::from(realpath.as_ref()),
            Err(err) => {
                return Err(anyhow!(
                    "Could not parse environment variable {} in config.toml. Reason: {}",
                    err.var_name,
                    err.cause
                ))
            }
        },
        None => {
            if let Some(mut path) = default {
                path.push("shellcaster");
                path
            } else {
                return Err(anyhow!("Could not identify a default directory for your OS. Please specify paths manually in config.toml."));
            }
        }
    };

    // create directories if they do not exist
    std::fs::create_dir_all(&final_path).with_context(|| {
        format!(
            "Could not create filepath: {}",
            final_path.to_string_lossy()
        )
    })?;

    return Ok(final_path);
}
