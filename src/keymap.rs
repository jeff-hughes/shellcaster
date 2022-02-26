use ahash::AHashMap;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::KeybindingsFromToml;

/// Enum delineating all actions that may be performed by the user, and
/// thus have keybindings associated with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserAction {
    Left,
    Right,
    Up,
    Down,

    BigUp,
    BigDown,
    PageUp,
    PageDown,
    GoTop,
    GoBot,

    AddFeed,
    Sync,
    SyncAll,

    Play,
    MarkPlayed,
    MarkAllPlayed,

    Download,
    DownloadAll,
    Delete,
    DeleteAll,
    Remove,
    RemoveAll,

    FilterPlayed,
    FilterDownloaded,

    Help,
    Quit,
}

/// Wrapper around a hash map that keeps track of all keybindings. Multiple
/// keys may perform the same action, but each key may only perform one
/// action.
#[derive(Debug, Clone)]
pub struct Keybindings(AHashMap<String, UserAction>);

impl Keybindings {
    /// Returns a new Keybindings struct.
    pub fn new() -> Self {
        return Self(AHashMap::new());
    }

    /// Returns a Keybindings struct with all default values set.
    pub fn default() -> Self {
        let defaults = Self::_defaults();
        let mut keymap = Self::new();
        for (action, defaults) in defaults.into_iter() {
            keymap.insert_from_vec(defaults, action);
        }
        return keymap;
    }

    /// Given a struct deserialized from config.toml (for which any or
    /// all fields may be missing), create a Keybindings struct using
    /// user-defined keys where specified, and default values otherwise.
    pub fn from_config(config: KeybindingsFromToml) -> Self {
        let config_actions: Vec<(Option<Vec<String>>, UserAction)> = vec![
            (config.left, UserAction::Left),
            (config.right, UserAction::Right),
            (config.up, UserAction::Up),
            (config.down, UserAction::Down),
            (config.big_up, UserAction::BigUp),
            (config.big_down, UserAction::BigDown),
            (config.page_up, UserAction::PageUp),
            (config.page_down, UserAction::PageDown),
            (config.go_top, UserAction::GoTop),
            (config.go_bot, UserAction::GoBot),
            (config.add_feed, UserAction::AddFeed),
            (config.sync, UserAction::Sync),
            (config.sync_all, UserAction::SyncAll),
            (config.play, UserAction::Play),
            (config.mark_played, UserAction::MarkPlayed),
            (config.mark_all_played, UserAction::MarkAllPlayed),
            (config.download, UserAction::Download),
            (config.download_all, UserAction::DownloadAll),
            (config.delete, UserAction::Delete),
            (config.delete_all, UserAction::DeleteAll),
            (config.remove, UserAction::Remove),
            (config.remove_all, UserAction::RemoveAll),
            (config.filter_played, UserAction::FilterPlayed),
            (config.filter_downloaded, UserAction::FilterDownloaded),
            (config.help, UserAction::Help),
            (config.quit, UserAction::Quit),
        ];

        let mut keymap = Self::default();
        for (config, action) in config_actions.into_iter() {
            if let Some(config) = config {
                keymap.insert_from_vec(config, action);
            }
        }
        return keymap;
    }

    /// Takes an Input object from crossterm and returns the associated
    /// user action, if one exists.
    pub fn get_from_input(&self, input: KeyEvent) -> Option<&UserAction> {
        match input_to_str(input) {
            Some(code) => self.0.get(&code),
            None => None,
        }
    }

    /// Inserts a new keybinding into the hash map. Will overwrite the
    /// value of a key if it already exists.
    pub fn insert(&mut self, code: String, action: UserAction) {
        self.0.insert(code, action);
    }

    /// Inserts a set of new keybindings into the hash map, each one
    /// corresponding to the same UserAction. Will overwrite the value
    /// of keys that already exist.
    pub fn insert_from_vec(&mut self, vec: Vec<String>, action: UserAction) {
        for key in vec.into_iter() {
            self.insert(key, action);
        }
    }

