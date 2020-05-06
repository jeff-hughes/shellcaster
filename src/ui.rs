use std::cmp::min;
use std::rc::Rc;
use core::cell::RefCell;

use std::thread;
use std::time::Duration;

use pancurses::{Window, newwin, Input};
use crate::config::Config;
use crate::keymap::{Keybindings, UserAction};
use crate::types::{Podcast, Episode, MutableVec, Menuable};

/// Enum used for communicating back to the main controller after user
/// input has been captured by the UI. `response` can be any String, and
/// `message` includes any corresponding details going along with the 
/// response (e.g., user has inputted a podcast feed URL).
#[derive(Debug)]
pub enum UiMessage {
    AddFeed(String),
    Play(i32, i32),
    Sync(i32),
    SyncAll,
    Download(i32, i32),
    DownloadAll(i32),
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
    podcast_menu: Menu<Podcast>,
    episode_menu: Menu<Episode>,
    active_menu: ActiveMenu,
    welcome_win: Option<Window>,
}

impl<'a> UI<'a> {
    /// Initializes the UI with a list of podcasts and podcast episodes,
    /// creates the pancurses window and draws it to the screen, and
    /// returns a UI object for future manipulation.
    pub fn new(config: &'a Config, items: &MutableVec<Podcast>) -> UI<'a> {
        let stdscr = pancurses::initscr();

        // set some options
        pancurses::cbreak();  // allows characters to be read one by one
        pancurses::noecho();  // turns off automatic echoing of characters
                              // to the screen as they are input
        pancurses::start_color();  // allows colours if available
        pancurses::curs_set(0);  // turn off cursor
        stdscr.keypad(true);  // returns special characters as single
                              // key codes

        let (n_row, n_col) = stdscr.get_max_yx();

        let pod_col = n_col / 2;
        let ep_col = n_col - pod_col;

        let podcast_menu_win = newwin(n_row, pod_col, 0, 0);
        let mut podcast_menu = Menu {
            window: podcast_menu_win,
            items: Rc::clone(items),
            n_row: n_row,
            n_col: pod_col,
            top_row: 0,
            selected: 0,
        };

        stdscr.noutrefresh();
        podcast_menu.init();
        podcast_menu.window.mvchgat(podcast_menu.selected, 0, -1, pancurses::A_REVERSE, 0);
        podcast_menu.window.noutrefresh();

        let episode_menu_win = newwin(n_row, ep_col, 0, pod_col);
        let first_pod = match items.borrow().get(0) {
            Some(pod) => Rc::clone(&pod.episodes),
            None => Rc::new(RefCell::new(Vec::new())),
        };
        let mut episode_menu = Menu {
            window: episode_menu_win,
            items: first_pod,
            n_row: n_row,
            n_col: ep_col,
            top_row: 0,
            selected: 0,
        };
        episode_menu.init();
        episode_menu.window.noutrefresh();

        // welcome screen if user does not have any podcasts yet
        let mut welcome_win = None;
        if items.borrow().len() == 0 {
            welcome_win = Some(UI::make_welcome_win(&config, n_row, n_col));
        }

        pancurses::doupdate();

        return UI {
            stdscr,
            n_row,
            n_col,
            keymap: &config.keybindings,
            podcast_menu: podcast_menu,
            episode_menu: episode_menu,
            active_menu: ActiveMenu::PodcastMenu,
            welcome_win: welcome_win,
        };
    }

    /// Waits for user input and, where necessary, provides UiMessages
    /// back to the main controller.
    /// 
    /// Anything UI-related (e.g., scrolling up and down menus) is handled
    /// internally, producing an empty UiMessage. This allows for some
    /// greater degree of abstraction; for example, input to add a new
    /// podcast feed spawns a UI window to capture the feed URL, and only
    /// then passes this data back to the main controller.
    pub fn getch(&mut self) -> UiMessage {
        match self.stdscr.getch() {
            Some(Input::KeyResize) => {
                pancurses::resize_term(0, 0);
                let (n_row, n_col) = self.stdscr.get_max_yx();
                self.n_row = n_row;
                self.n_col = n_col;

                let pod_col = n_col / 2;
                let ep_col = n_col - pod_col;
                self.podcast_menu.resize(n_row, pod_col);
                self.episode_menu.resize(n_row, ep_col);

                // apparently pancurses does not implement `wresize()`
                // from ncurses, so instead we create an entirely new
                // window every time the terminal is resized...not ideal,
                // but c'est la vie
                let pod_oldwin = std::mem::replace(
                    &mut self.podcast_menu.window,
                    newwin(n_row, pod_col, 0, 0));
                let ep_oldwin = std::mem::replace(
                    &mut self.episode_menu.window,
                    newwin(n_row, ep_col, 0, pod_col));
                pod_oldwin.delwin();
                ep_oldwin.delwin();
                self.stdscr.refresh();
                self.update_menus();

                match self.active_menu {
                    ActiveMenu::PodcastMenu => self.podcast_menu.activate(),
                    ActiveMenu::EpisodeMenu => {
                        self.podcast_menu.activate();
                        self.episode_menu.activate();
                    },
                }
            },

            Some(input) => {
                let pod_len = self.podcast_menu.items.borrow().len();
                let ep_len = self.episode_menu.items.borrow().len();
                let current_pod_index = self.podcast_menu.selected +
                    self.podcast_menu.top_row;
                let current_ep_index = self.episode_menu.selected +
                    self.episode_menu.top_row;

                // get rid of the "welcome" window once the podcast list
                // is no longer empty
                if pod_len > 0 && self.welcome_win.is_some() {
                    let ww = self.welcome_win.take().unwrap();
                    ww.delwin();
                }

                match self.keymap.get_from_input(&input) {
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
                                }
                            },
                            ActiveMenu::EpisodeMenu => {
                                if ep_len > 0 {
                                    self.episode_menu.scroll(1);
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
                                }
                            },
                            ActiveMenu::EpisodeMenu => {
                                if pod_len > 0 {
                                    self.episode_menu.scroll(-1);
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
                                    // self.podcast_menu.deactivate();
                                    self.episode_menu.activate();
                                },
                                ActiveMenu::EpisodeMenu => (),
                            }
                        }
                    },

                    Some(UserAction::AddFeed) => {
                        let url = &self.spawn_input_win("Feed URL: ");
                        if url.len() > 0 {
                            return UiMessage::AddFeed(url.to_string());
                        }
                    },

                    Some(UserAction::Sync) => {
                        return UiMessage::Sync(current_pod_index);
                    },
                    Some(UserAction::SyncAll) => {
                        return UiMessage::SyncAll;
                    },
                    Some(UserAction::Play) => {
                        if ep_len > 0 {
                            return UiMessage::Play(current_pod_index, current_ep_index);
                        }
                    },
                    Some(UserAction::MarkPlayed) => {},
                    Some(UserAction::MarkAllPlayed) => {},

                    Some(UserAction::Download) => {
                        if ep_len > 0 {
                            return UiMessage::Download(current_pod_index, current_ep_index);
                        }
                    },

                    Some(UserAction::DownloadAll) => {
                        if pod_len > 0 {
                            return UiMessage::DownloadAll(current_pod_index);
                        }
                    },

                    Some(UserAction::Delete) => {},
                    Some(UserAction::DeleteAll) => {},
                    Some(UserAction::Remove) => {},
                    Some(UserAction::RemoveAll) => {},
                    Some(UserAction::Search) => {},

                    Some(UserAction::Quit) => {
                        return UiMessage::Quit;
                    },
                    None => (),
                }  // end of input match
            },
            None => (),
        };  // end of getch() match
        return UiMessage::Noop;
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

    /// Adds a one-line pancurses window to the bottom of the screen for
    /// displaying messages to the user. `duration` indicates how long
    /// (in milliseconds) this message will remain on screen. Useful for
    /// presenting error messages, among other things.
    pub fn spawn_msg_win(&self, message: &str, duration: u64) {
        let n_col = self.n_col;
        let begy = self.n_row - 1;
        let msg = message.to_string();
        thread::spawn(move || {
            let msg_win = newwin(1, n_col, begy, 0);
            msg_win.mv(begy, 0);
            msg_win.attrset(pancurses::A_NORMAL);
            msg_win.addstr(msg);
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
        self.episode_menu.update_items();
    }
    
    /// When the program is ending, this performs tear-down functions so
    /// that the terminal is properly restored to its prior settings.
    pub fn tear_down(&self) {
        pancurses::endwin();
    }

    /// Creates a pancurses window with a welcome message for when users
    /// start the program for the first time. Responsibility for managing
    /// the window is given back to the main UI object.
    pub fn make_welcome_win(config: &Config,
        n_row: i32, n_col:i32) -> Window {

        let add_keys = config.keybindings.keys_for_action(UserAction::AddFeed);
        let quit_keys = config.keybindings.keys_for_action(UserAction::Quit);

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

        let welcome_win = newwin(n_row, n_col, 0, 0);
        welcome_win.mv(0, 0);
        welcome_win.addstr(format!("Welcome to shellcaster!\n\nYour podcast list is currently empty. Press {} to add a new podcast feed, or {} to quit.\n\nOther keybindings can be found on the Github repo readme:\nhttps://github.com/jeff-hughes/shellcaster", add_str, quit_str));
        welcome_win.refresh();
        return welcome_win;
    }
}

/// Generic struct holding details about a list menu. These menus are
/// contained by the UI, and hold the list of podcasts or podcast
/// episodes. They also hold the pancurses window used to display the menu
/// to the user.
/// 
/// * `n_row` and `n_col` store the size of the `window`
/// * `top_row` indicates the top line of text that is shown on screen
///   (since the list of items can be longer than the available size of
///   the screen). `top_row` is calculated relative to the `items` index,
///   i.e., it will be a value between 0 and items.len()
/// * `selected` indicates which item on screen is currently highlighted.
///   It is calculated relative to the screen itself, i.e., a value between
///   0 and (n_row - 1)
/// * `old_selected` indicates which item on screen *was* highlighted,
///   which is used when the user is scrolling through the list (TODO:
///   this will probably be changed at some point)
#[derive(Debug)]
pub struct Menu<T> {
    window: Window,
    items: MutableVec<T>,
    n_row: i32,
    n_col: i32,
    top_row: i32,  // top row of text shown in window
    selected: i32,  // which line of text is highlighted
}

impl<T: Menuable> Menu<T> {
    /// Prints the list of visible items to the pancurses window and
    /// refreshes it.
    pub fn init(&mut self) {
        self.update_items();
    }

    /// Scrolls the menu up or down by `lines` lines. Negative values of
    /// `lines` will scroll the menu up.
    /// 
    /// This function examines the new selected value, ensures it does
    /// not fall out of bounds, and then updates the pancurses window to
    /// represent the new visible list.
    fn scroll(&mut self, lines: i32) {
        // TODO: currently only handles scroll value of 1; need to extend
        // to be able to scroll multiple lines at a time
        let mut old_selected = self.selected;
        self.selected += lines;

        // don't allow scrolling past last item in list (if shorter than
        // self.n_row)
        let abs_bottom = min(self.n_row,
            (self.items.borrow().len() - 1) as i32);
        if self.selected > abs_bottom {
            self.selected = abs_bottom;
        }

        // scroll list if necessary
        if self.selected > (self.n_row - 1) {
            self.selected = self.n_row - 1;
            if let Some(elem) = self.items.borrow().get((self.top_row + self.n_row) as usize) {
                self.top_row += 1;
                self.window.mv(0, 0);
                self.window.deleteln();
                old_selected -= 1;

                self.window.mv(self.n_row-1, 0);
                self.window.clrtoeol();
                self.window.addstr(elem.get_title(self.n_col as usize));
            }

        } else if self.selected < 0 {
            self.selected = 0;
            if let Some(elem) = self.items.borrow().get((self.top_row - 1) as usize) {
                self.top_row -= 1;
                self.window.mv(0, 0);
                self.window.insertln();
                old_selected += 1;

                self.window.mv(0, 0);
                self.window.addstr(elem.get_title(self.n_col as usize));
            }
        }

        self.window.mvchgat(old_selected, 0, -1, pancurses::A_NORMAL, 0);
        self.window.mvchgat(self.selected, 0, -1, pancurses::A_REVERSE, 0);
        self.window.refresh();
    }

    /// Controls how the window changes when it is active (i.e., available
    /// for user input to modify state).
    fn activate(&mut self) {
        self.window.mvchgat(self.selected, 0, -1, pancurses::A_REVERSE, 0);
        self.window.refresh();
    }

    /// Controls how the window changes when it is inactive (i.e., not
    /// available for user input to modify state).
    fn deactivate(&mut self) {
        self.window.mvchgat(self.selected, 0, -1, pancurses::A_NORMAL, 0);
        self.window.refresh();
    }

    /// Prints or reprints the list of visible items to the pancurses
    /// window and refreshes it.
    fn update_items(&mut self) {
        self.window.erase();
        // for visible rows, print strings from list
        for i in 0..self.n_row {
            let item_idx = self.top_row + i;
            if let Some(elem) = self.items.borrow().get(item_idx as usize) {
                self.window.mvaddstr(i, 0, elem.get_title(self.n_col as usize));
            } else {
                break;
            }
        }
        self.window.refresh();
    }

    /// Updates window size
    fn resize(&mut self, n_row: i32, n_col: i32) {
        self.n_row = n_row;
        self.n_col = n_col;

        // if resizing moves selected item off screen, scroll the list
        // upwards to keep same item selected
        if self.selected > (self.n_row - 1) {
            self.top_row = self.top_row + self.selected - (self.n_row - 1);
            self.selected = self.n_row - 1;
        }
    }
}

impl Menu<Podcast> {
    /// Returns a cloned reference to the list of episodes from the
    /// currently selected podcast.
    pub fn get_episodes(&self) -> MutableVec<Episode> {
        let index = self.selected + self.top_row;
        return Rc::clone(&self.items.borrow()
            .get(index as usize).unwrap().episodes);
    }
}