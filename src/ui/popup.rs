use pancurses::Input;
use std::cmp::min;

use super::{ColorType, Colors};
use super::{Menu, Panel, UiMsg};
use crate::config::BIG_SCROLL_AMOUNT;
use crate::keymap::{Keybindings, UserAction};
use crate::types::*;

/// Enum indicating the type of the currently active popup window.
#[derive(Debug)]
pub enum ActivePopup {
    WelcomeWin(Panel),
    HelpWin(Panel),
    DownloadWin(Menu<NewEpisode>),
    None,
}

impl ActivePopup {
    pub fn is_welcome_win(&self) -> bool {
        return matches!(self, ActivePopup::WelcomeWin(_));
    }

    pub fn is_help_win(&self) -> bool {
        return matches!(self, ActivePopup::HelpWin(_));
    }

    pub fn is_download_win(&self) -> bool {
        return matches!(self, ActivePopup::DownloadWin(_));
    }

    pub fn is_none(&self) -> bool {
        return matches!(self, ActivePopup::None);
    }
}

/// Holds all state relevant for handling popup windows.
#[derive(Debug)]
pub struct PopupWin<'a> {
    popup: ActivePopup,
    new_episodes: Vec<NewEpisode>,
    colors: Colors,
    keymap: &'a Keybindings,
    total_rows: i32,
    total_cols: i32,
    pub welcome_win: bool,
    pub help_win: bool,
    pub download_win: bool,
}

impl<'a> PopupWin<'a> {
    /// Set up struct for handling popup windows.
    pub fn new(colors: Colors, keymap: &'a Keybindings, total_rows: i32, total_cols: i32) -> Self {
        return Self {
            popup: ActivePopup::None,
            new_episodes: Vec::new(),
            colors: colors,
            keymap: keymap,
            total_rows: total_rows,
            total_cols: total_cols,
            welcome_win: false,
            help_win: false,
            download_win: false,
        };
    }

    /// Indicates whether any sort of popup window is currently on the
    /// screen.
    pub fn is_popup_active(&self) -> bool {
        return self.welcome_win || self.help_win || self.download_win;
    }

    /// Indicates whether a popup window *other than the welcome window*
    /// is currently on the screen.
    pub fn is_non_welcome_popup_active(&self) -> bool {
        return self.help_win || self.download_win;
    }

    /// Resize the currently active popup window if one exists.
    pub fn resize(&mut self, total_rows: i32, total_cols: i32) {
        self.total_rows = total_rows;
        self.total_cols = total_cols;
        match &self.popup {
            ActivePopup::WelcomeWin(_win) => {
                let welcome_win = self.make_welcome_win();
                welcome_win.refresh();
                self.popup = ActivePopup::WelcomeWin(welcome_win);
            }
            ActivePopup::HelpWin(_win) => {
                let help_win = self.make_help_win();
                help_win.refresh();
                self.popup = ActivePopup::HelpWin(help_win);
            }
            ActivePopup::DownloadWin(_win) => {
                let mut download_win = self.make_download_win();
                download_win.highlight_selected(true);
                self.popup = ActivePopup::DownloadWin(download_win);
            }
            ActivePopup::None => (),
        }
        // if let Some(panel) = &self.panel {
        //     panel.refresh();
        // }
    }

    /// Create a welcome window and draw it to the screen.
    pub fn spawn_welcome_win(&mut self) {
        self.welcome_win = true;
        self.change_win();
    }

    /// Create a new Panel holding a welcome window.
    pub fn make_welcome_win(&self) -> Panel {
        // get list of all keybindings for adding a feed, quitting
        // program, or opening help menu
        let actions = vec![UserAction::AddFeed, UserAction::Quit, UserAction::Help];
        let mut key_strs = Vec::new();
        for action in actions {
            key_strs.push(self.list_keys(action, None));
        }

        // the warning on the unused mut is a function of Rust getting
        // confused between panel.rs and mock_panel.rs
        #[allow(unused_mut)]
        let mut welcome_win = Panel::new(
            self.colors.clone(),
            "Shellcaster".to_string(),
            0,
            self.total_rows - 1,
            self.total_cols,
            0,
            0,
        );

        let mut row = 0;
        row = welcome_win.write_wrap_line(row + 1, "Welcome to shellcaster!".to_string());

        row = welcome_win.write_wrap_line(row+2,
            format!("Your podcast list is currently empty. Press {} to add a new podcast feed, {} to quit, or see all available commands by typing {} to get help.", key_strs[0], key_strs[1], key_strs[2]));

        row = welcome_win.write_wrap_line(
            row + 2,
            "More details of how to customize shellcaster can be found on the Github repo readme:"
                .to_string(),
        );
        let _ = welcome_win.write_wrap_line(
            row + 1,
            "https://github.com/jeff-hughes/shellcaster".to_string(),
        );

        return welcome_win;
    }

    /// Create a new help window and draw it to the screen.
    pub fn spawn_help_win(&mut self) {
        self.help_win = true;
        self.change_win();
    }

