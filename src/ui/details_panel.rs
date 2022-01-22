use std::rc::Rc;

use chrono::{DateTime, Utc};
use crossterm::style;

use super::panel::Panel;
use super::AppColors;
use super::Scroll;

/// Used to hold one line of content used in the details panel.
#[derive(Debug)]
pub enum DetailsLine {
    Blank,
    Line(String, Option<style::ContentStyle>),
    KeyValueLine(
        (String, Option<style::ContentStyle>),
        (String, Option<style::ContentStyle>),
    ),
}


/// Struct holding the raw data used for building the details panel.
#[derive(Debug)]
pub struct Details {
    pub pod_title: Option<String>,
    pub ep_title: Option<String>,
    pub pubdate: Option<DateTime<Utc>>,
    pub duration: Option<String>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct DetailsPanel {
    pub panel: Panel,
    pub details: Option<Details>,
    pub content: Vec<DetailsLine>,
    pub top_row: u16,    // top row of text shown in window
    pub total_rows: u16, // the total number of rows the details take up
}

impl DetailsPanel {
    /// Creates a new details panel.
    pub fn new(
        title: String,
        screen_pos: usize,
        colors: Rc<AppColors>,
        n_row: u16,
        n_col: u16,
        start_x: u16,
        margins: (u16, u16, u16, u16),
    ) -> Self {
        let panel = Panel::new(title, screen_pos, colors, n_row, n_col, start_x, margins);
        return Self {
            panel: panel,
            details: None,
            content: Vec::new(),
            top_row: 0,
            total_rows: 0,
        };
    }

    /// Redraws borders and refreshes the window to display on terminal.
    pub fn redraw(&self) {
        self.panel.redraw();
    }

    /// Insert new details into the details pane.
    pub fn change_details(&mut self, details: Details) {
        self.top_row = 0;
        self.details = Some(details);
        self.stringify_content();
        self.redraw();
        self.write_details();
    }

    /// Updates window size.
    pub fn resize(&mut self, n_row: u16, n_col: u16, start_x: u16) {
        self.panel.resize(n_row, n_col, start_x);
        self.stringify_content();
        self.redraw();
        self.write_details();
    }

    /// Scrolls the details panel up or down by `lines` lines.
    ///
    /// This function examines the new selected value, ensures it does
    /// not fall out of bounds, and then updates the panel to
    /// represent the new visible list.
    pub fn scroll(&mut self, lines: Scroll) {
        if self.content.is_empty() {
            return;
        }
        let total_rows = self.content.len() as u16;
        let old_top_row = self.top_row;

        match lines {
            Scroll::Up(v) => {
                if let Some(top) = self.top_row.checked_sub(v) {
                    self.top_row = top;
                } else {
                    self.top_row = 0;
                }
                if self.top_row != old_top_row {
                    self.panel.clear_inner();
                    // self.details_template(self.top_row);
                    self.write_details();
                }
            }
            Scroll::Down(v) => {
                let n_row = self.panel.get_rows();
                // can't scroll if details are shorter than screen
                if total_rows <= n_row {
                    return;
                }
                let move_dist = std::cmp::min(v, total_rows - self.top_row - n_row);
                self.top_row += move_dist;
                if self.top_row != old_top_row {
                    self.panel.clear_inner();
                    // self.details_template(self.top_row);
                    self.write_details();
                }
            }
        }
    }

    /// Format the details content to fit the panel as currently sized
    /// and save it as Strings. This needs to be done to allow the
    /// content to be scrollable.
    fn stringify_content(&mut self) {
        if let Some(details) = &self.details {
            let num_cols = self.panel.get_cols() as usize;
            let bold = style::ContentStyle::new()
                .foreground(self.panel.colors.bold.0)
                .background(self.panel.colors.bold.1)
                .attribute(style::Attribute::Bold);
            let underlined = style::ContentStyle::new()
                .foreground(self.panel.colors.normal.0)
                .background(self.panel.colors.normal.1)
                .attribute(style::Attribute::Underlined);

            self.content.clear();

            // podcast title
            let text = match &details.pod_title {
                Some(t) => t,
                None => "No title",
            };
            let wrapper = textwrap::wrap(text, num_cols);
            for line in wrapper {
                self.content
                    .push(DetailsLine::Line(line.to_string(), Some(bold)));
            }

            // episode title
            let text = match &details.ep_title {
                Some(t) => t,
                None => "No title",
            };
            let wrapper = textwrap::wrap(text, num_cols);
            for line in wrapper {
                self.content
                    .push(DetailsLine::Line(line.to_string(), Some(bold)));
            }

            self.content.push(DetailsLine::Blank); // blank line

            // published date
            if let Some(date) = details.pubdate {
                self.content.push(DetailsLine::KeyValueLine(
                    ("Published".to_string(), Some(underlined)),
                    (format!("{}", date.format("%B %-d, %Y")), None),
                ));
            }

            // duration
            if let Some(dur) = &details.duration {
                self.content.push(DetailsLine::KeyValueLine(
                    ("Duration".to_string(), Some(underlined)),
                    (dur.clone(), None),
                ));
            }

            // explicit
            if let Some(exp) = details.explicit {
                let exp_string = if exp {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                };
                self.content.push(DetailsLine::KeyValueLine(
                    ("Explicit".to_string(), Some(underlined)),
                    (exp_string, None),
                ));
            }

            self.content.push(DetailsLine::Blank); // blank line

            // description
            match &details.description {
                Some(desc) => {
                    let wrapper = textwrap::wrap("Description:", num_cols);
                    for line in wrapper {
                        self.content
                            .push(DetailsLine::Line(line.to_string(), Some(bold)));
                    }
                    let wrapper = textwrap::wrap(desc, num_cols);
                    for line in wrapper {
                        self.content.push(DetailsLine::Line(line.to_string(), None));
                    }
                }
                None => {
                    let wrapper = textwrap::wrap("No description.", num_cols);
                    for line in wrapper {
                        self.content.push(DetailsLine::Line(line.to_string(), None));
                    }
                }
            }
        }
    }

    /// Write the details content to the screen.
    pub fn write_details(&mut self) {
        if !self.content.is_empty() {
            let mut row = 0;
            for line in self.content.iter().skip(self.top_row as usize) {
                match line {
                    DetailsLine::Blank => row += 1,
                    DetailsLine::Line(text, style) => {
                        row = self.panel.write_wrap_line(row, text, *style);
                        row += 1;
                    }
                    DetailsLine::KeyValueLine((key, key_style), (val, val_style)) => {
                        self.panel.write_key_value_line(
                            row,
                            key.clone(),
                            val.clone(),
                            *key_style,
                            *val_style,
                        );
                        row += 1;
                    }
                }
            }
        }
    }
}
