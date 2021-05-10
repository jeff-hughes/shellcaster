use std::rc::Rc;
use std::time::{Duration, Instant};

use super::{ColorType, Colors};
use pancurses::{Input, Window};

/// Holds details of a notification message.
#[derive(Debug, Clone, PartialEq)]
struct Notification {
    message: String,
    error: bool,
    expiry: Option<Instant>,
}

impl Notification {
    /// Creates a new Notification. The `expiry` is optional, and is
    /// used to create timed notifications -- `Instant` should refer
    /// to the timestamp when the message should disappear.
    pub fn new(message: String, error: bool, expiry: Option<Instant>) -> Self {
        return Self {
            message: message,
            error: error,
            expiry: expiry,
        };
    }
}

/// A struct handling the one-line message window at the bottom of the
/// screen. Holds state about the size of the window as well as any
/// persistent message text.
///
/// The `msg_stack` holds a vector of all timed notifications, each
/// pushed on the end of the stack. The last notification on the stack
/// will be the one displayed; however, they will be removed from the
/// stack based on their expiry times. As such, it will generally be a
/// FIFO approach (older notifications will generally expire first), but
/// not necessarily.
#[derive(Debug)]
pub struct NotifWin {
    window: Window,
    colors: Rc<Colors>,
    total_rows: i32,
    total_cols: i32,
    msg_stack: Vec<Notification>,
    persistent_msg: Option<Notification>,
    current_msg: Option<Notification>,
}

impl NotifWin {
    /// Creates a new NotifWin.
    pub fn new(colors: Rc<Colors>, total_rows: i32, total_cols: i32) -> Self {
        let win = pancurses::newwin(1, total_cols, total_rows - 1, 0);
        return Self {
            window: win,
            colors: colors,
            total_rows: total_rows,
            total_cols: total_cols,
            msg_stack: Vec::new(),
            persistent_msg: None,
            current_msg: None,
        };
    }

    /// Initiates the window -- primarily, sets the background on the
    /// window.
    pub fn init(&mut self) {
        self.window.bkgd(pancurses::ColorPair(
            self.colors.get(ColorType::Normal) as u8
        ));
        self.window.refresh();
    }

    /// Checks if the current notification needs to be changed, and
    /// updates the message window accordingly.
    pub fn check_notifs(&mut self) {
        if !self.msg_stack.is_empty() {
            // compare expiry times of all notifications to current time,
            // remove expired ones
            let now = Instant::now();
            self.msg_stack.retain(|x| match x.expiry {
                Some(exp) => now < exp,
                None => true,
            });

            if !self.msg_stack.is_empty() {
                // check if last item changed, and update screen if it has
                let last_item = &self.msg_stack[self.msg_stack.len() - 1];
                match &self.current_msg {
                    Some(curr) => {
                        if last_item != curr {
                            self.display_notif(last_item);
                        }
                    }
                    None => self.display_notif(last_item),
                };
                self.current_msg = Some(last_item.clone());
            } else if let Some(msg) = &self.persistent_msg {
                // if no other timed notifications exist, display a
                // persistent notification if there is one
                match &self.current_msg {
                    Some(curr) => {
                        if msg != curr {
                            self.display_notif(msg);
                        }
                    }
                    None => self.display_notif(msg),
                };
                self.current_msg = Some(msg.clone());
            } else {
                // otherwise, there was a notification before but there
                // isn't now, so erase
                self.window.erase();
                self.window.bkgdset(pancurses::ColorPair(
                    self.colors.get(ColorType::Normal) as u8
                ));
                self.window.refresh();
                self.current_msg = None;
            }
        }
    }

