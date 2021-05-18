use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

// use super::ColorType;
use crossterm::{
    cursor,
    event::{self, KeyCode},
    execute, queue, style,
};

use super::AppColors;

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
    colors: Rc<AppColors>,
    start_y: u16,
    total_rows: u16,
    total_cols: u16,
    msg_stack: Vec<Notification>,
    persistent_msg: Option<Notification>,
    current_msg: Option<Notification>,
}

impl NotifWin {
    /// Creates a new NotifWin.
    pub fn new(colors: Rc<AppColors>, start_y: u16, total_rows: u16, total_cols: u16) -> Self {
        return Self {
            colors: colors,
            start_y: start_y,
            total_rows: total_rows,
            total_cols: total_cols,
            msg_stack: Vec::new(),
            persistent_msg: None,
            current_msg: None,
        };
    }

    /// Initiates the window -- primarily, sets the background on the
    /// window.
    pub fn redraw(&self) {
        // clear the panel
        // TODO: Set the background color first
        let empty = vec![" "; self.total_cols as usize];
        let empty_string = empty.join("");
        queue!(
            io::stdout(),
            cursor::MoveTo(0, self.start_y),
            style::PrintStyledContent(
                style::style(&empty_string)
                    .with(self.colors.normal.0)
                    .on(self.colors.normal.1)
            ),
        )
        .unwrap();
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
                self.redraw();
                self.current_msg = None;
            }
        }
    }

    /// Adds a notification that solicits user text input. A prefix can
    /// be specified as a prompt for the user at the beginning of the
    /// input line. This returns the user's input; if the user cancels
    /// their input, the String will be empty.
    pub fn input_notif(&self, prefix: &str) -> String {
        execute!(
            io::stdout(),
            cursor::MoveTo(0, self.start_y),
            style::Print(&prefix),
            cursor::Show
        )
        .unwrap();
        // self.window.mv(self.total_rows - 1, 0);
        // self.window.addstr(&prefix);
        // self.window.keypad(true);
        // self.window.refresh();
        // pancurses::curs_set(2);

        let mut inputs = String::new();
        let mut cancelled = false;

        let min_x = prefix.len() as u16;
        let mut current_max_x = prefix.len() as u16;
        let mut cursor_x = prefix.len() as u16;
        loop {
            if let event::Event::Key(input) = event::read().expect("") {
                let cursor_idx = (cursor_x - min_x) as usize;
                match input.code {
                    // Cancel input
                    KeyCode::Esc | KeyCode::Char('\u{1b}') => {
                        cancelled = true;
                        break;
                    }
                    // Complete input
                    KeyCode::Enter | KeyCode::Char('\n') => {
                        break;
                    }
                    KeyCode::Backspace | KeyCode::Char('\u{7f}') => {
                        if current_max_x > min_x {
                            current_max_x -= 1;
                            cursor_x -= 1;
                            let _ = inputs.remove(cursor_idx - 1);
                            execute!(io::stdout(), cursor::MoveLeft(1)).unwrap();
                            for i in inputs.chars().skip(cursor_idx - 1) {
                                execute!(io::stdout(), style::Print(i)).unwrap();
                            }
                            execute!(
                                io::stdout(),
                                style::Print(" "),
                                cursor::MoveTo(cursor_x, self.start_y)
                            )
                            .unwrap();
                        }
                    }
                    KeyCode::Delete => {
                        if cursor_x < current_max_x {
                            current_max_x -= 1;
                            let _ = inputs.remove(cursor_idx);
                            for i in inputs.chars().skip(cursor_idx) {
                                execute!(io::stdout(), style::Print(i)).unwrap();
                            }
                            execute!(
                                io::stdout(),
                                style::Print(" "),
                                cursor::MoveTo(cursor_x, self.start_y)
                            )
                            .unwrap();
                        }
                    }
                    KeyCode::Left => {
                        if cursor_x > min_x {
                            cursor_x -= 1;
                            execute!(io::stdout(), cursor::MoveLeft(1)).unwrap();
                        }
                    }
                    KeyCode::Right => {
                        if cursor_x < current_max_x {
                            cursor_x += 1;
                            execute!(io::stdout(), cursor::MoveRight(1)).unwrap();
                        }
                    }
                    KeyCode::Char(c) => {
                        if cursor_x < current_max_x {
                            current_max_x += 1;
                            cursor_x += 1;
                            inputs.insert(cursor_idx, c);
                            for i in inputs.chars().skip(cursor_idx) {
                                execute!(io::stdout(), style::Print(i)).unwrap();
                            }
                            execute!(io::stdout(), cursor::MoveTo(cursor_x, self.start_y)).unwrap();
                        } else {
                            current_max_x += 1;
                            cursor_x += 1;
                            inputs.push(c);
                            execute!(io::stdout(), style::Print(c)).unwrap();
                        }
                    }
                    _ => (),
                }
            }
        }

        execute!(io::stdout(), cursor::Hide).unwrap();
        self.redraw();

        if cancelled {
            return String::from("");
        }
        return inputs;
    }

    /// Prints a notification to the window.
    fn display_notif(&self, notif: &Notification) {
        self.redraw();
        let styled = if notif.error {
            style::style(&notif.message)
                .with(self.colors.error.0)
                .on(self.colors.error.1)
                .attribute(style::Attribute::Bold)
        } else {
            style::style(&notif.message)
                .with(self.colors.normal.0)
                .on(self.colors.normal.1)
        };
        queue!(
            io::stdout(),
            cursor::MoveTo(0, self.start_y),
            style::PrintStyledContent(styled)
        )
        .unwrap();
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
            self.redraw();
            self.current_msg = None;
        }
    }

    /// Updates window size/location
    pub fn resize(&mut self, total_rows: u16, total_cols: u16) {
        // self.total_rows = total_rows;
        // self.total_cols = total_cols;

        // // apparently pancurses does not implement `wresize()`
        // // from ncurses, so instead we create an entirely new
        // // window every time the terminal is resized...not ideal,
        // // but c'est la vie
        // let oldwin = std::mem::replace(
        //     &mut self.window,
        //     pancurses::newwin(1, total_cols, total_rows - 1, 0),
        // );
        // oldwin.delwin();

        // self.window
        //     .bkgdset(pancurses::ColorPair(ColorType::Normal as u8));
        // if let Some(curr) = &self.current_msg {
        //     self.display_notif(curr);
        // }
        // self.window.refresh();
    }
}
