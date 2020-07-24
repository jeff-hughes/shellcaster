use pancurses::{Window, Attribute};
use chrono::{DateTime, Utc};

use super::{Colors, ColorType};

/// Struct holding the raw data used for building the details panel.
pub struct Details {
    pub pod_title: Option<String>,
    pub ep_title: Option<String>,
    pub pubdate: Option<DateTime<Utc>>,
    pub duration: Option<String>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
}

/// Panels abstract away a pancurses window, and handles all methods
/// associated with writing data to that window. A panel includes a
/// border and margin around the edge of the window, and a title that
/// appears at the top. The Panel will translate the x and y coordinates
/// to account for the border and margins, so users of the methods can
/// calculate rows and columns relative to the Panel.
#[derive(Debug)]
pub struct Panel {
    window: Window,
    screen_pos: usize,
    colors: Colors,
    title: String,
    n_row: i32,
    n_col: i32,
}

impl Panel {
    /// Creates a new panel.
    pub fn new(colors: Colors,
        title: String, screen_pos: usize, n_row: i32, n_col: i32, start_y: i32, start_x: i32) -> Self {

        let panel_win = pancurses::newwin(
            n_row,
            n_col,
            start_y,
            start_x);

        return Panel {
            window: panel_win,
            screen_pos: screen_pos,
            colors: colors,
            title: title,
            n_row: n_row,
            n_col: n_col,
        };
    }

    /// Initiates the menu -- primarily, draws borders on the window.
    pub fn init(&self) {
        self.draw_border();
    }

    /// Redraws borders and refreshes the window to display on terminal.
    pub fn refresh(&self) {
        self.draw_border();
        self.window.refresh();
    }

    /// Draws a border around the window.
    fn draw_border(&self) {
        let top_left;
        let bot_left;
        match self.screen_pos {
            0 => {
                top_left = pancurses::ACS_ULCORNER();
                bot_left = pancurses::ACS_LLCORNER();
            }
            _ => {
                top_left = pancurses::ACS_TTEE();
                bot_left = pancurses::ACS_BTEE();
            }
        }
        self.window.border(
            pancurses::ACS_VLINE(),
            pancurses::ACS_VLINE(),
            pancurses::ACS_HLINE(),
            pancurses::ACS_HLINE(),
            top_left,
            pancurses::ACS_URCORNER(),
            bot_left,
            pancurses::ACS_LRCORNER());

        self.window.mvaddstr(0, 2, self.title.clone());
    }

    /// Erases all content on the window, and redraws the border. Does
    /// not refresh the screen.
    pub fn erase(&self) {
        self.window.erase();
        self.draw_border();
    }

    /// Writes a line of text to the window. Note that this does not do
    /// checking for line length, so strings that are too long will end
    /// up wrapping and may mess up the format. use `write_wrap_line()`
    /// if you need line wrapping.
    pub fn write_line(&self, y: i32, string: String) {
        self.window.mvaddstr(self.abs_y(y), self.abs_x(0), string);
    }

    /// Writes a line of text to the window, first moving all text on
    /// line `y` and below down one row.
    pub fn insert_line(&self, y: i32, string: String) {
        self.window.mv(self.abs_y(y), 0);
        self.window.insertln();
        self.window.mv(self.abs_y(y), self.abs_x(0));
        self.window.addstr(string);
    }

    /// Deletes a line of text from the window.
    pub fn delete_line(&self, y: i32) {
        self.window.mv(self.abs_y(y), self.abs_x(-1));
        self.window.deleteln();
    }

    /// Writes one or more lines of text from a String, word wrapping
    /// when necessary. `start_y` refers to the row to start at (word
    /// wrapping makes it unknown where text will end). Returns the row
    /// on which the text ended.
    pub fn write_wrap_line(&self, start_y: i32, string: String) -> i32 {
        let mut row = start_y;
        let max_row = self.get_rows();
        let wrapper = textwrap::wrap_iter(&string, self.get_cols() as usize);
        for line in wrapper {
            self.window.mvaddstr(self.abs_y(row), self.abs_x(0), line.clone());
            row += 1;

            if row >= max_row {
                break;
            }
        }
        return row-1;
    }