    /// Returns a Vec with all of the keys mapped to a particular user
    /// action.
    pub fn keys_for_action(&self, action: UserAction) -> Vec<String> {
        return self
            .0
            .iter()
            .filter_map(|(key, &val)| {
                if val == action {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
    }

    fn _defaults() -> Vec<(UserAction, Vec<String>)> {
        return vec![
            (UserAction::Left, vec!["Left".to_string(), "h".to_string()]),
            (UserAction::Right, vec![
                "Right".to_string(),
                "l".to_string(),
            ]),
            (UserAction::Up, vec!["Up".to_string(), "k".to_string()]),
            (UserAction::Down, vec!["Down".to_string(), "j".to_string()]),
            (UserAction::BigUp, vec!["K".to_string()]),
            (UserAction::BigDown, vec!["J".to_string()]),
            (UserAction::PageUp, vec!["PgUp".to_string()]),
            (UserAction::PageDown, vec!["PgDn".to_string()]),
            (UserAction::GoTop, vec!["g".to_string()]),
            (UserAction::GoBot, vec!["G".to_string()]),
            (UserAction::AddFeed, vec!["a".to_string()]),
            (UserAction::Sync, vec!["s".to_string()]),
            (UserAction::SyncAll, vec!["S".to_string()]),
            (UserAction::Play, vec!["Enter".to_string(), "p".to_string()]),
            (UserAction::MarkPlayed, vec!["m".to_string()]),
            (UserAction::MarkAllPlayed, vec!["M".to_string()]),
            (UserAction::Download, vec!["d".to_string()]),
            (UserAction::DownloadAll, vec!["D".to_string()]),
            (UserAction::Delete, vec!["x".to_string()]),
            (UserAction::DeleteAll, vec!["X".to_string()]),
            (UserAction::Remove, vec!["r".to_string()]),
            (UserAction::RemoveAll, vec!["R".to_string()]),
            (UserAction::FilterPlayed, vec!["1".to_string()]),
            (UserAction::FilterDownloaded, vec!["2".to_string()]),
            (UserAction::Help, vec!["?".to_string()]),
            (UserAction::Quit, vec!["q".to_string()]),
        ];
    }
}

/// Helper function converting a crossterm KeyEvent object to a unique
/// string representing that input.
pub fn input_to_str(input: KeyEvent) -> Option<String> {
    let ctrl = if input.modifiers.intersects(KeyModifiers::CONTROL) {
        "Ctrl+"
    } else {
        ""
    };
    let alt = if input.modifiers.intersects(KeyModifiers::ALT) {
        "Alt+"
    } else {
        ""
    };
    let shift = if input.modifiers.intersects(KeyModifiers::SHIFT) {
        "Shift+"
    } else {
        ""
    };
    let mut tmp = [0; 4];
    return match input.code {
        KeyCode::Backspace => Some(format!("{ctrl}{alt}{shift}Backspace")),
        KeyCode::Enter => Some(format!("{ctrl}{alt}{shift}Enter")),
        KeyCode::Left => Some(format!("{ctrl}{alt}{shift}Left")),
        KeyCode::Right => Some(format!("{ctrl}{alt}{shift}Right")),
        KeyCode::Up => Some(format!("{ctrl}{alt}{shift}Up")),
        KeyCode::Down => Some(format!("{ctrl}{alt}{shift}Down")),
        KeyCode::Home => Some(format!("{ctrl}{alt}{shift}Home")),
        KeyCode::End => Some(format!("{ctrl}{alt}{shift}End")),
        KeyCode::PageUp => Some(format!("{ctrl}{alt}{shift}PgUp")),
        KeyCode::PageDown => Some(format!("{ctrl}{alt}{shift}PgDn")),
        KeyCode::Tab => Some(format!("{ctrl}{alt}{shift}Tab")),
        KeyCode::BackTab => Some(format!("{ctrl}{alt}{shift}Tab")),
        KeyCode::Delete => Some(format!("{ctrl}{alt}{shift}Del")),
        KeyCode::Insert => Some(format!("{ctrl}{alt}{shift}Ins")),
        KeyCode::Esc => Some(format!("{ctrl}{alt}{shift}Esc")),
        KeyCode::F(num) => Some(format!("{ctrl}{alt}{shift}F{num}")), // Function keys
        KeyCode::Char(c) => {
            if c == '\u{7f}' {
                Some(format!("{ctrl}{alt}{shift}Backspace"))
            } else if c == '\u{1b}' {
                Some(format!("{ctrl}{alt}{shift}Esc"))
            } else if c == '\n' {
                Some(format!("{ctrl}{alt}{shift}Enter"))
            } else if c == '\t' {
                Some(format!("{ctrl}{alt}{shift}Tab"))
            } else {
                // here we don't include "shift" because that will
                // already be encoded in the character itself
                Some(format!("{}{}{}", ctrl, alt, c.encode_utf8(&mut tmp)))
            }
        }
        _ => None,
    };
}
