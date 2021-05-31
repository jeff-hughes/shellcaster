use std::rc::Rc;

use chrono::{DateTime, Utc};
use crossterm::style;

use super::AppColors;

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
    pub buffer: Vec<String>,
    pub screen_pos: usize,
    pub colors: Rc<AppColors>,
    pub title: String,
    pub start_x: u16,
    pub n_row: u16,
    pub n_col: u16,
    pub margins: (u16, u16, u16, u16),
}

impl Panel {
    pub fn new(
        title: String,
        screen_pos: usize,
        colors: Rc<AppColors>,
        n_row: u16,
        n_col: u16,
        start_x: u16,
        margins: (u16, u16, u16, u16),
    ) -> Self {
        // we represent the window as a vector of Strings instead of
        // printing to the terminal buffer
        let buffer = vec![String::new(); (n_row - 2) as usize];

        return Panel {
            buffer: buffer,
            screen_pos: screen_pos,
            colors: colors,
            title: title,
            start_x: start_x,
            n_row: n_row,
            n_col: n_col,
            margins: margins,
        };
    }

    pub fn redraw(&self) {}

    // pub fn clear(&mut self) {
    //     self.clear_inner();
    // }

    pub fn clear_inner(&mut self) {
        self.buffer = vec![String::new(); (self.n_row - 2) as usize];
    }

    pub fn write_line(&mut self, y: u16, string: String, _style: Option<style::ContentStyle>) {
        self.buffer[y as usize] = string;
    }

    pub fn write_key_value_line(
        &mut self,
        y: u16,
        key: String,
        value: String,
        _key_style: Option<style::ContentStyle>,
        _value_style: Option<style::ContentStyle>,
    ) {
        self.buffer[y as usize] = format!("{}: {}", key, value);
    }

    pub fn write_wrap_line(
        &mut self,
        start_y: u16,
        string: &str,
        _style: Option<style::ContentStyle>,
    ) -> u16 {
        let mut row = start_y;
        let max_row = self.get_rows();
        let wrapper = textwrap::wrap(&string, self.get_cols() as usize);
        for line in wrapper {
            self.write_line(row, line.to_string(), None);
            row += 1;

            if row >= max_row {
                break;
            }
        }
        return row - 1;
    }

    pub fn details_template(&mut self, start_y: u16, details: Details) {
        let mut row = start_y - 1;

        // podcast title
        match details.pod_title {
            Some(t) => row = self.write_wrap_line(row + 1, &t, None),
            None => row = self.write_wrap_line(row + 1, "No title", None),
        }

        // episode title
        match details.ep_title {
            Some(t) => row = self.write_wrap_line(row + 1, &t, None),
            None => row = self.write_wrap_line(row + 1, "No title", None),
        }

        row += 1; // blank line

        // published date
        if let Some(date) = details.pubdate {
            self.write_key_value_line(
                row + 1,
                "Published".to_string(),
                format!("{}", date.format("%B %-d, %Y")),
                None,
                None,
            );
            row += 1;
        }

        // duration
        if let Some(dur) = details.duration {
            self.write_key_value_line(row + 1, "Duration".to_string(), dur, None, None);
            row += 1;
        }

        // explicit
        if let Some(exp) = details.explicit {
            let exp_string = if exp {
                "Yes".to_string()
            } else {
                "No".to_string()
            };
            self.write_key_value_line(row + 1, "Explicit".to_string(), exp_string, None, None);
            row += 1;
        }

        row += 1; // blank line

        // description
        match details.description {
            Some(desc) => {
                row = self.write_wrap_line(row + 1, "Description:", None);
                let _row = self.write_wrap_line(row + 1, &desc, None);
            }
            None => {
                let _row = self.write_wrap_line(row + 1, "No description.", None);
            }
        }
    }

    pub fn resize(&mut self, n_row: u16, n_col: u16, start_x: u16) {
        self.n_row = n_row;
        self.n_col = n_col;
        self.start_x = start_x;

        let new_len = (n_row - 2) as usize;
        let len = self.buffer.len();
        if new_len < len {
            self.buffer.truncate(new_len);
        } else if new_len > len {
            for _ in (new_len - len)..new_len {
                self.buffer.push(String::new());
            }
        }
    }

    pub fn get_rows(&self) -> u16 {
        // 2 for border on top and bottom
        return self.n_row - self.margins.0 - self.margins.2 - 2;
    }

    pub fn get_cols(&self) -> u16 {
        // 2 for border, and 1 extra for some reason...
        return self.n_col - self.margins.1 - self.margins.3 - 3;
    }

    pub fn get_row(&self, row: usize) -> String {
        return self.buffer[row].clone();
    }
}
