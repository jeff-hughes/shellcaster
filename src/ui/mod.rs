use std::thread;
use std::sync::mpsc;
use std::time::Duration;

#[cfg_attr(not(test), path="panel.rs")]
#[cfg_attr(test, path="mock_panel.rs")]
mod panel;

mod menu;
mod colors;

use self::panel::{Panel, Details};
use self::menu::Menu;
use self::colors::{Colors, ColorType};

use pancurses::{Window, newwin, Input};
use lazy_static::lazy_static;
use regex::Regex;

use crate::config::Config;
use crate::keymap::{Keybindings, UserAction};
use crate::types::*;
use super::MainMessage;

lazy_static! {
    /// Regex for finding HTML tags
    static ref RE_HTML_TAGS: Regex = Regex::new(r"<[^<>]*>").unwrap();

    /// Regex for finding more than two line breaks
    static ref RE_MULT_LINE_BREAKS: Regex = Regex::new(r"((\r\n)|\r|\n){3,}").unwrap();
}


/// Enum used for communicating back to the main controller after user
/// input has been captured by the UI. usize values always represent the
/// selected podcast, and (if applicable), the selected episode, in that
/// order.
#[derive(Debug)]
pub enum UiMsg {
    AddFeed(String),
    Play(usize, usize),
    MarkPlayed(usize, usize, bool),
    MarkAllPlayed(usize, bool),
    Sync(usize),
    SyncAll,
    Download(usize, usize),
    DownloadAll(usize),
    Delete(usize, usize),
    DeleteAll(usize),
    RemovePodcast(usize, bool),
    RemoveEpisode(usize, usize, bool),
    RemoveAllEpisodes(usize, bool),
    Quit,
    Noop,
}

/// Simple enum to identify which menu is currently active.
#[derive(Debug)]
enum ActiveMenu {
    PodcastMenu,
    EpisodeMenu,
}


/// Struct containing all interface elements of the TUI. Functionally, it
/// encapsulates the pancurses windows, and holds data about the size of
/// the screen.
#[derive(Debug)]
pub struct UI<'a> {
    stdscr: Window,
    n_row: i32,
    n_col: i32,
    keymap: &'a Keybindings,
    colors: Colors,
    podcast_menu: Menu<Podcast>,
    episode_menu: Menu<Episode>,
    active_menu: ActiveMenu,
    details_panel: Option<Panel>,
    welcome_win: Option<Panel>,
}

