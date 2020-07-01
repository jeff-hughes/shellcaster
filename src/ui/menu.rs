use std::cmp::min;

use pancurses::Window;
use crate::types::*;
use super::{Colors, ColorType};

/// Generic struct holding details about a list menu. These menus are
/// contained by the UI, and hold the list of podcasts or podcast
/// episodes. They also hold the pancurses window used to display the menu
/// to the user.
///
/// * `screen_pos` stores the position of the window on the screen, from
///   left to right 
/// * `n_row` and `n_col` store the size of the `window`
/// * `top_row` indicates the top line of text that is shown on screen
///   (since the list of items can be longer than the available size of
///   the screen). `top_row` is calculated relative to the `items` index,
///   i.e., it will be a value between 0 and items.len()
/// * `selected` indicates which item on screen is currently highlighted.
///   It is calculated relative to the screen itself, i.e., a value between
///   0 and (n_row - 1)
#[derive(Debug)]
pub struct Menu<T>
    where T: Clone + Menuable {
    pub window: Window,
    pub screen_pos: usize,
    pub colors: Colors,
    pub title: String,
    pub items: LockVec<T>,
    pub n_row: i32,
    pub n_col: i32,
    pub top_row: i32,  // top row of text shown in window
    pub selected: i32,  // which line of text is highlighted
}

