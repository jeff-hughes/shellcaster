use std::{convert::TryInto, io};

use chrono::{DateTime, Utc};
use crossterm::{cursor, queue, style};

use super::ColorType;


pub const VERTICAL: &str = "│";
pub const HORIZONTAL: &str = "─";
pub const TOP_RIGHT: &str = "┐";
pub const TOP_LEFT: &str = "┌";
pub const BOTTOM_RIGHT: &str = "┘";
pub const BOTTOM_LEFT: &str = "└";
pub const TOP_TEE: &str = "┬";
pub const BOTTOM_TEE: &str = "┴";


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
    screen_pos: usize,
    title: String,
    start_x: u16,
    n_row: u16,
    n_col: u16,
}

impl Panel {
    /// Creates a new panel.
    pub fn new(title: String, screen_pos: usize, n_row: u16, n_col: u16, start_x: u16) -> Self {
        return Panel {
            screen_pos: screen_pos,
            title: title,
            start_x: start_x,
            n_row: n_row,
            n_col: n_col,
        };
    }

    /// Redraws borders and refreshes the window to display on terminal.
    pub fn redraw(&self) {
        // clear the panel
        // TODO: Set the background color first
        let empty = vec![" "; self.n_col as usize];
        let empty_string = empty.join("");
        for r in 0..(self.n_row - 1) {
            queue!(
                io::stdout(),
                cursor::MoveTo(self.start_x, r),
                style::Print(&empty_string),
            )
            .unwrap();
        }
        self.draw_border();
    }

    /// Draws a border around the window.
    fn draw_border(&self) {
        let top_left;
        let bot_left;
        match self.screen_pos {
            0 => {
                top_left = TOP_LEFT;
                bot_left = BOTTOM_LEFT;
            }
            _ => {
                top_left = TOP_TEE;
                bot_left = BOTTOM_TEE;
            }
        }
        let mut border_top = vec![top_left];
        let mut border_bottom = vec![bot_left];
        for _ in 0..(self.n_col - 2) {
            border_top.push(HORIZONTAL);
            border_bottom.push(HORIZONTAL);
        }
        border_top.push(TOP_RIGHT);
        border_bottom.push(BOTTOM_RIGHT);

        queue!(
            io::stdout(),
            cursor::MoveTo(self.start_x, 0),
            style::Print(border_top.join("")),
            cursor::MoveTo(self.start_x, self.n_row - 1),
            style::Print(border_bottom.join("")),
        )
        .unwrap();

        for r in 1..(self.n_row - 1) {
            queue!(
                io::stdout(),
                cursor::MoveTo(self.start_x, r),
                style::Print(VERTICAL.to_string()),
                cursor::MoveTo(self.start_x + self.n_col - 1, r),
                style::Print(VERTICAL.to_string()),
            )
            .unwrap();
        }

        queue!(
            io::stdout(),
            cursor::MoveTo(self.start_x + 2, 0),
            style::Print(&self.title),
        )
        .unwrap();
    }

    /// Writes a line of text to the window. Note that this does not do
    /// checking for line length, so strings that are too long will end
    /// up wrapping and may mess up the format. use `write_wrap_line()`
    /// if you need line wrapping.
    pub fn write_line(&self, y: u16, string: String) {
        queue!(
            io::stdout(),
            cursor::MoveTo(self.abs_x(0), self.abs_y(y as i16)),
            style::Print(string)
        )
        .unwrap();
    }

    /// Writes one or more lines of text from a String, word wrapping
    /// when necessary. `start_y` refers to the row to start at (word
    /// wrapping makes it unknown where text will end). Returns the row
    /// on which the text ended.
    pub fn write_wrap_line(&self, start_y: u16, string: &str) -> u16 {
        // let mut row = start_y;
        // let max_row = self.get_rows();
        // let wrapper = textwrap::wrap(string, self.get_cols() as usize);
        // for line in wrapper {
        //     self.window.mvaddstr(self.abs_y(row), self.abs_x(0), line);
        //     row += 1;

        //     if row >= max_row {
        //         break;
        //     }
        // }
        // return row - 1;
        return 0;
    }