    /// Adds a notification that solicits user text input. A prefix can
    /// be specified as a prompt for the user at the beginning of the
    /// input line. This returns the user's input; if the user cancels
    /// their input, the String will be empty.
    pub fn input_notif(&self, prefix: &str) -> String {
        self.window.mv(self.total_rows - 1, 0);
        self.window.addstr(&prefix);
        self.window.keypad(true);
        self.window.refresh();
        pancurses::curs_set(2);

        let mut inputs = String::new();
        let mut cancelled = false;

        let min_x = prefix.len() as i32;
        let mut current_x = prefix.len() as i32;
        let mut cursor_x = prefix.len() as i32;
        loop {
            match self.window.getch() {
                // Cancel input
                Some(Input::KeyExit) | Some(Input::Character('\u{1b}')) => {
                    cancelled = true;
                    break;
                }
                // Complete input
                Some(Input::KeyEnter) | Some(Input::Character('\n')) => {
                    break;
                }
                Some(Input::KeyBackspace) | Some(Input::Character('\u{7f}')) => {
                    if current_x > min_x {
                        current_x -= 1;
                        cursor_x -= 1;
                        let _ = inputs.remove((cursor_x as usize) - prefix.len());
                        self.window.mv(0, cursor_x);
                        self.window.delch();
                    }
                }
                Some(Input::KeyDC) => {
                    if cursor_x < current_x {
                        let _ = inputs.remove((cursor_x as usize) - prefix.len());
                        self.window.delch();
                    }
                }
                Some(Input::KeyLeft) => {
                    if cursor_x > min_x {
                        cursor_x -= 1;
                        self.window.mv(0, cursor_x);
                    }
                }
                Some(Input::KeyRight) => {
                    if cursor_x < current_x {
                        cursor_x += 1;
                        self.window.mv(0, cursor_x);
                    }
                }
                Some(Input::Character(c)) => {
                    current_x += 1;
                    cursor_x += 1;
                    self.window.insch(c);
                    self.window.mv(0, cursor_x);
                    inputs.push(c);
                }
                Some(_) => (),
                None => (),
            }
            self.window.refresh();
        }

        pancurses::curs_set(0);
        self.window.clear();
        self.window.refresh();

        if cancelled {
            return String::from("");
        }
        return inputs;
    }

    /// Prints a notification to the window.
    fn display_notif(&self, notif: &Notification) {
        self.window.erase();
        self.window.mv(self.total_rows - 1, 0);
        self.window.attrset(pancurses::A_NORMAL);
        self.window.addstr(&notif.message);

        if notif.error {
            self.window.mvchgat(
                0,
                0,
                -1,
                pancurses::A_BOLD,
                self.colors.get(ColorType::Error),
            );
        }
        self.window.refresh();
    }

    /// Adds a notification to the user. `duration` indicates how long
    /// (in milliseconds) this message will remain on screen. Useful for
    /// presenting error messages, among other things.
    pub fn timed_notif(&mut self, message: String, duration: u64, error: bool) {
        let expiry = Instant::now() + Duration::from_millis(duration);
        self.msg_stack
            .push(Notification::new(message, error, Some(expiry)));
    }

    /// Adds a notification that will stay on screen indefinitely. Must
    /// use `clear_persistent_notif()` to erase. If a persistent
    /// notification is already being displayed, this method will
    /// overwrite that message.
    pub fn persistent_notif(&mut self, message: String, error: bool) {
        let notif = Notification::new(message, error, None);
        self.persistent_msg = Some(notif.clone());
        if self.msg_stack.is_empty() {
            self.display_notif(&notif);
            self.current_msg = Some(notif);
        }
    }

    /// Clears any persistent notification that is being displayed. Does
    /// not affect timed notifications, user input notifications, etc.
    pub fn clear_persistent_notif(&mut self) {
        self.persistent_msg = None;
        if self.msg_stack.is_empty() {
            self.window.erase();
            self.window.refresh();
            self.current_msg = None;
        }
    }

    /// Updates window size/location
    pub fn resize(&mut self, total_rows: i32, total_cols: i32) {
        self.total_rows = total_rows;
        self.total_cols = total_cols;

        // apparently pancurses does not implement `wresize()`
        // from ncurses, so instead we create an entirely new
        // window every time the terminal is resized...not ideal,
        // but c'est la vie
        let oldwin = std::mem::replace(
            &mut self.window,
            pancurses::newwin(1, total_cols, total_rows - 1, 0),
        );
        oldwin.delwin();

        self.window.bkgdset(pancurses::ColorPair(
            self.colors.get(ColorType::Normal) as u8
        ));
        if let Some(curr) = &self.current_msg {
            self.display_notif(curr);
        }
        self.window.refresh();
    }
}