impl<'a> UI<'a> {
    /// Spawns a UI object in a new thread, with message channels to send
    /// and receive messages
    pub fn spawn(config: Config, items: LockVec<Podcast>, rx_from_main: mpsc::Receiver<MainMessage>, tx_to_main: mpsc::Sender<Message>) -> thread::JoinHandle<()> {
        return thread::spawn(move || {
            let mut ui = UI::new(&config, &items);
            ui.init();
            let mut message_iter = rx_from_main.try_iter();
            // on each loop, we check for user input, then we process
            // any messages from the main thread
            loop {
                match ui.getch() {
                    UiMsg::Noop => (),
                    input => tx_to_main.send(Message::Ui(input)).unwrap(),
                }

                if let Some(message) = message_iter.next() {
                    match message {
                        MainMessage::UiUpdateMenus => ui.update_menus(),
                        MainMessage::UiSpawnMsgWin(msg, duration, error) => ui.spawn_msg_win(msg, duration, error),
                        MainMessage::UiTearDown => {
                            ui.tear_down();
                            break;
                        }
                    }
                }

                // slight delay to avoid excessive CPU usage
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    /// Initializes the UI with a list of podcasts and podcast episodes,
    /// creates the pancurses window and draws it to the screen, and
    /// returns a UI object for future manipulation.
    pub fn new(config: &'a Config, items: &LockVec<Podcast>) -> UI<'a> {
        let stdscr = pancurses::initscr();

        // set some options
        pancurses::cbreak();  // allows characters to be read one by one
        pancurses::noecho();  // turns off automatic echoing of characters
                              // to the screen as they are input
        pancurses::start_color();  // allows colours if available
        pancurses::curs_set(0);  // turn off cursor
        stdscr.keypad(true);  // returns special characters as single
                              // key codes
        stdscr.nodelay(true);  // getch() will not wait for user input

        // set colors
        let colors = self::colors::set_colors();

        let (n_row, n_col) = stdscr.get_max_yx();
        let (pod_col, ep_col, det_col) = Self::calculate_sizes(n_col);

        let podcast_panel = Panel::new(
            colors.clone(),
            "Podcasts".to_string(),
            0,
            n_row - 1, pod_col,
            0, 0
        );
        let podcast_menu = Menu {
            panel: podcast_panel,
            items: items.clone(),
            top_row: 0,
            selected: 0,
        };

        let episode_panel = Panel::new(
            colors.clone(),
            "Episodes".to_string(),
            1,
            n_row - 1, ep_col,
            0, pod_col - 1
        );
        let first_pod: LockVec<Episode> = match items.borrow().get(0) {
            Some(pod) => pod.episodes.clone(),
            None => LockVec::new(Vec::new()),
        };
        let episode_menu = Menu {
            panel: episode_panel,
            items: first_pod,
            top_row: 0,
            selected: 0,
        };

        let details_panel = if n_col > crate::config::DETAILS_PANEL_LENGTH {
            Some(Self::make_details_panel(
                colors.clone(),
                n_row-1, det_col,
                0, pod_col + ep_col - 2))
        } else {
            None
        };

        // welcome screen if user does not have any podcasts yet
        let welcome_win = if items.borrow().is_empty() {
            Some(UI::make_welcome_win(colors.clone(), &config.keybindings, n_row-1, n_col))
        } else {
            None
        };

        return UI {
            stdscr,
            n_row,
            n_col,
            keymap: &config.keybindings,
            colors: colors,
            podcast_menu: podcast_menu,
            episode_menu: episode_menu,
            active_menu: ActiveMenu::PodcastMenu,
            details_panel: details_panel,
            welcome_win: welcome_win,
        };
    }

    /// This should be called immediately after creating the UI, in order
    /// to draw everything to the screen.
    pub fn init(&mut self) {
        self.stdscr.refresh();
        self.podcast_menu.init();
        self.podcast_menu.activate();
        self.episode_menu.init();
        self.update_details_panel();

        if self.welcome_win.is_some() {
            let ww = self.welcome_win.as_mut().unwrap();
            ww.refresh();
        }
    }

    /// Waits for user input and, where necessary, provides UiMessages
    /// back to the main controller.
    /// 
    /// Anything UI-related (e.g., scrolling up and down menus) is handled
    /// internally, producing an empty UiMessage. This allows for some
    /// greater degree of abstraction; for example, input to add a new
    /// podcast feed spawns a UI window to capture the feed URL, and only
    /// then passes this data back to the main controller.
    pub fn getch(&mut self) -> UiMsg {
        match self.stdscr.getch() {
            Some(Input::KeyResize) => {
                pancurses::resize_term(0, 0);
                let (n_row, n_col) = self.stdscr.get_max_yx();
                self.n_row = n_row;
                self.n_col = n_col;

                let (pod_col, ep_col, det_col) = Self::calculate_sizes(n_col);

                self.podcast_menu.resize(n_row-1, pod_col, 0, 0);
                self.episode_menu.resize(n_row-1, ep_col, 0, pod_col - 1);

                if self.details_panel.is_some() {
                    if det_col > 0 {
                        let det = self.details_panel.as_mut().unwrap();
                        det.resize(n_row-1, det_col, 0, pod_col+ep_col-2);
                    } else {
                        self.details_panel = None;
                    }
                } else if det_col > 0 {
                    self.details_panel = Some(Self::make_details_panel(
                        self.colors.clone(),
                        n_row-1, det_col,
                        0, pod_col + ep_col - 2));
                }

                self.stdscr.refresh();
                self.update_menus();
                
                match self.active_menu {
                    ActiveMenu::PodcastMenu => self.podcast_menu.activate(),
                    ActiveMenu::EpisodeMenu => {
                        self.podcast_menu.activate();
                        self.episode_menu.activate();
                    },
                }

                if self.details_panel.is_some() {
                    self.update_details_panel();
                }

                // resize welcome window, if it exists
                if self.welcome_win.is_some() {
                    let _ = std::mem::replace(
                        &mut self.welcome_win,
                        Some(UI::make_welcome_win(self.colors.clone(), &self.keymap, n_row-1, n_col)));
                    
                    let ww = self.welcome_win.as_mut().unwrap();
                    ww.refresh();
                }
                self.stdscr.refresh();
            },

            Some(input) => {
                let pod_len = self.podcast_menu.items.borrow().len();
                let ep_len = self.episode_menu.items.borrow().len();
                let current_pod_index = (self.podcast_menu.selected +
                    self.podcast_menu.top_row) as usize;
                let current_ep_index = (self.episode_menu.selected +
                    self.episode_menu.top_row) as usize;

                // get rid of the "welcome" window once the podcast list
                // is no longer empty
                if self.welcome_win.is_some() && pod_len > 0 {
                    self.welcome_win = None;
                }

                match self.keymap.get_from_input(input) {
                    Some(UserAction::Down) => {
                        match self.active_menu {
                            ActiveMenu::PodcastMenu => {
                                if pod_len > 0 {
                                    self.podcast_menu.scroll(1);

                                    self.episode_menu.top_row = 0;
                                    self.episode_menu.selected = 0;

                                    // update episodes menu with new list
                                    self.episode_menu.items = self.podcast_menu.get_episodes();
                                    self.episode_menu.update_items();
                                    self.update_details_panel();
                                }
                            },
                            ActiveMenu::EpisodeMenu => {
                                if ep_len > 0 {
                                    self.episode_menu.scroll(1);
                                    self.update_details_panel();
                                }
                            },
                        }
                    },

                    Some(UserAction::Up) => {
                        match self.active_menu {
                            ActiveMenu::PodcastMenu => {
                                if pod_len > 0 {
                                    self.podcast_menu.scroll(-1);

                                    self.episode_menu.top_row = 0;
                                    self.episode_menu.selected = 0;

                                    // update episodes menu with new list
                                    self.episode_menu.items = self.podcast_menu.get_episodes();
                                    self.episode_menu.update_items();
                                    self.update_details_panel();
                                }
                            },
                            ActiveMenu::EpisodeMenu => {
                                if pod_len > 0 {
                                    self.episode_menu.scroll(-1);
                                    self.update_details_panel();
                                }
                            },
                        }
                    },

                    Some(UserAction::Left) => {
                        if pod_len > 0 {
                            match self.active_menu {
                                ActiveMenu::PodcastMenu => (),
                                ActiveMenu::EpisodeMenu => {
                                    self.active_menu = ActiveMenu::PodcastMenu;
                                    self.podcast_menu.activate();
                                    self.episode_menu.deactivate();
                                },
                            }
                        }
                    },

                    Some(UserAction::Right) => {
                        if pod_len > 0 && ep_len > 0 {
                            match self.active_menu {
                                ActiveMenu::PodcastMenu => {
                                    self.active_menu = ActiveMenu::EpisodeMenu;
                                    self.podcast_menu.deactivate();
                                    self.episode_menu.activate();
                                },
                                ActiveMenu::EpisodeMenu => (),
                            }
                        }
                    },

                    Some(UserAction::AddFeed) => {
                        let url = &self.spawn_input_win("Feed URL: ");
                        if !url.is_empty() {
                            return UiMsg::AddFeed(url.to_string());
                        }
                    },

                    Some(UserAction::Sync) => {
                        if pod_len > 0 {
                            return UiMsg::Sync(current_pod_index);
                        }
                    },
                    Some(UserAction::SyncAll) => {
                        if pod_len > 0 {
                            return UiMsg::SyncAll;
                        }
                    },
                    Some(UserAction::Play) => {
                        if ep_len > 0 {
                            return UiMsg::Play(current_pod_index, current_ep_index);
                        }
                    },
                    Some(UserAction::MarkPlayed) => {
                        match self.active_menu {
                            ActiveMenu::PodcastMenu => (),
                            ActiveMenu::EpisodeMenu => {
                                if ep_len > 0 {
                                    let played = self.episode_menu.items
                                        .borrow()
                                        .get(current_ep_index).unwrap()
                                        .is_played();
                                    return UiMsg::MarkPlayed(current_pod_index, current_ep_index, !played);
                                }
                            },
                        }
                    },
                    Some(UserAction::MarkAllPlayed) => {
                        // if there are any unplayed episodes, MarkAllPlayed
                        // will convert all to played; if all are played
                        // already, only then will it convert all to unplayed
                        if pod_len > 0 {
                            let played = self.podcast_menu.items
                                .borrow()
                                .get(current_pod_index).unwrap()
                                .is_played();
                            return UiMsg::MarkAllPlayed(current_pod_index, !played);
                        }
                    },

                    Some(UserAction::Download) => {
                        if ep_len > 0 {
                            return UiMsg::Download(current_pod_index, current_ep_index);
                        }
                    },

                    Some(UserAction::DownloadAll) => {
                        if pod_len > 0 {
                            return UiMsg::DownloadAll(current_pod_index);
                        }
                    },

                    Some(UserAction::Delete) => {
                        match self.active_menu {
                            ActiveMenu::PodcastMenu => (),
                            ActiveMenu::EpisodeMenu => {
                                if ep_len > 0 {
                                    return UiMsg::Delete(current_pod_index, current_ep_index);
                                }
                            },
                        }
                    },

                    Some(UserAction::DeleteAll) => {
                        if pod_len > 0 {
                            return UiMsg::DeleteAll(current_pod_index);
                        }
                    },

                    Some(UserAction::Remove) => {
                        let mut delete = false;

                        match self.active_menu {
                            ActiveMenu::PodcastMenu => {
                                if pod_len > 0 {
                                    // check if we have local files first
                                    let mut any_downloaded = false;
                                    {
                                        let borrowed_podcast_list = self.podcast_menu.items.borrow();
                                        let borrowed_podcast = borrowed_podcast_list.get(current_pod_index).unwrap();
                                        let borrowed_ep_list = borrowed_podcast.episodes.borrow();

                                        for ep in borrowed_ep_list.iter() {
                                            if ep.path.is_some() {
                                                any_downloaded = true;
                                                break;
                                            }
                                        }
                                    }

                                    if any_downloaded {
                                        let ask_delete = self.spawn_yes_no_win("Delete local files too?");
                                        delete = match ask_delete {
                                            Some(val) => val,
                                            None => false,  // default not to delete
                                        };
                                    }

                                    return UiMsg::RemovePodcast(current_pod_index, delete);
                                }
                            },
                            ActiveMenu::EpisodeMenu => {
                                if ep_len > 0 {

                                    // check if we have local files first
                                    let is_downloaded;
                                    {
                                        let borrowed_ep_list = self.episode_menu.items.borrow();
                                        is_downloaded = borrowed_ep_list
                                            .get(current_ep_index).unwrap()
                                            .path.is_some();
                                    }
                                    if is_downloaded {
                                        let ask_delete = self.spawn_yes_no_win("Delete local file too?");
                                        delete = match ask_delete {
                                            Some(val) => val,
                                            None => false,  // default not to delete
                                        };
                                    }

                                    return UiMsg::RemoveEpisode(current_pod_index, current_ep_index, delete);
                                }
                            },
                        }
                    },
                    Some(UserAction::RemoveAll) => {
                        if pod_len > 0 {
                            let mut delete = false;
                            
                            // check if we have local files first
                            let mut any_downloaded = false;
                            {
                                let borrowed_podcast_list = self.podcast_menu.items.borrow();
                                let borrowed_podcast = borrowed_podcast_list.get(current_pod_index).unwrap();
                                let borrowed_ep_list = borrowed_podcast.episodes.borrow();

                                for ep in borrowed_ep_list.iter() {
                                    if ep.path.is_some() {
                                        any_downloaded = true;
                                        break;
                                    }
                                }
                            }

                            if any_downloaded {
                                let ask_delete = self.spawn_yes_no_win("Delete local files too?");
                                delete = match ask_delete {
                                    Some(val) => val,
                                    None => false,  // default not to delete
                                };
                            }
                            return match self.active_menu {
                                ActiveMenu::PodcastMenu => UiMsg::RemovePodcast(current_pod_index, delete),
                                ActiveMenu::EpisodeMenu => UiMsg::RemoveAllEpisodes(current_pod_index, delete),
                            }
                        }
                    },

                    Some(UserAction::Quit) => {
                        return UiMsg::Quit;
                    },
                    None => (),
                }  // end of input match
            },
            None => (),
        };  // end of getch() match
        return UiMsg::Noop;
    }

    /// Calculates the number of columns to allocate for each of the
    /// main panels: podcast menu, episodes menu, and details panel; if
    /// the screen is too small to display the details panel, this size
    /// will be 0
    #[allow(clippy::useless_let_if_seq)]
    pub fn calculate_sizes(n_col: i32) -> (i32, i32, i32) {
        let pod_col;
        let ep_col;
        let det_col;
        if n_col > crate::config::DETAILS_PANEL_LENGTH {
            pod_col = n_col / 3;
            ep_col = n_col / 3 + 1;
            det_col = n_col - pod_col - ep_col + 2;
        } else {
            pod_col = n_col / 2;
            ep_col = n_col - pod_col + 1;
            det_col = 0;
        }
        return (pod_col, ep_col, det_col);
    }

    /// Adds a one-line pancurses window to the bottom of the screen to
    /// solicit user text input. A prefix can be specified as a prompt
    /// for the user at the beginning of the input line. This returns the
    /// user's input; if the user cancels their input, the String will be
    /// empty.
    pub fn spawn_input_win(&self, prefix: &str) -> String {
        let input_win = newwin(1, self.n_col, self.n_row-1, 0);
        // input_win.overlay(&self.podcast_menu.window);
        input_win.mv(self.n_row-1, 0);
        input_win.addstr(&prefix);
        input_win.keypad(true);
        input_win.refresh();
        pancurses::curs_set(2);
        
        let mut inputs = String::new();
        let mut cancelled = false;

        let min_x = prefix.len() as i32;
        let mut current_x = prefix.len() as i32;
        let mut cursor_x = prefix.len() as i32;
        loop {
            match input_win.getch() {
                // Cancel input
                Some(Input::KeyExit) |
                Some(Input::Character('\u{1b}')) => {
                    cancelled = true;
                    break;
                },
                // Complete input
                Some(Input::KeyEnter) |
                Some(Input::Character('\n')) => {
                    break;
                },
                Some(Input::KeyBackspace) |
                Some(Input::Character('\u{7f}')) => {
                    if current_x > min_x {
                        current_x -= 1;
                        cursor_x -= 1;
                        let _ = inputs.remove((cursor_x as usize) - prefix.len());
                        input_win.mv(0, cursor_x);
                        input_win.delch();
                    }
                },
                Some(Input::KeyDC) => {
                    if cursor_x < current_x {
                        let _ = inputs.remove((cursor_x as usize) - prefix.len());
                        input_win.delch();
                    }
                },
                Some(Input::KeyLeft) => {
                    if cursor_x > min_x {
                        cursor_x -= 1;
                        input_win.mv(0, cursor_x);
                    }
                },
                Some(Input::KeyRight) => {
                    if cursor_x < current_x {
                        cursor_x += 1;
                        input_win.mv(0, cursor_x);
                    }
                },
                Some(Input::Character(c)) => {
                    current_x += 1;
                    cursor_x += 1;
                    input_win.insch(c);
                    input_win.mv(0, cursor_x);
                    inputs.push(c);
                },
                Some(_) => (),
                None => (),
            }
            input_win.refresh();
        }

        pancurses::curs_set(0);
        input_win.deleteln();
        input_win.refresh();
        input_win.delwin();

        if cancelled {
            return String::from("");
        }
        return inputs;
    }

    /// Adds a one-line pancurses window to the bottom of the screen to
    /// solicit user for a yes/no input. A prefix can be specified as a
    /// prompt for the user at the beginning of the input line. "(y/n)"
    /// will automatically be appended to the end of the prefix. If the
    /// user types 'y' or 'n', the boolean will represent this value. If
    /// the user cancels the input or types anything else, the function
    /// will return None.
    pub fn spawn_yes_no_win(&self, prefix: &str) -> Option<bool> {
        let mut out_val = None;
        let input = self.spawn_input_win(&format!("{} {}", prefix, "(y/n) "));
        if let Some(c) = input.trim().chars().next() {
            if c == 'Y' || c == 'y' {
                out_val = Some(true);
            } else if c == 'N' || c == 'n' {
                out_val = Some(false);
            }
        }
        return out_val;
    }

    /// Adds a one-line pancurses window to the bottom of the screen for
    /// displaying messages to the user. `duration` indicates how long
    /// (in milliseconds) this message will remain on screen. Useful for
    /// presenting error messages, among other things.
    pub fn spawn_msg_win(&self, message: String, duration: u64, error: bool) {
        let n_col = self.n_col;
        let begy = self.n_row - 1;
        let err_color = self.colors.get(ColorType::Error);
        thread::spawn(move || {
            let msg_win = newwin(1, n_col, begy, 0);
            msg_win.mv(begy, 0);
            msg_win.attrset(pancurses::A_NORMAL);
            msg_win.addstr(message);

            if error {
                msg_win.mvchgat(0, 0, -1, pancurses::A_BOLD,
                    err_color);
            }
            msg_win.refresh();

            // TODO: This probably should be some async function, but this
            // works for now
            // pancurses::napms(duration);
            thread::sleep(Duration::from_millis(duration));
            
            msg_win.erase();
            msg_win.refresh();
            msg_win.delwin();
        });
    }

    /// Forces the menus to check the list of podcasts/episodes again and
    /// update.
    pub fn update_menus(&mut self) {
        self.podcast_menu.update_items();

        self.episode_menu.items = if self.podcast_menu.items.len() > 0 {
            self.podcast_menu.get_episodes()
        } else {
            LockVec::new(Vec::new())
        };
        self.episode_menu.update_items();

        match self.active_menu {
            ActiveMenu::PodcastMenu => self.podcast_menu.highlight_selected(true),
            ActiveMenu::EpisodeMenu => {
                self.podcast_menu.highlight_selected(false);
                self.episode_menu.highlight_selected(true);
            },
        }
    }
    
    /// When the program is ending, this performs tear-down functions so
    /// that the terminal is properly restored to its prior settings.
    pub fn tear_down(&self) {
        pancurses::endwin();
    }

    /// Create a details panel.
    pub fn make_details_panel(colors: Colors, n_row: i32, n_col: i32, start_y: i32, start_x: i32) -> Panel {
        return Panel::new(
            colors,
            "Details".to_string(),
            2,
            n_row, n_col,
            start_y, start_x);
    }

    /// Updates the details panel with information about the current
    /// podcast and episode, and redraws to the screen.
    pub fn update_details_panel(&mut self) {
        if self.details_panel.is_some() {
            let det = self.details_panel.as_mut().unwrap();
            det.erase();
            if self.episode_menu.items.len() > 0 {
                // let det = self.details_panel.as_ref().unwrap();
                let current_pod = (self.podcast_menu.selected +
                    self.podcast_menu.top_row) as usize;
                let current_ep = (self.episode_menu.selected +
                    self.episode_menu.top_row) as usize;

                    // get a couple details from the current podcast
                    let mut pod_title = None;
                    let mut pod_explicit = None;
                    if let Some(pod) = self.podcast_menu.items.borrow().get(current_pod) {
                        pod_title = if pod.title.is_empty() {
                            None
                        } else {
                            Some(pod.title.clone())
                        };
                        pod_explicit = pod.explicit;
                    };

                    // the rest of the details come from the current episode
                    if let Some(ep) = self.episode_menu.items.borrow().get(current_ep) {
                        let ep_title = if ep.title.is_empty() {
                            None
                        } else {
                            Some(ep.title.clone())
                        };

                        let desc = if ep.description.is_empty() {
                            None
                        } else {
                            // strip all HTML tags and excessive line breaks
                            let stripped_tags = RE_HTML_TAGS.replace_all(&ep.description, "").to_string();

                            // remove anything more than two line breaks (i.e., one blank line)
                            let no_line_breaks = RE_MULT_LINE_BREAKS.replace_all(&stripped_tags, "\n\n");

                            Some(no_line_breaks.to_string())
                        };

                        let details = Details {
                            pod_title: pod_title,
                            ep_title: ep_title,
                            pubdate: ep.pubdate,
                            duration: Some(ep.format_duration()),
                            explicit: pod_explicit,
                            description: desc,
                        };
                        det.details_template(0, details);
                    };

                det.refresh();
            }
        }
    }

    /// Creates a pancurses window with a welcome message for when users
    /// start the program for the first time. Responsibility for managing
    /// the window is given back to the main UI object.
    pub fn make_welcome_win(colors: Colors, keymap: &Keybindings,
        n_row: i32, n_col:i32) -> Panel {

        let add_keys = keymap.keys_for_action(UserAction::AddFeed);
        let quit_keys = keymap.keys_for_action(UserAction::Quit);

        let add_str = match add_keys.len() {
            0 => "<missing>".to_string(),
            1 => format!("\"{}\"", &add_keys[0]),
            2 => format!("\"{}\" or \"{}\"", add_keys[0], add_keys[1]),
            _ => {
                let mut s = "".to_string();
                for i in 0..add_keys.len() {
                    if i == add_keys.len() - 1 {
                        s = format!("{}, \"{}\"", s, add_keys[i]);
                    } else {
                        s = format!("{}, or \"{}\"", s, add_keys[i]);
                    }
                }
                s
            }
        };

        let quit_str = match quit_keys.len() {
            0 => "<missing>".to_string(),
            1 => format!("\"{}\"", &quit_keys[0]),
            2 => format!("\"{}\" or \"{}\"", quit_keys[0], quit_keys[1]),
            _ => {
                let mut s = "".to_string();
                for i in 0..quit_keys.len() {
                    if i == quit_keys.len() - 1 {
                        s = format!("{}, \"{}\"", s, quit_keys[i]);
                    } else {
                        s = format!("{}, or \"{}\"", s, quit_keys[i]);
                    }
                }
                s
            }
        };

        // the warning on the unused mut is a function of Rust getting
        // confused between panel.rs and mock_panel.rs
        #[allow(unused_mut)]
        let mut welcome_win = Panel::new(
            colors,
            "Shellcaster".to_string(),
            0,
            n_row, n_col, 0, 0
        );

        let mut row = 0;
        row = welcome_win.write_wrap_line(row+1, "Welcome to shellcaster!".to_string());

        row = welcome_win.write_wrap_line(row+2,
            format!("Your podcast list is currently empty. Press {} to add a new podcast feed, or {} to quit.", add_str, quit_str));

        row = welcome_win.write_wrap_line(row+2, "Other keybindings can be found on the Github repo readme:".to_string());
        let _ = welcome_win.write_wrap_line(row+1, "https://github.com/jeff-hughes/shellcaster".to_string());

        return welcome_win;
    }
}