    /// Write the specific template used for the details panel. This is
    /// not the most elegant code, but it works.
    pub fn details_template(&self, start_y: i32, details: Details) {
        let mut row = start_y-1;

        self.window.attron(Attribute::Bold);
        // podcast title
        match details.pod_title {
            Some(t) => row = self.write_wrap_line(row+1, t),
            None => row = self.write_wrap_line(row+1, "No title".to_string()),
        }

        // episode title
        match details.ep_title {
            Some(t) => row = self.write_wrap_line(row+1, t),
            None => row = self.write_wrap_line(row+1, "No title".to_string()),
        }
        self.window.attroff(Attribute::Bold);

        row += 1;  // blank line

        // published date
        if let Some(date) = details.pubdate {
            let new_row = self.write_wrap_line(row+1,
                format!("Published: {}", date.format("%B %-d, %Y").to_string()));
            self.change_attr(row+1, 0, 10,
                pancurses::A_UNDERLINE, ColorType::Normal);
            row = new_row;
        }

        // duration
        if let Some(dur) = details.duration {
            let new_row = self.write_wrap_line(row+1,
                format!("Duration: {}", dur));
            self.change_attr(row+1, 0, 9,
                pancurses::A_UNDERLINE, ColorType::Normal);
            row = new_row;
        }

        // explicit
        if let Some(exp) = details.explicit {
            let new_row = if exp {
                self.write_wrap_line(row+1, "Explicit: Yes".to_string())
            } else {
                self.write_wrap_line(row+1, "Explicit: No".to_string())
            };
            self.change_attr(row+1, 0, 9,
                pancurses::A_UNDERLINE, ColorType::Normal);
            row = new_row;
        }

        row += 1;  // blank line

        // description
        match details.description {
            Some(desc) => {
                self.window.attron(Attribute::Bold);
                row = self.write_wrap_line(row+1, "Description:".to_string());
                self.window.attroff(Attribute::Bold);
                let _row = self.write_wrap_line(row+1, desc);
            },
            None => {
                let _row = self.write_wrap_line(row+1, "No description.".to_string());
            },
        }
    }

    /// Changes the attributes (text style and color) for a line of
    /// text.
    pub fn change_attr(&self, y: i32, x: i32, nchars: i32, attr: pancurses::chtype, color: ColorType) {
        self.window.mvchgat(self.abs_y(y), self.abs_x(x), nchars,
            attr, self.colors.get(color));
    }

    /// Updates window size
    pub fn resize(&mut self, n_row: i32, n_col: i32, start_y: i32, start_x: i32) {
        self.n_row = n_row;
        self.n_col = n_col;

        // apparently pancurses does not implement `wresize()`
        // from ncurses, so instead we create an entirely new
        // window every time the terminal is resized...not ideal,
        // but c'est la vie
        let oldwin = std::mem::replace(
            &mut self.window,
            pancurses::newwin(n_row, n_col, start_y, start_x));
        oldwin.delwin();
    }

    /// Returns the effective number of rows (accounting for borders
    /// and margins).
    pub fn get_rows(&self) -> i32 {
        return self.n_row - 2;  // border on top and bottom
    }

    /// Returns the effective number of columns (accounting for 
    /// borders and margins).
    pub fn get_cols(&self) -> i32 {
        return self.n_col - 5;  // 2 for border, 2 for margins, and 1
                                // extra for some reason...
    }

    /// Calculates the y-value relative to the window rather than to the
    /// panel (i.e., taking into account borders and margins).
    fn abs_y(&self, y: i32) -> i32 {
        return y + 1;
    }

    /// Calculates the x-value relative to the window rather than to the
    /// panel (i.e., taking into account borders and margins).
    fn abs_x(&self, x: i32) -> i32 {
        return x + 2;
    }
}