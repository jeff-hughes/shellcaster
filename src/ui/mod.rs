use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[cfg_attr(not(test), path = "panel.rs")]
#[cfg_attr(test, path = "mock_panel.rs")]
mod panel;

mod colors;
mod menu;
mod notification;

use self::colors::{ColorType, Colors};
use self::menu::Menu;
use self::notification::NotifWin;
use self::panel::{Details, Panel};

use lazy_static::lazy_static;
use pancurses::{Input, Window};
use regex::Regex;

use super::MainMessage;
use crate::config::Config;
use crate::keymap::{Keybindings, UserAction};
use crate::types::*;

lazy_static! {
    /// Regex for finding <br/> tags -- also captures any surrounding
    /// line breaks
    static ref RE_BR_TAGS: Regex = Regex::new(r"((\r\n)|\r|\n)*<br */?>((\r\n)|\r|\n)*").unwrap();

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
    Play(i64, i64),
    MarkPlayed(i64, i64, bool),
    MarkAllPlayed(i64, bool),
    Sync(i64),
    SyncAll,
    Download(i64, i64),
    DownloadAll(i64),
    Delete(i64, i64),
    DeleteAll(i64),
    RemovePodcast(i64, bool),
    RemoveEpisode(i64, i64, bool),
    RemoveAllEpisodes(i64, bool),
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
    notif_win: NotifWin,
    welcome_win: Option<Panel>,
}