    /// Create a new Panel holding a help window.
    pub fn make_help_win(&self) -> Panel {
        let big_scroll_up = format!("Up 1/{} page:", BIG_SCROLL_AMOUNT);
        let big_scroll_dn = format!("Down 1/{} page:", BIG_SCROLL_AMOUNT);
        let actions = vec![
            (Some(UserAction::Left), "Left:"),
            (Some(UserAction::Right), "Right:"),
            (Some(UserAction::Up), "Up:"),
            (Some(UserAction::Down), "Down:"),
            (Some(UserAction::BigUp), &big_scroll_up),
            (Some(UserAction::BigDown), &big_scroll_dn),
            (Some(UserAction::PageUp), "Page up:"),
            (Some(UserAction::PageDown), "Page down:"),
            (Some(UserAction::GoTop), "Go to top:"),
            (Some(UserAction::GoBot), "Go to bottom:"),
            // (None, ""),
            (Some(UserAction::AddFeed), "Add feed:"),
            (Some(UserAction::Sync), "Sync:"),
            (Some(UserAction::SyncAll), "Sync all:"),
            // (None, ""),
            (Some(UserAction::Play), "Play:"),
            (Some(UserAction::MarkPlayed), "Mark as played:"),
            (Some(UserAction::MarkAllPlayed), "Mark all as played:"),
            // (None, ""),
            (Some(UserAction::Download), "Download:"),
            (Some(UserAction::DownloadAll), "Download all:"),
            (Some(UserAction::Delete), "Delete file:"),
            (Some(UserAction::DeleteAll), "Delete all files:"),
            (Some(UserAction::Remove), "Remove from list:"),
            (Some(UserAction::RemoveAll), "Remove all from list:"),
            // (None, ""),
            (Some(UserAction::Help), "Help:"),
            (Some(UserAction::Quit), "Quit:"),
        ];
        let mut key_strs = Vec::new();
        for (action, action_str) in actions {
            match action {
                Some(action) => {
                    let keys = self.keymap.keys_for_action(action);
                    // longest prefix is 21 chars long
                    let key_str = match keys.len() {
                        0 => format!("{:>21} <missing>", action_str),
                        1 => format!("{:>21} \"{}\"", action_str, &keys[0]),
                        _ => format!("{:>21} \"{}\" or \"{}\"", action_str, &keys[0], &keys[1]),
                    };
                    key_strs.push(key_str);
                }
                None => key_strs.push(" ".to_string()),
            }
        }

        // the warning on the unused mut is a function of Rust getting
        // confused between panel.rs and mock_panel.rs
        #[allow(unused_mut)]
        let mut help_win = Panel::new(
            self.colors.clone(),
            "Help".to_string(),
            0,
            self.total_rows - 1,
            self.total_cols,
            0,
            0,
        );

        let mut row = 0;
        row = help_win.write_wrap_line(row + 1, "Available keybindings:".to_string());
        help_win.change_attr(row, 0, 22, pancurses::A_UNDERLINE, ColorType::Normal);
        row += 1;

        // check how long our strings are, and map to two columns
        // if possible; `col_spacing` is the space to leave in between
        // the two columns
        let longest_line = key_strs
            .iter()
            .map(|x| x.chars().count())
            .max()
            .expect("Could not parse keybindings.");
        let col_spacing = 5;
        let n_cols = if help_win.get_cols() > (longest_line * 2 + col_spacing) as i32 {
            2
        } else {
            1
        };
        let keys_per_row = key_strs.len() as i32 / n_cols;

        // write each line of keys -- the list will be presented "down"
        // rather than "across", but we print to the screen a line at a
        // time, so the offset jumps down in the list if we have more
        // than one column
        for i in 0..keys_per_row {
            let mut line = String::new();
            for j in 0..n_cols {
                let offset = j * keys_per_row;
                if let Some(val) = key_strs.get((i + offset) as usize) {
                    // apply `col_spacing` to the right side of the
                    // first column
                    let width = if n_cols > 1 && offset == 0 {
                        longest_line + col_spacing
                    } else {
                        longest_line
                    };
                    line += &format!("{:<width$}", val, width = width);
                }
            }
            help_win.write_line(row + 1, line);
            row += 1;
        }

        let _ = help_win.write_wrap_line(row + 2, "Press \"q\" to close this window.".to_string());
        return help_win;
    }

    /// Create a new download window and draw it to the screen.
    pub fn spawn_download_win(&mut self, episodes: Vec<NewEpisode>, selected: bool) {
        for mut ep in episodes {
            ep.selected = selected;
            self.new_episodes.push(ep);
        }
        self.download_win = true;
        self.change_win();
    }

