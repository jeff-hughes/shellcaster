use crossterm::event::KeyCode;
use std::collections::HashMap;

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

    Help,
    Quit,
}

/// Wrapper around a hash map that keeps track of all keybindings. Multiple
/// keys may perform the same action, but each key may only perform one
/// action.
#[derive(Debug, Clone)]
pub struct Keybindings(HashMap<String, UserAction>);

impl Keybindings {
    /// Returns a new Keybindings struct.
    pub fn new() -> Self {
        return Self(HashMap::new());
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
        let defaults = Self::_defaults();
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
            (config.help, UserAction::Help),
            (config.quit, UserAction::Quit),
        ];

        let mut keymap = Self::new();
        for (config, action) in config_actions.into_iter() {
            keymap.insert_from_vec(
                config.unwrap_or_else(|| defaults.get(&action).unwrap().clone()),
                action,
            );
        }
        return keymap;
    }

    /// Takes an Input object from pancurses and returns the associated
    /// user action, if one exists.
    pub fn get_from_input(&self, input: KeyCode) -> Option<&UserAction> {
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

    fn _defaults() -> HashMap<UserAction, Vec<String>> {
        let action_map: Vec<(UserAction, Vec<String>)> = vec![
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
            (UserAction::Help, vec!["?".to_string()]),
            (UserAction::Quit, vec!["q".to_string()]),
        ];
        let mut default_map = HashMap::new();
        for (action, defaults) in action_map.into_iter() {
            default_map.insert(action, defaults);
        }
        return default_map;
    }
}

/// Helper function converting a pancurses Input object to a unique
/// string representing that input.
/// This function is a bit ridiculous, given that 95% of keyboards
/// probably don't even have half these special keys, but at any rate...
/// they're mapped, if anyone wants them.
pub fn input_to_str(input: KeyCode) -> Option<String> {
    let mut tmp = [0; 4];
    let code = match input {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "S_Tab".to_string(),
        KeyCode::Delete => "Del".to_string(),
        KeyCode::Insert => "Ins".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::F(num) => format!("F{}", num), // Function keys
        KeyCode::Char(c) => {
            if c == '\u{7f}' {
                "Backspace".to_string()
            } else if c == '\u{1b}' {
                "Escape".to_string()
            } else if c == '\n' {
                "Enter".to_string()
            } else if c == '\t' {
                "Tab".to_string()
            } else {
                c.encode_utf8(&mut tmp).to_string()
            }
        }
        _ => "".to_string(),
    };
    if code.is_empty() {
        return None;
    } else {
        return Some(code);
    }
}
