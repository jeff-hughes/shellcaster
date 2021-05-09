use pancurses::Input;
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
    pub fn get_from_input(&self, input: Input) -> Option<&UserAction> {
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
pub fn input_to_str(input: Input) -> Option<String> {
    let mut tmp = [0; 4];
    let code = match input {
        Input::KeyCodeYes => "CodeYes",
        Input::KeyBreak => "Break",
        Input::KeyDown => "Down",
        Input::KeyUp => "Up",
        Input::KeyLeft => "Left",
        Input::KeyRight => "Right",
        Input::KeyHome => "Home",
        Input::KeyBackspace => "Backspace",
        Input::KeyF0 => "F0",
        Input::KeyF1 => "F1",
        Input::KeyF2 => "F2",
        Input::KeyF3 => "F3",
        Input::KeyF4 => "F4",
        Input::KeyF5 => "F5",
        Input::KeyF6 => "F6",
        Input::KeyF7 => "F7",
        Input::KeyF8 => "F8",
        Input::KeyF9 => "F9",
        Input::KeyF10 => "F10",
        Input::KeyF11 => "F11", // F11 triggers KeyResize for me
        Input::KeyF12 => "F12",
        Input::KeyF13 => "F13",
        Input::KeyF14 => "F14",
        Input::KeyF15 => "F15",
        Input::KeyDL => "DL",
        Input::KeyIL => "IL",
        Input::KeyDC => "Del",
        Input::KeyIC => "Ins",
        Input::KeyEIC => "EIC",
        Input::KeyClear => "Clear",
        Input::KeyEOS => "EOS",
        Input::KeyEOL => "EOL",
        Input::KeySF => "S_Down",
        Input::KeySR => "S_Up",
        Input::KeyNPage => "PgDn",
        Input::KeyPPage => "PgUp",
        Input::KeySTab => "STab", // this doesn't appear to be Shift+Tab
        Input::KeyCTab => "C_Tab",
        Input::KeyCATab => "CATab",
        Input::KeyEnter => "Enter",
        Input::KeySReset => "SReset",
        Input::KeyReset => "Reset",
        Input::KeyPrint => "Print",
        Input::KeyLL => "LL",
        Input::KeyAbort => "Abort",
        Input::KeySHelp => "SHelp",
        Input::KeyLHelp => "LHelp",
        Input::KeyBTab => "S_Tab", // Shift+Tab
        Input::KeyBeg => "Beg",
        Input::KeyCancel => "Cancel",
        Input::KeyClose => "Close",
        Input::KeyCommand => "Command",
        Input::KeyCopy => "Copy",
        Input::KeyEnd => "End",
        Input::KeyExit => "Exit",
        Input::KeyFind => "Find",
        Input::KeyHelp => "Help",
        Input::KeyMark => "Mark",
        Input::KeyMessage => "Message",
        Input::KeyMove => "Move",
        Input::KeyNext => "Next",
        Input::KeyOpen => "Open",
        Input::KeyOptions => "Options",
        Input::KeyPrevious => "Previous",
        Input::KeyRedo => "Redo",
        Input::KeyReference => "Reference",
        Input::KeyRefresh => "Refresh",
        Input::KeyResume => "Resume",
        Input::KeyRestart => "Restart",
        Input::KeySave => "Save",
        Input::KeySBeg => "S_Beg",
        Input::KeySCancel => "S_Cancel",
        Input::KeySCommand => "S_Command",
        Input::KeySCopy => "S_Copy",
        Input::KeySCreate => "S_Create",
        Input::KeySDC => "S_Del",
        Input::KeySDL => "S_DL",
        Input::KeySelect => "Select",
        Input::KeySEnd => "S_End",
        Input::KeySEOL => "S_EOL",
        Input::KeySExit => "S_Exit",
        Input::KeySFind => "S_Find",
        Input::KeySHome => "S_Home",
        Input::KeySIC => "S_Ins",
        Input::KeySLeft => "S_Left",
        Input::KeySMessage => "S_Message",
        Input::KeySMove => "S_Move",
        Input::KeySNext => "S_PgDn",
        Input::KeySOptions => "S_Options",
        Input::KeySPrevious => "S_PgUp",
        Input::KeySPrint => "S_Print",
        Input::KeySRedo => "S_Redo",
        Input::KeySReplace => "S_Replace",
        Input::KeySRight => "S_Right",
        Input::KeySResume => "S_Resume",
        Input::KeySSave => "S_Save",
        Input::KeySSuspend => "S_Suspend",
        Input::KeySUndo => "S_Undo",
        Input::KeySuspend => "Suspend",
        Input::KeyUndo => "Undo",
        Input::KeyResize => "F11", // I'm marking this as F11 as well
        Input::KeyEvent => "Event",
        Input::KeyMouse => "Mouse",
        Input::KeyA1 => "A1",
        Input::KeyA3 => "A3",
        Input::KeyB2 => "B2",
        Input::KeyC1 => "C1",
        Input::KeyC3 => "C3",
        Input::Character(c) => {
            if c == '\u{7f}' {
                "Backspace"
            } else if c == '\u{1b}' {
                "Escape"
            } else if c == '\n' {
                "Enter"
            } else if c == '\t' {
                "Tab"
            } else {
                c.encode_utf8(&mut tmp)
            }
        }
        _ => "",
    };
    if code.is_empty() {
        return None;
    } else {
        return Some(code.to_string());
    }
}