    /// Create a new Panel holding a download window.
    pub fn make_download_win(&self) -> Menu<NewEpisode> {
        // the warning on the unused mut is a function of Rust getting
        // confused between panel.rs and mock_panel.rs
        #[allow(unused_mut)]
        let mut download_panel = Panel::new(
            self.colors.clone(),
            "New episodes".to_string(),
            0,
            self.total_rows - 1,
            self.total_cols,
            0,
            0,
        );

        let header = format!(
            "Select which episodes to download with {}. Select all/none with {}. Press {} to confirm the selection and exit the menu.",
            self.list_keys(UserAction::MarkPlayed, Some(2)),
            self.list_keys(UserAction::MarkAllPlayed, Some(2)),
            self.list_keys(UserAction::Quit, Some(2)));
        let mut download_win = Menu::new(
            download_panel,
            Some(header),
            LockVec::new(self.new_episodes.clone()),
        );
        download_win.init();

        return download_win;
    }

    pub fn _add_episodes(&mut self, mut episodes: Vec<NewEpisode>) {
        self.new_episodes.append(&mut episodes);
    }

    /// Gets rid of the welcome window.
    pub fn turn_off_welcome_win(&mut self) {
        self.welcome_win = false;
        self.change_win();
    }

    /// Gets rid of the help window.
    pub fn turn_off_help_win(&mut self) {
        self.help_win = false;
        self.change_win();
    }

    /// Gets rid of the download window.
    pub fn turn_off_download_win(&mut self) {
        self.download_win = false;
        self.change_win();
    }

    /// When there is a change to the active popup window, this should
    /// be called to check for other popup windows that are "in the
    /// queue" -- this lets one popup window appear over top of another
    /// one, while keeping that second one in reserve. This function will
    /// check for other popup windows to appear and change the active
    /// window accordingly.
    fn change_win(&mut self) {
        // The help window takes precedence over all other popup windows;
        // the welcome window is lowest priority and only appears if all
        // other windows are inactive
        if self.help_win && !self.popup.is_help_win() {
            let win = self.make_help_win();
            win.refresh();
            self.popup = ActivePopup::HelpWin(win);
        } else if self.download_win && !self.popup.is_download_win() {
            let mut win = self.make_download_win();
            win.update_items();
            win.highlight_selected(true);
            self.popup = ActivePopup::DownloadWin(win);
        } else if self.welcome_win && !self.popup.is_welcome_win() {
            let win = self.make_welcome_win();
            win.refresh();
            self.popup = ActivePopup::WelcomeWin(win);
        } else if !self.help_win && !self.download_win && !self.welcome_win && !self.popup.is_none()
        {
            self.popup = ActivePopup::None;
        }
    }

    /// When a popup window is active, this handles the user's keyboard
    /// input that is relevant for that window.
    pub fn handle_input(&mut self, input: Input) -> UiMsg {
        let mut msg = UiMsg::Noop;
        match self.popup {
            ActivePopup::HelpWin(ref mut _win) => {
                match input {
                    Input::KeyExit
                    | Input::Character('\u{1b}') // Esc
                    | Input::Character('q')
                    | Input::Character('Q') => {
                        self.turn_off_help_win();
                    }
                    _ => (),
                }
            }
            ActivePopup::DownloadWin(ref mut menu) => match self.keymap.get_from_input(input) {
                Some(UserAction::Down) => menu.scroll(1),
                Some(UserAction::Up) => menu.scroll(-1),

                Some(UserAction::MarkPlayed) => {
                    menu.select_item();
                }

                Some(UserAction::MarkAllPlayed) => {
                    menu.select_all_items();
                }

                Some(UserAction::Quit) => {
                    let mut eps_to_download = Vec::new();
                    {
                        let map = menu.items.borrow_map();
                        for (_, ep) in map.iter() {
                            if ep.selected {
                                eps_to_download.push((ep.pod_id, ep.id));
                            }
                        }
                    }
                    if !eps_to_download.is_empty() {
                        msg = UiMsg::DownloadMulti(eps_to_download);
                    }
                    self.turn_off_download_win();
                }

                Some(_) | None => (),
            },
            _ => (),
        }
        return msg;
    }


    /// Helper function that gets the keybindings for a particular
    /// user action, and converts it to a string, e.g., '"a", "b", or
    /// "c"'. If `max_num` is set, will only list up to that number of
    /// items.
    fn list_keys(&self, action: UserAction, max_num: Option<usize>) -> String {
        let keys = self.keymap.keys_for_action(action);
        let mut max_keys = keys.len();
        if let Some(max_num) = max_num {
            max_keys = min(keys.len(), max_num);
        }
        return match max_keys {
            0 => "<missing>".to_string(),
            1 => format!("\"{}\"", &keys[0]),
            2 => format!("\"{}\" or \"{}\"", &keys[0], &keys[1]),
            _ => {
                let mut s = "".to_string();
                for (i, key) in keys.iter().enumerate().take(max_keys) {
                    if i == max_keys - 1 {
                        s = format!("{}, \"{}\"", s, key);
                    } else {
                        s = format!("{}, or \"{}\"", s, key);
                    }
                }
                s
            }
        };
    }
}
