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

#[derive(Debug)]
pub struct Panel {
    pub window: Vec<(String, u32, ColorType)>,
    pub screen_pos: usize,
    pub colors: Colors,
    pub title: String,
    pub n_row: i32,
    pub n_col: i32,
}

impl Panel {
    pub fn new(colors: Colors,
        title: String, screen_pos: usize, n_row: i32, n_col: i32, _start_y: i32, _start_x: i32) -> Self {

        // we represent the window as a vector of Strings instead of
        // the pancurses window
        let panel_win = vec![
            (String::new(), pancurses::A_NORMAL, ColorType::Normal);
            (n_row-2) as usize];

        return Panel {
            window: panel_win,
            screen_pos: screen_pos,
            colors: colors,
            title: title,
            n_row: n_row,
            n_col: n_col,
        };
    }

    pub fn init(&self) {}

    pub fn refresh(&self) {}

    pub fn erase(&mut self) {
        self.window = vec![
            (String::new(), pancurses::A_NORMAL, ColorType::Normal);
            self.n_row as usize];
    }

    pub fn write_line(&mut self, y: i32, string: String) {
        self.window[y as usize] = (string, pancurses::A_NORMAL, ColorType::Normal);
    }

    pub fn insert_line(&mut self, y: i32, string: String) {
        self.window.insert(y as usize,
            (string, pancurses::A_NORMAL, ColorType::Normal));
        let _ = self.window.pop();
    }

    pub fn delete_line(&mut self, y: i32) {
        let _ = self.window.remove(y as usize);
        // add a new empty line to the end so the vector stays the
        // same size
        self.window.push((String::new(), pancurses::A_NORMAL, ColorType::Normal));
    }

    pub fn write_wrap_line(&mut self, start_y: i32, string: String) -> i32 {
        let mut row = start_y;
        let max_row = self.get_rows();
        let wrapper = textwrap::wrap_iter(&string, self.get_cols() as usize);
        for line in wrapper {
            self.write_line(row, line.to_string());
            row += 1;

            if row >= max_row {
                break;
            }
        }
        return row-1;
    }

    pub fn details_template(&mut self, start_y: i32, details: Details) {
        let mut row = start_y-1;

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
                row = self.write_wrap_line(row+1, "Description:".to_string());
                let _row = self.write_wrap_line(row+1, desc);
            },
            None => {
                let _row = self.write_wrap_line(row+1, "No description.".to_string());
            },
        }
    }

    // This doesn't fully replicate the functionality of Panel, as it
    // only applies the attribute to the line as a whole, rather than
    // specific characters. But I'm primarily using it to change whole
    // lines anyway.
    pub fn change_attr(&mut self, y: i32, _x: i32, _nchars: i32, attr: u32, color: ColorType) {
        let current = &self.window[y as usize];
        self.window[y as usize] = (current.0.clone(), attr, color);
    }

    pub fn resize(&mut self, n_row: i32, n_col: i32, _start_y: i32, _start_x: i32) {
        self.n_row = n_row;
        self.n_col = n_col;

        let new_len = (n_row-2) as usize;
        let len = self.window.len();
        if new_len < len {
            self.window.truncate(new_len);
        } else if new_len > len {
            for _ in (new_len - len)..new_len {
                self.window.push((String::new(), pancurses::A_NORMAL, ColorType::Normal));
            }
        }
    }

    pub fn get_rows(&self) -> i32 {
        return self.n_row - 2;  // border on top and bottom
    }

    pub fn get_cols(&self) -> i32 {
        return self.n_col - 5;  // 2 for border, 2 for margins, and 1
                                // extra for some reason...
    }

    pub fn get_row(&self, row: usize) -> (String, u32, ColorType) {
        return self.window[row].clone();
    }
}