    /// Write the specific template used for the details panel. This is
    /// not the most elegant code, but it works.
    pub fn details_template(&self, start_y: u16, details: Details) {
        // let mut row = start_y - 1;

        // self.window.attron(Attribute::Bold);
        // // podcast title
        // match details.pod_title {
        //     Some(t) => row = self.write_wrap_line(row + 1, &t),
        //     None => row = self.write_wrap_line(row + 1, "No title"),
        // }

        // // episode title
        // match details.ep_title {
        //     Some(t) => row = self.write_wrap_line(row + 1, &t),
        //     None => row = self.write_wrap_line(row + 1, "No title"),
        // }
        // self.window.attroff(Attribute::Bold);

        // row += 1; // blank line

        // // published date
        // if let Some(date) = details.pubdate {
        //     let new_row = self.write_wrap_line(
        //         row + 1,
        //         &format!("Published: {}", date.format("%B %-d, %Y")),
        //     );
        //     self.change_attr(row + 1, 0, 10, pancurses::A_UNDERLINE, ColorType::Normal);
        //     row = new_row;
        // }

        // // duration
        // if let Some(dur) = details.duration {
        //     let new_row = self.write_wrap_line(row + 1, &format!("Duration: {}", dur));
        //     self.change_attr(row + 1, 0, 9, pancurses::A_UNDERLINE, ColorType::Normal);
        //     row = new_row;
        // }

        // // explicit
        // if let Some(exp) = details.explicit {
        //     let new_row = if exp {
        //         self.write_wrap_line(row + 1, "Explicit: Yes")
        //     } else {
        //         self.write_wrap_line(row + 1, "Explicit: No")
        //     };
        //     self.change_attr(row + 1, 0, 9, pancurses::A_UNDERLINE, ColorType::Normal);
        //     row = new_row;
        // }

        // row += 1; // blank line

        // // description
        // match details.description {
        //     Some(desc) => {
        //         self.window.attron(Attribute::Bold);
        //         row = self.write_wrap_line(row + 1, "Description:");
        //         self.window.attroff(Attribute::Bold);
        //         let _row = self.write_wrap_line(row + 1, &desc);
        //     }
        //     None => {
        //         let _row = self.write_wrap_line(row + 1, "No description.");
        //     }
        // }
    }

    /// Changes the attributes (text style and color) for a line of
    /// text.
    pub fn change_attr(
        &self,
        y: i16,
        x: i16,
        nchars: u16,
        attr: pancurses::chtype,
        color: ColorType,
    ) {
        // self.window
        //     .mvchgat(self.abs_y(y), self.abs_x(x), nchars, attr, color as i16);
    }

    /// Updates window size
    pub fn resize(&mut self, n_row: u16, n_col: u16, start_y: u16, start_x: u16) {
        // self.n_row = n_row;
        // self.n_col = n_col;

        // // apparently pancurses does not implement `wresize()`
        // // from ncurses, so instead we create an entirely new
        // // window every time the terminal is resized...not ideal,
        // // but c'est la vie
        // let oldwin = std::mem::replace(
        //     &mut self.window,
        //     pancurses::newwin(n_row, n_col, start_y, start_x),
        // );
        // oldwin.delwin();
    }

    /// Returns the effective number of rows (accounting for borders
    /// and margins).
    pub fn get_rows(&self) -> u16 {
        return self.n_row - 2; // border on top and bottom
    }

    /// Returns the effective number of columns (accounting for
    /// borders and margins).
    pub fn get_cols(&self) -> u16 {
        return self.n_col - 5; // 2 for border, 2 for margins, and 1
                               // extra for some reason...
    }

    /// Calculates the y-value relative to the terminal rather than to
    /// the panel (i.e., taking into account borders and margins).
    fn abs_y(&self, y: i16) -> u16 {
        return (y + 1)
            .try_into()
            .expect("Can't convert signed integer to unsigned");
    }

    /// Calculates the x-value relative to the terminal rather than to
    /// the panel (i.e., taking into account borders and margins).
    fn abs_x(&self, x: i16) -> u16 {
        return (x + self.start_x as i16 + 2)
            .try_into()
            .expect("Can't convert signed integer to unsigned");
    }
}