impl<T: Clone + Menuable> Menu<T> {
    /// Prints the list of visible items to the pancurses window and
    /// refreshes it.
    pub fn init(&mut self) {
        self.draw_border();
        self.update_items();
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

    /// Prints or reprints the list of visible items to the pancurses
    /// window and refreshes it.
    pub fn update_items(&mut self) {
        self.window.erase();
        self.draw_border();

        let borrow = self.items.borrow();
        if !borrow.is_empty() {
            // update selected item if list has gotten shorter
            let current_selected = self.selected + self.top_row;
            let list_len = borrow.len() as i32;
            if current_selected >= list_len {
                self.selected = self.selected - (current_selected - list_len) - 1;
            }

            // for visible rows, print strings from list
            for i in 0..self.n_row {
                let item_idx = (self.top_row + i) as usize;
                if let Some(elem) = borrow.get(item_idx) {
                    self.window.mvaddstr(self.abs_y(i), self.abs_x(0),
                        elem.get_title(self.n_col as usize));

                    // this is literally the same logic as
                    // self.set_attrs(), but it's complaining about
                    // immutable borrows, so...
                    let attr = if elem.is_played() {
                        pancurses::A_NORMAL
                    } else {
                        pancurses::A_BOLD
                    };
                    self.window.mvchgat(self.abs_y(i), self.abs_x(-1),
                        self.n_col + 3,
                        attr,
                        self.colors.get(ColorType::Normal));
                } else {
                    break;
                }
            }
        }
        self.window.refresh();
    }

    /// Scrolls the menu up or down by `lines` lines. Negative values of
    /// `lines` will scroll the menu up.
    /// 
    /// This function examines the new selected value, ensures it does
    /// not fall out of bounds, and then updates the pancurses window to
    /// represent the new visible list.
    pub fn scroll(&mut self, lines: i32) {
        let mut old_selected;
        let old_played;
        let new_played;
        
        {
            let borrow = self.items.borrow();
            if borrow.is_empty() {
                return;
            }

            // TODO: currently only handles scroll value of 1; need to extend
            // to be able to scroll multiple lines at a time
            old_selected = self.selected;
            self.selected += lines;

            // don't allow scrolling past last item in list (if shorter than
            // self.n_row)
            let abs_bottom = min(self.n_row,
                (borrow.len() - 1) as i32);
            if self.selected > abs_bottom {
                self.selected = abs_bottom;
            }

            // scroll list if necessary:
            // scroll down
            if self.selected > (self.n_row - 1) {
                self.selected = self.n_row - 1;
                if let Some(elem) = borrow.get((self.top_row + self.n_row) as usize) {
                    self.top_row += 1;
                    self.window.mv(self.abs_y(0), self.abs_x(0));
                    self.window.deleteln();
                    old_selected -= 1;

                    self.window.mv(self.abs_y(self.n_row-1), self.abs_x(-1));
                    self.window.clrtobot();
                    self.window.mvaddstr(self.abs_y(self.n_row-1), self.abs_x(0), elem.get_title(self.n_col as usize));

                    self.draw_border();
                }

            // scroll up
            } else if self.selected < 0 {
                self.selected = 0;
                if let Some(elem) = borrow.get((self.top_row - 1) as usize) {
                    self.top_row -= 1;
                    self.window.mv(self.abs_y(0), 0);
                    self.window.insertln();
                    old_selected += 1;

                    self.window.mv(self.abs_y(0), self.abs_x(0));
                    self.window.addstr(elem.get_title(self.n_col as usize));

                    self.draw_border();
                }
            }

            old_played = borrow.get((self.top_row + old_selected) as usize).unwrap().is_played();
            new_played = borrow.get((self.top_row + self.selected) as usize).unwrap().is_played();
        }

        self.set_attrs(old_selected, old_played, ColorType::Normal);
        self.set_attrs(self.selected, new_played, ColorType::HighlightedActive);
        self.window.refresh();
    }

    /// Sets font style and color of menu item. `index` is the position
    /// of the menu item to be changed. `played` is an indicator of
    /// whether that item has been played or not. `color` is a ColorType
    /// representing the appropriate state of the item (e.g., Normal,
    /// Highlighted).
    pub fn set_attrs(&mut self, index: i32, played: bool, color: ColorType) {
        let attr = if played {
            pancurses::A_NORMAL
        } else {
            pancurses::A_BOLD
        };
        self.window.mvchgat(self.abs_y(index), self.abs_x(-1),
            self.n_col + 3,
            attr,
            self.colors.get(color));
    }

    /// Highlights the currently selected item in the menu, based on
    /// whether the menu is currently active or not.
    pub fn highlight_selected(&mut self, active_menu: bool) {
        let mut is_played = None;
        {
            let borrow = self.items.borrow();
            let selected = borrow.get((self.top_row + self.selected) as usize);
    
            if let Some(el) = selected {
                is_played = Some(el.is_played());
            }
        }

        if let Some(played) = is_played {
            if active_menu {
                self.set_attrs(self.selected, played, ColorType::HighlightedActive);
            } else {
                self.set_attrs(self.selected, played, ColorType::Highlighted);
            }
            self.window.refresh();
        }
    }

    /// Controls how the window changes when it is active (i.e., available
    /// for user input to modify state).
    pub fn activate(&mut self) {
        let played;
        {
            let borrow = self.items.borrow();
            if borrow.is_empty() {
                return;
            }
                played = borrow.get(self.selected as usize).unwrap().is_played();
        }
        self.set_attrs(self.selected, played, ColorType::HighlightedActive);
        self.window.refresh();
    }

    /// Updates window size
    pub fn resize(&mut self, n_row: i32, n_col: i32) {
        self.n_row = n_row;
        self.n_col = n_col;

        // if resizing moves selected item off screen, scroll the list
        // upwards to keep same item selected
        if self.selected > (self.n_row - 1) {
            self.top_row = self.top_row + self.selected - (self.n_row - 1);
            self.selected = self.n_row - 1;
        }
    }

    /// Calculates the y-value relative to the window rather than to the
    /// menu (i.e., taking into account borders and margins).
    fn abs_y(&self, y: i32) -> i32 {
        return y + 1;
    }

    /// Calculates the x-value relative to the window rather than to the
    /// menu (i.e., taking into account borders and margins).
    fn abs_x(&self, x: i32) -> i32 {
        return x + 2;
    }
}


impl Menu<Podcast> {
    /// Returns a cloned reference to the list of episodes from the
    /// currently selected podcast.
    pub fn get_episodes(&self) -> LockVec<Episode> {
        let index = self.selected + self.top_row;
        return self.items.borrow()
            .get(index as usize).unwrap().episodes.clone();
    }

    /// Controls how the window changes when it is inactive (i.e., not
    /// available for user input to modify state).
    pub fn deactivate(&mut self) {
        let played;
        {
            let borrow = self.items.borrow();
            if borrow.is_empty() {
                return;
            }
                played = borrow.get(self.selected as usize).unwrap().is_played();
        }
        self.set_attrs(self.selected, played, ColorType::Highlighted);
        self.window.refresh();
    }
}

impl Menu<Episode> {
    /// Controls how the window changes when it is inactive (i.e., not
    /// available for user input to modify state).
    pub fn deactivate(&mut self) {
        if !self.items.borrow().is_empty() {
            let played = self.items.borrow().get(self.selected as usize).unwrap().is_played();
            self.set_attrs(self.selected, played, ColorType::Normal);
        }
        self.window.refresh();
    }
}