impl<'a> UI<'a> {
    /// Spawns a UI object in a new thread, with message channels to send
    /// and receive messages
    pub fn spawn(
        config: Config,
        items: LockVec<Podcast>,
        rx_from_main: mpsc::Receiver<MainMessage>,
        tx_to_main: mpsc::Sender<Message>,
    ) -> thread::JoinHandle<()>
    {
        return thread::spawn(move || {
            let mut ui = UI::new(&config, &items);
            ui.init();
            let mut message_iter = rx_from_main.try_iter();
            // this is the main event loop: on each loop, we update
            // any messages at the bottom, check for user input, and
            // then process any messages from the main thread
            loop {
                ui.notif_win.check_notifs();

                match ui.getch() {
                    UiMsg::Noop => (),
                    input => tx_to_main.send(Message::Ui(input)).unwrap(),
                }

                if let Some(message) = message_iter.next() {
                    match message {
                        MainMessage::UiUpdateMenus => ui.update_menus(),
                        MainMessage::UiSpawnNotif(msg, duration, error) => {
                            ui.timed_notif(msg, error, duration)
                        }
                        MainMessage::UiSpawnPersistentNotif(msg, error) => {
                            ui.persistent_notif(msg, error)
                        }
                        MainMessage::UiClearPersistentNotif => ui.clear_persistent_notif(),
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
        pancurses::cbreak(); // allows characters to be read one by one
        pancurses::noecho(); // turns off automatic echoing of characters
                             // to the screen as they are input
        pancurses::start_color(); // allows colours if available
        pancurses::curs_set(0); // turn off cursor
        stdscr.keypad(true); // returns special characters as single
                             // key codes
        stdscr.nodelay(true); // getch() will not wait for user input

        // set colors
        let colors = self::colors::set_colors();

        let (n_row, n_col) = stdscr.get_max_yx();
        let (pod_col, ep_col, det_col) = Self::calculate_sizes(n_col);

        let podcast_panel = Panel::new(
            colors.clone(),
            "Podcasts".to_string(),
            0,
            n_row - 1,
            pod_col,
            0,
            0,
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
            n_row - 1,
            ep_col,
            0,
            pod_col - 1,
        );

        let first_pod = match items.borrow_order().get(0) {
            Some(first_id) => match items.borrow_map().get(first_id) {
                Some(pod) => pod.episodes.clone(),
                None => LockVec::new(Vec::new()),
            },
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
                n_row - 1,
                det_col,
                0,
                pod_col + ep_col - 2,
            ))
        } else {
            None
        };

        let notif_win = NotifWin::new(colors.clone(), n_row, n_col);

        // welcome screen if user does not have any podcasts yet
        let welcome_win = if items.is_empty() {
            Some(UI::make_welcome_win(
                colors.clone(),
                &config.keybindings,
                n_row - 1,
                n_col,
            ))
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
            notif_win: notif_win,
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
    #[allow(clippy::cognitive_complexity)]
    pub fn getch(&mut self) -> UiMsg {
        match self.stdscr.getch() {
            Some(Input::KeyResize) => {
                pancurses::resize_term(0, 0);
                let (n_row, n_col) = self.stdscr.get_max_yx();
                self.n_row = n_row;
                self.n_col = n_col;

                let (pod_col, ep_col, det_col) = Self::calculate_sizes(n_col);

                self.podcast_menu.resize(n_row - 1, pod_col, 0, 0);
                self.episode_menu.resize(n_row - 1, ep_col, 0, pod_col - 1);

                if self.details_panel.is_some() {
                    if det_col > 0 {
                        let det = self.details_panel.as_mut().unwrap();
                        det.resize(n_row - 1, det_col, 0, pod_col + ep_col - 2);
                    } else {
                        self.details_panel = None;
                    }
                } else if det_col > 0 {
                    self.details_panel = Some(Self::make_details_panel(
                        self.colors.clone(),
                        n_row - 1,
                        det_col,
                        0,
                        pod_col + ep_col - 2,
                    ));
                }

                self.stdscr.refresh();
                self.update_menus();

                match self.active_menu {
                    ActiveMenu::PodcastMenu => self.podcast_menu.activate(),
                    ActiveMenu::EpisodeMenu => {
                        self.podcast_menu.activate();
                        self.episode_menu.activate();
                    }
                }

                if self.details_panel.is_some() {
                    self.update_details_panel();
                }

                // resize welcome window, if it exists
                if self.welcome_win.is_some() {
                    let _ = std::mem::replace(
                        &mut self.welcome_win,
                        Some(UI::make_welcome_win(
                            self.colors.clone(),
                            &self.keymap,
                            n_row - 1,
                            n_col,
                        )),
                    );

                    let ww = self.welcome_win.as_mut().unwrap();
                    ww.refresh();
                }

                self.notif_win.resize(n_row, n_col);
                self.stdscr.refresh();
            }

            Some(input) => {
                let (curr_pod_id, curr_ep_id) = self.get_current_ids();

                // get rid of the "welcome" window once the podcast list
                // is no longer empty
                if self.welcome_win.is_some() && !self.podcast_menu.items.len() > 0 {
                    self.welcome_win = None;
                }

                match self.keymap.get_from_input(input) {
                    Some(UserAction::Down) => {
                        match self.active_menu {
                            ActiveMenu::PodcastMenu => {
                                if curr_pod_id.is_some() {
                                    self.podcast_menu.scroll(1);

                                    self.episode_menu.top_row = 0;
                                    self.episode_menu.selected = 0;

                                    // update episodes menu with new list
                                    self.episode_menu.items = self.podcast_menu.get_episodes();
                                    self.episode_menu.update_items();
                                    self.update_details_panel();
                                }
                            }
                            ActiveMenu::EpisodeMenu => {
                                if curr_ep_id.is_some() {
                                    self.episode_menu.scroll(1);
                                    self.update_details_panel();
                                }
                            }
                        }
                    }

                    Some(UserAction::Up) => {
                        match self.active_menu {
                            ActiveMenu::PodcastMenu => {
                                if curr_pod_id.is_some() {
                                    self.podcast_menu.scroll(-1);

                                    self.episode_menu.top_row = 0;
                                    self.episode_menu.selected = 0;

                                    // update episodes menu with new list
                                    self.episode_menu.items = self.podcast_menu.get_episodes();
                                    self.episode_menu.update_items();
                                    self.update_details_panel();
                                }
                            }
                            ActiveMenu::EpisodeMenu => {
                                if curr_pod_id.is_some() {
                                    self.episode_menu.scroll(-1);
                                    self.update_details_panel();
                                }
                            }
                        }
                    }

                    Some(UserAction::Left) => {
                        if curr_pod_id.is_some() {
                            match self.active_menu {
                                ActiveMenu::PodcastMenu => (),
                                ActiveMenu::EpisodeMenu => {
                                    self.active_menu = ActiveMenu::PodcastMenu;
                                    self.podcast_menu.activate();
                                    self.episode_menu.deactivate();
                                }
                            }
                        }
                    }

                    Some(UserAction::Right) => {
                        if curr_pod_id.is_some() && curr_ep_id.is_some() {
                            match self.active_menu {
                                ActiveMenu::PodcastMenu => {
                                    self.active_menu = ActiveMenu::EpisodeMenu;
                                    self.podcast_menu.deactivate();
                                    self.episode_menu.activate();
                                }
                                ActiveMenu::EpisodeMenu => (),
                            }
                        }
                    }

                    Some(UserAction::AddFeed) => {
                        let url = &self.spawn_input_notif("Feed URL: ");
                        if !url.is_empty() {
                            return UiMsg::AddFeed(url.to_string());
                        }
                    }

                    Some(UserAction::Sync) => {
                        if let Some(pod_id) = curr_pod_id {
                            return UiMsg::Sync(pod_id);
                        }
                    }
                    Some(UserAction::SyncAll) => {
                        if curr_pod_id.is_some() {
                            return UiMsg::SyncAll;
                        }
                    }
                    Some(UserAction::Play) => {
                        if let Some(pod_id) = curr_pod_id {
                            if let Some(ep_id) = curr_ep_id {
                                return UiMsg::Play(pod_id, ep_id);
                            }
                        }
                    }
                    Some(UserAction::MarkPlayed) => match self.active_menu {
                        ActiveMenu::PodcastMenu => (),
                        ActiveMenu::EpisodeMenu => {
                            if let Some(pod_id) = curr_pod_id {
                                if let Some(ep_id) = curr_ep_id {
                                    if let Some(played) = self
                                        .episode_menu
                                        .items
                                        .map_single(ep_id, |ep| ep.is_played())
                                    {
                                        return UiMsg::MarkPlayed(pod_id, ep_id, !played);
                                    }
                                }
                            }
                        }
                    },
                    Some(UserAction::MarkAllPlayed) => {
                        // if there are any unplayed episodes, MarkAllPlayed
                        // will convert all to played; if all are played
                        // already, only then will it convert all to unplayed
                        if let Some(pod_id) = curr_pod_id {
                            if let Some(played) = self
                                .podcast_menu
                                .items
                                .map_single(pod_id, |pod| pod.is_played())
                            {
                                return UiMsg::MarkAllPlayed(pod_id, !played);
                            }
                        }
                    }

                    Some(UserAction::Download) => {
                        if let Some(pod_id) = curr_pod_id {
                            if let Some(ep_id) = curr_ep_id {
                                return UiMsg::Download(pod_id, ep_id);
                            }
                        }
                    }

                    Some(UserAction::DownloadAll) => {
                        if let Some(pod_id) = curr_pod_id {
                            return UiMsg::DownloadAll(pod_id);
                        }
                    }

                    Some(UserAction::Delete) => match self.active_menu {
                        ActiveMenu::PodcastMenu => (),
                        ActiveMenu::EpisodeMenu => {
                            if let Some(pod_id) = curr_pod_id {
                                if let Some(ep_id) = curr_ep_id {
                                    return UiMsg::Delete(pod_id, ep_id);
                                }
                            }
                        }
                    },

                    Some(UserAction::DeleteAll) => {
                        if let Some(pod_id) = curr_pod_id {
                            return UiMsg::DeleteAll(pod_id);
                        }
                    }

                    Some(UserAction::Remove) => {
                        let mut delete = false;

                        match self.active_menu {
                            ActiveMenu::PodcastMenu => {
                                if let Some(pod_id) = curr_pod_id {
                                    // check if we have local files first
                                    let mut any_downloaded = false;
                                    {
                                        let borrowed_map = self.podcast_menu.items.borrow_map();
                                        let borrowed_pod = borrowed_map.get(&pod_id).unwrap();

                                        let borrowed_ep_list = borrowed_pod.episodes.borrow_map();

                                        for (_ep_id, ep) in borrowed_ep_list.iter() {
                                            if ep.path.is_some() {
                                                any_downloaded = true;
                                                break;
                                            }
                                        }
                                    }

                                    if any_downloaded {
                                        let ask_delete =
                                            self.spawn_yes_no_notif("Delete local files too?");
                                        delete = match ask_delete {
                                            Some(val) => val,
                                            None => false, // default not to delete
                                        };
                                    }

                                    return UiMsg::RemovePodcast(pod_id, delete);
                                }
                            }
                            ActiveMenu::EpisodeMenu => {
                                if let Some(pod_id) = curr_pod_id {
                                    if let Some(ep_id) = curr_ep_id {
                                        // check if we have local files first
                                        let is_downloaded = self
                                            .episode_menu
                                            .items
                                            .map_single(ep_id, |ep| ep.path.is_some())
                                            .unwrap();
                                        if is_downloaded {
                                            let ask_delete =
                                                self.spawn_yes_no_notif("Delete local file too?");
                                            delete = match ask_delete {
                                                Some(val) => val,
                                                None => false, // default not to delete
                                            };
                                        }

                                        return UiMsg::RemoveEpisode(pod_id, ep_id, delete);
                                    }
                                }
                            }
                        }
                    }
                    Some(UserAction::RemoveAll) => {
                        if let Some(pod_id) = curr_pod_id {
                            let mut delete = false;

                            // check if we have local files first
                            let mut any_downloaded = false;
                            {
                                let borrowed_map = self.podcast_menu.items.borrow_map();
                                let borrowed_pod = borrowed_map.get(&pod_id).unwrap();

                                let borrowed_ep_list = borrowed_pod.episodes.borrow_map();

                                for (_ep_id, ep) in borrowed_ep_list.iter() {
                                    if ep.path.is_some() {
                                        any_downloaded = true;
                                        break;
                                    }
                                }
                            }

                            if any_downloaded {
                                let ask_delete = self.spawn_yes_no_notif("Delete local files too?");
                                delete = match ask_delete {
                                    Some(val) => val,
                                    None => false, // default not to delete
                                };
                            }
                            return match self.active_menu {
                                ActiveMenu::PodcastMenu => UiMsg::RemovePodcast(pod_id, delete),
                                ActiveMenu::EpisodeMenu => UiMsg::RemoveAllEpisodes(pod_id, delete),
                            };
                        }
                    }

                    Some(UserAction::Quit) => {
                        return UiMsg::Quit;
                    }
                    None => (),
                } // end of input match
            }
            None => (),
        }; // end of getch() match
        return UiMsg::Noop;
    }

    /// Based on the current selected value of the podcast and episode
    /// menus, returns the IDs of the current podcast and episode (if
    /// they exist).
    pub fn get_current_ids(&self) -> (Option<i64>, Option<i64>) {
        let current_pod_index = (self.podcast_menu.selected + self.podcast_menu.top_row) as usize;
        let current_ep_index = (self.episode_menu.selected + self.episode_menu.top_row) as usize;

        let current_pod_id = self
            .podcast_menu
            .items
            .borrow_order()
            .get(current_pod_index)
            .copied();
        let current_ep_id = self
            .episode_menu
            .items
            .borrow_order()
            .get(current_ep_index)
            .copied();
        return (current_pod_id, current_ep_id);
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

    /// Adds a notification to the bottom of the screen that solicits
    /// user text input. A prefix can be specified as a prompt for the
    /// user at the beginning of the input line. This returns the user's
    /// input; if the user cancels their input, the String will be empty.
    pub fn spawn_input_notif(&self, prefix: &str) -> String {
        return self.notif_win.input_notif(prefix);
    }

    /// Adds a notification to the bottom of the screen that solicits
    /// user for a yes/no input. A prefix can be specified as a prompt
    /// for the user at the beginning of the input line. "(y/n)" will
    /// automatically be appended to the end of the prefix. If the user
    /// types 'y' or 'n', the boolean will represent this value. If the
    /// user cancels the input or types anything else, the function will
    /// return None.
    pub fn spawn_yes_no_notif(&self, prefix: &str) -> Option<bool> {
        let mut out_val = None;
        let input = self
            .notif_win
            .input_notif(&format!("{} {}", prefix, "(y/n) "));
        if let Some(c) = input.trim().chars().next() {
            if c == 'Y' || c == 'y' {
                out_val = Some(true);
            } else if c == 'N' || c == 'n' {
                out_val = Some(false);
            }
        }
        return out_val;
    }

    /// Adds a notification to the bottom of the screen for `duration`
    /// time  (in milliseconds). Useful for presenting error messages,
    /// among other things.
    pub fn timed_notif(&mut self, message: String, duration: u64, error: bool) {
        self.notif_win.timed_notif(message, duration, error);
    }

    /// Adds a notification to the bottom of the screen that will stay on
    /// screen indefinitely. Must use `clear_persistent_msg()` to erase.
    pub fn persistent_notif(&mut self, message: String, error: bool) {
        self.notif_win.persistent_notif(message, error);
    }

    /// Clears any persistent notification that is being displayed at the
    /// bottom of the screen. Does not affect timed notifications, user
    /// input notifications, etc.
    pub fn clear_persistent_notif(&mut self) {
        self.notif_win.clear_persistent_notif();
    }

    /// Forces the menus to check the list of podcasts/episodes again and
    /// update.
    pub fn update_menus(&mut self) {
        self.podcast_menu.update_items();

        self.episode_menu.items = if !self.podcast_menu.items.is_empty() {
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
            }
        }
    }

    /// When the program is ending, this performs tear-down functions so
    /// that the terminal is properly restored to its prior settings.
    pub fn tear_down(&self) {
        pancurses::endwin();
    }

    /// Create a details panel.
    pub fn make_details_panel(
        colors: Colors,
        n_row: i32,
        n_col: i32,
        start_y: i32,
        start_x: i32,
    ) -> Panel
    {
        return Panel::new(
            colors,
            "Details".to_string(),
            2,
            n_row,
            n_col,
            start_y,
            start_x,
        );
    }

    /// Updates the details panel with information about the current
    /// podcast and episode, and redraws to the screen.
    pub fn update_details_panel(&mut self) {
        if self.details_panel.is_some() {
            let (curr_pod_id, curr_ep_id) = self.get_current_ids();
            let det = self.details_panel.as_mut().unwrap();
            det.erase();
            if let Some(pod_id) = curr_pod_id {
                if let Some(ep_id) = curr_ep_id {
                    // get a couple details from the current podcast
                    let mut pod_title = None;
                    let mut pod_explicit = None;
                    if let Some(pod) = self.podcast_menu.items.borrow_map().get(&pod_id) {
                        pod_title = if pod.title.is_empty() {
                            None
                        } else {
                            Some(pod.title.clone())
                        };
                        pod_explicit = pod.explicit;
                    };

                    // the rest of the details come from the current episode
                    if let Some(ep) = self.episode_menu.items.borrow_map().get(&ep_id) {
                        let ep_title = if ep.title.is_empty() {
                            None
                        } else {
                            Some(ep.title.clone())
                        };

                        let desc = if ep.description.is_empty() {
                            None
                        } else {
                            // convert <br/> tags to a single line break
                            let br_to_lb = RE_BR_TAGS.replace_all(&ep.description, "\n");

                            // strip all HTML tags
                            let stripped_tags = RE_HTML_TAGS.replace_all(&br_to_lb, "");

                            // convert HTML entities (e.g., &amp;)
                            let decoded = match escaper::decode_html(&stripped_tags) {
                                Err(_) => stripped_tags.to_string(),
                                Ok(s) => s,
                            };

                            // remove anything more than two line breaks (i.e., one blank line)
                            let no_line_breaks = RE_MULT_LINE_BREAKS.replace_all(&decoded, "\n\n");

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
    }

    /// Creates a pancurses window with a welcome message for when users
    /// start the program for the first time. Responsibility for managing
    /// the window is given back to the main UI object.
    pub fn make_welcome_win(colors: Colors, keymap: &Keybindings, n_row: i32, n_col: i32) -> Panel {
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
        let mut welcome_win = Panel::new(colors, "Shellcaster".to_string(), 0, n_row, n_col, 0, 0);

        let mut row = 0;
        row = welcome_win.write_wrap_line(row + 1, "Welcome to shellcaster!".to_string());

        row = welcome_win.write_wrap_line(row+2,
            format!("Your podcast list is currently empty. Press {} to add a new podcast feed, or {} to quit.", add_str, quit_str));

        row = welcome_win.write_wrap_line(
            row + 2,
            "Other keybindings can be found on the Github repo readme:".to_string(),
        );
        let _ = welcome_win.write_wrap_line(
            row + 1,
            "https://github.com/jeff-hughes/shellcaster".to_string(),
        );

        return welcome_win;
    }
}
