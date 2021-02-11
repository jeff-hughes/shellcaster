use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[cfg_attr(not(test), path = "panel.rs")]
#[cfg_attr(test, path = "mock_panel.rs")]
mod panel;

mod colors;
mod menu;
mod notification;
mod popup;

use self::colors::{ColorType, Colors};
use self::menu::Menu;
use self::notification::NotifWin;
use self::panel::{Details, Panel};
use self::popup::PopupWin;

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
    DownloadMulti(Vec<(i64, i64)>),
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
    popup_win: PopupWin<'a>,
}

impl<'a> UI<'a> {
    /// Spawns a UI object in a new thread, with message channels to send
    /// and receive messages
    pub fn spawn(
        config: Config,
        items: LockVec<Podcast>,
        rx_from_main: mpsc::Receiver<MainMessage>,
        tx_to_main: mpsc::Sender<Message>,
    ) -> thread::JoinHandle<()> {
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
                        MainMessage::UiSpawnDownloadPopup(episodes, selected) => {
                            ui.popup_win.spawn_download_win(episodes, selected);
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
        let podcast_menu = Menu::new(podcast_panel, None, items.clone());

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

        let episode_menu = Menu::new(episode_panel, None, first_pod);

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
        let popup_win = PopupWin::new(colors.clone(), &config.keybindings, n_row, n_col);

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
            popup_win: popup_win,
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

        // welcome screen if user does not have any podcasts yet
        if self.podcast_menu.items.is_empty() {
            self.popup_win.spawn_welcome_win();
        }
    }

    /// Waits for user input and, where necessary, provides UiMsgs
    /// back to the main controller.
    ///
    /// Anything UI-related (e.g., scrolling up and down menus) is handled
    /// internally, producing an empty UiMsg. This allows for some
    /// greater degree of abstraction; for example, input to add a new
    /// podcast feed spawns a UI window to capture the feed URL, and only
    /// then passes this data back to the main controller.
    pub fn getch(&mut self) -> UiMsg {
        match self.stdscr.getch() {
            Some(Input::KeyResize) => self.resize(),

            Some(input) => {
                let (curr_pod_id, curr_ep_id) = self.get_current_ids();

                // get rid of the "welcome" window once the podcast list
                // is no longer empty
                if self.popup_win.welcome_win && !self.podcast_menu.items.is_empty() {
                    self.popup_win.turn_off_welcome_win();
                }

                // if there is a popup window active (apart from the
                // welcome window which takes no input), then
                // redirect user input there
                if self.popup_win.is_non_welcome_popup_active() {
                    let popup_msg = self.popup_win.handle_input(input);

                    // need to check if popup window is still active, as
                    // handling character input above may involve
                    // closing the popup window
                    if !self.popup_win.is_popup_active() {
                        self.stdscr.refresh();
                        self.update_menus();
                        if self.details_panel.is_some() {
                            self.update_details_panel();
                        }
                    }
                    return popup_msg;
                } else {
                    match self.keymap.get_from_input(input) {
                        Some(a @ UserAction::Down)
                        | Some(a @ UserAction::Up)
                        | Some(a @ UserAction::Left)
                        | Some(a @ UserAction::Right)
                        | Some(a @ UserAction::PageUp)
                        | Some(a @ UserAction::PageDown)
                        | Some(a @ UserAction::BigUp)
                        | Some(a @ UserAction::BigDown)
                        | Some(a @ UserAction::GoTop)
                        | Some(a @ UserAction::GoBot) => {
                            self.move_cursor(a, curr_pod_id, curr_ep_id)
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
                                if let Some(ui_msg) = self.mark_played(curr_pod_id, curr_ep_id) {
                                    return ui_msg;
                                }
                            }
                        },
                        Some(UserAction::MarkAllPlayed) => {
                            if let Some(ui_msg) = self.mark_all_played(curr_pod_id) {
                                return ui_msg;
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

                        Some(UserAction::Remove) => match self.active_menu {
                            ActiveMenu::PodcastMenu => {
                                if let Some(ui_msg) = self.remove_podcast(curr_pod_id) {
                                    return ui_msg;
                                }
                            }
                            ActiveMenu::EpisodeMenu => {
                                if let Some(ui_msg) = self.remove_episode(curr_pod_id, curr_ep_id) {
                                    return ui_msg;
                                }
                            }
                        },
                        Some(UserAction::RemoveAll) => {
                            let ui_msg = match self.active_menu {
                                ActiveMenu::PodcastMenu => self.remove_podcast(curr_pod_id),
                                ActiveMenu::EpisodeMenu => self.remove_all_episodes(curr_pod_id),
                            };
                            if let Some(ui_msg) = ui_msg {
                                return ui_msg;
                            }
                        }

                        Some(UserAction::Help) => self.popup_win.spawn_help_win(),

                        Some(UserAction::Quit) => {
                            return UiMsg::Quit;
                        }
                        None => (),
                    } // end of input match
                }
            }
            None => (),
        }; // end of getch() match
        return UiMsg::Noop;
    }

    /// Resize all the windows on the screen and refresh.
    pub fn resize(&mut self) {
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

        self.popup_win.resize(n_row, n_col);
        self.notif_win.resize(n_row, n_col);
        self.stdscr.refresh();
    }

    /// Move the menu cursor around and refresh menus when necessary.
    pub fn move_cursor(
        &mut self,
        action: &UserAction,
        curr_pod_id: Option<i64>,
        curr_ep_id: Option<i64>,
    ) {
        match action {
            UserAction::Down => {
                self.scroll_current_window(curr_pod_id, 1);
            }

            UserAction::Up => {
                self.scroll_current_window(curr_pod_id, -1);
            }

            UserAction::Left => {
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
                if let Some(det) = &self.details_panel {
                    det.refresh();
                }
            }

            UserAction::Right => {
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
                if let Some(det) = &self.details_panel {
                    det.refresh();
                }
            }

            UserAction::PageUp => {
                self.scroll_current_window(curr_pod_id, -self.n_row + 3);
            }

            UserAction::PageDown => {
                self.scroll_current_window(curr_pod_id, self.n_row - 3);
            }

            UserAction::BigUp => {
                self.scroll_current_window(
                    curr_pod_id,
                    -self.n_row / crate::config::BIG_SCROLL_AMOUNT,
                );
            }

            UserAction::BigDown => {
                self.scroll_current_window(
                    curr_pod_id,
                    self.n_row / crate::config::BIG_SCROLL_AMOUNT,
                );
            }

            UserAction::GoTop => {
                self.scroll_current_window(curr_pod_id, -i32::MAX);
            }

            UserAction::GoBot => {
                self.scroll_current_window(curr_pod_id, i32::MAX);
            }

            // this shouldn't occur because we only trigger this
            // function when the UserAction is Up, Down, Left, Right, BigUp, BigDown,
            // PageUp, PageDown, GoBot and GoTop
            _ => (),
        }
    }

    /// Scrolls the current active menu by
    /// the specified amount and refreshes
    /// the window.
    /// Positive Scroll is down.
    pub fn scroll_current_window(&mut self, pod_id: Option<i64>, scroll: i32) {
        match self.active_menu {
            ActiveMenu::PodcastMenu => {
                if pod_id.is_some() {
                    self.podcast_menu.scroll(scroll);

                    self.episode_menu.top_row = 0;
                    self.episode_menu.selected = 0;

                    // update episodes menu with new list
                    self.episode_menu.items = self.podcast_menu.get_episodes();
                    self.episode_menu.update_items();
                    self.update_details_panel();
                }
            }
            ActiveMenu::EpisodeMenu => {
                if pod_id.is_some() {
                    self.episode_menu.scroll(scroll);
                    self.update_details_panel();
                }
            }
        }
    }

    /// Mark an episode as played or unplayed (opposite of its current
    /// status).
    pub fn mark_played(
        &mut self,
        curr_pod_id: Option<i64>,
        curr_ep_id: Option<i64>,
    ) -> Option<UiMsg> {
        if let Some(pod_id) = curr_pod_id {
            if let Some(ep_id) = curr_ep_id {
                if let Some(played) = self
                    .episode_menu
                    .items
                    .map_single(ep_id, |ep| ep.is_played())
                {
                    return Some(UiMsg::MarkPlayed(pod_id, ep_id, !played));
                }
            }
        }
        return None;
    }

    /// Mark all episodes for a given podcast as played or unplayed. If
    /// there are any unplayed episodes, this will convert all episodes
    /// to played; if all are played already, only then will it convert
    /// all to unplayed.
    pub fn mark_all_played(&mut self, curr_pod_id: Option<i64>) -> Option<UiMsg> {
        if let Some(pod_id) = curr_pod_id {
            if let Some(played) = self
                .podcast_menu
                .items
                .map_single(pod_id, |pod| pod.is_played())
            {
                return Some(UiMsg::MarkAllPlayed(pod_id, !played));
            }
        }
        return None;
    }

    /// Remove a podcast from the list.
    pub fn remove_podcast(&mut self, curr_pod_id: Option<i64>) -> Option<UiMsg> {
        let confirm = self.ask_for_confirmation("Are you sure you want to remove the podcast?");
        // If we don't get a confirmation to delete, then don't remove
        if !confirm {
            return None;
        }
        let mut delete = false;

        if let Some(pod_id) = curr_pod_id {
            // check if we have local files first and if so, ask whether
            // to delete those too
            if self.check_for_local_files(pod_id) {
                let ask_delete = self.spawn_yes_no_notif("Delete local files too?");
                delete = match ask_delete {
                    Some(val) => val,
                    None => false, // default not to delete
                };
            }

            return Some(UiMsg::RemovePodcast(pod_id, delete));
        }
        return None;
    }

    /// Remove an episode from the list for the current podcast.
    fn remove_episode(
        &mut self,
        curr_pod_id: Option<i64>,
        curr_ep_id: Option<i64>,
    ) -> Option<UiMsg> {
        let confirm = self.ask_for_confirmation("Are you sure you want to remove the episode?");
        // If we don't get a confirmation to delete, then don't remove
        if !confirm {
            return None;
        }
        let mut delete = false;
        if let Some(pod_id) = curr_pod_id {
            if let Some(ep_id) = curr_ep_id {
                // check if we have local files first
                let is_downloaded = self
                    .episode_menu
                    .items
                    .map_single(ep_id, |ep| ep.path.is_some())
                    .unwrap();
                if is_downloaded {
                    let ask_delete = self.spawn_yes_no_notif("Delete local file too?");
                    delete = match ask_delete {
                        Some(val) => val,
                        None => false, // default not to delete
                    };
                }

                return Some(UiMsg::RemoveEpisode(pod_id, ep_id, delete));
            }
        }
        return None;
    }

    /// Remove all episodes from the list for the current podcast.
    fn remove_all_episodes(&mut self, curr_pod_id: Option<i64>) -> Option<UiMsg> {
        if let Some(pod_id) = curr_pod_id {
            let mut delete = false;

            // check if we have local files first and if so, ask whether
            // to delete those too
            if self.check_for_local_files(pod_id) {
                let ask_delete = self.spawn_yes_no_notif("Delete local files too?");
                delete = match ask_delete {
                    Some(val) => val,
                    None => false, // default not to delete
                };
            }
            return Some(UiMsg::RemoveAllEpisodes(pod_id, delete));
        }
        return None;
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

    /// Checks whether the user has downloaded any episodes for the
    /// given podcast to their local system.
    pub fn check_for_local_files(&self, pod_id: i64) -> bool {
        let mut any_downloaded = false;
        let borrowed_map = self.podcast_menu.items.borrow_map();
        let borrowed_pod = borrowed_map.get(&pod_id).unwrap();

        let borrowed_ep_list = borrowed_pod.episodes.borrow_map();

        for (_ep_id, ep) in borrowed_ep_list.iter() {
            if ep.path.is_some() {
                any_downloaded = true;
                break;
            }
        }
        return any_downloaded;
    }

    /// Spawns a "(y/n)" notification with the specified input
    /// `message` using `spawn_input_notif`. If the the user types
    /// 'y', then the function returns `true`, and 'n' returns
    /// `false`. Cancelling the action returns `false` as well.
    pub fn ask_for_confirmation(&self, message: &str) -> bool {
        match self.spawn_yes_no_notif(message) {
            Some(val) => val,
            None => false,
        }
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
    ) -> Panel {
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
}
