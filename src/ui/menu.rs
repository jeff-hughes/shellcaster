use std::cmp::min;

use crate::types::*;
use super::ColorType;
use super::Panel;


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
    pub panel: Panel,
    pub items: LockVec<T>,
    pub top_row: i32,  // top row of text shown in window
    pub selected: i32,  // which line of text is highlighted
}

impl<T: Clone + Menuable> Menu<T> {
    /// Prints the list of visible items to the pancurses window and
    /// refreshes it.
    pub fn init(&mut self) {
        self.panel.init();
        self.update_items();
    }

    /// Prints or reprints the list of visible items to the pancurses
    /// window and refreshes it.
    pub fn update_items(&mut self) {
        self.panel.erase();

        let borrow = self.items.borrow();
        if !borrow.is_empty() {
            // update selected item if list has gotten shorter
            let current_selected = self.selected + self.top_row;
            let list_len = borrow.len() as i32;
            if current_selected >= list_len {
                self.selected = self.selected - (current_selected - list_len) - 1;
            }

            // for visible rows, print strings from list
            for i in 0..self.panel.get_rows() {
                let item_idx = (self.top_row + i) as usize;
                if let Some(elem) = borrow.get(item_idx) {
                    self.panel.write_line(i,
                        elem.get_title(self.panel.get_cols() as usize));

                    // this is literally the same logic as
                    // self.set_attrs(), but it's complaining about
                    // immutable borrows, so...
                    let attr = if elem.is_played() {
                        pancurses::A_NORMAL
                    } else {
                        pancurses::A_BOLD
                    };
                    self.panel.change_attr(i, -1, self.panel.get_cols() + 3,
                        attr, ColorType::Normal);
                } else {
                    break;
                }
            }
        }
        self.panel.refresh();
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
            let list_len = self.items.len();
            if list_len == 0 {
                return;
            }

            let n_row = self.panel.get_rows();

            // TODO: currently only handles scroll value of 1; need to extend
            // to be able to scroll multiple lines at a time
            old_selected = self.selected;
            self.selected += lines;

            // don't allow scrolling past last item in list (if shorter
            // than self.panel.get_rows())
            let abs_bottom = min(self.panel.get_rows(),
                (list_len - 1) as i32);
            if self.selected > abs_bottom {
                self.selected = abs_bottom;
            }

            // scroll list if necessary:
            // scroll down
            if self.selected > (n_row - 1) {
                self.selected = n_row - 1;
                if let Some(title) = self.items
                    .map_single((self.top_row + n_row) as usize, 
                        |el| el.get_title(self.panel.get_cols() as usize)) {

                    self.top_row += 1;
                    self.panel.delete_line(0);
                    old_selected -= 1;

                    self.panel.delete_line(n_row-1);
                    self.panel.write_line(n_row-1, title);
                }

            // scroll up
            } else if self.selected < 0 {
                self.selected = 0;
                if let Some(title) = self.items
                    .map_single((self.top_row - 1) as usize,
                        |el| el.get_title(self.panel.get_cols() as usize)) {

                    self.top_row -= 1;
                    self.panel.insert_line(0, title);
                    old_selected += 1;
                }
            }

            old_played = self.items
                .map_single((self.top_row + old_selected) as usize, 
                    |el| el.is_played()).unwrap();
            new_played = self.items
                .map_single((self.top_row + self.selected) as usize,
                    |el| el.is_played()).unwrap();
        }

        self.set_attrs(old_selected, old_played, ColorType::Normal);
        self.set_attrs(self.selected, new_played, ColorType::HighlightedActive);
        self.panel.refresh();
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
        self.panel.change_attr(index, -1, self.panel.get_cols() + 3,
            attr, color);
    }

    /// Highlights the currently selected item in the menu, based on
    /// whether the menu is currently active or not.
    pub fn highlight_selected(&mut self, active_menu: bool) {
        let is_played = self.items
            .map_single((self.top_row + self.selected) as usize,
            |el| el.is_played());

        if let Some(played) = is_played {
            if active_menu {
                self.set_attrs(self.selected, played, ColorType::HighlightedActive);
            } else {
                self.set_attrs(self.selected, played, ColorType::Highlighted);
            }
            self.panel.refresh();
        }
    }

    /// Controls how the window changes when it is active (i.e., available
    /// for user input to modify state).
    pub fn activate(&mut self) {
        // if list is empty, will return None
        if let Some(played) = self.items
            .map_single((self.top_row + self.selected) as usize,
            |el| el.is_played()) {

            self.set_attrs(self.selected, played, ColorType::HighlightedActive);
            self.panel.refresh();
        }
    }

    /// Updates window size
    pub fn resize(&mut self, n_row: i32, n_col: i32, start_y: i32, start_x: i32) {
        self.panel.resize(n_row, n_col, start_y, start_x);
        let n_row = self.panel.get_rows();

        // if resizing moves selected item off screen, scroll the list
        // upwards to keep same item selected
        if self.selected > (n_row - 1) {
            self.top_row = self.top_row + self.selected - (n_row - 1);
            self.selected = n_row - 1;
        }
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
        // if list is empty, will return None
        if let Some(played) = self.items
            .map_single((self.top_row + self.selected) as usize,
            |el| el.is_played()) {

            self.set_attrs(self.selected, played, ColorType::Highlighted);
            self.panel.refresh();
        }
    }
}

impl Menu<Episode> {
    /// Controls how the window changes when it is inactive (i.e., not
    /// available for user input to modify state).
    pub fn deactivate(&mut self) {
        // if list is empty, will return None
        if let Some(played) = self.items
            .map_single((self.top_row + self.selected) as usize,
            |el| el.is_played()) {

            self.set_attrs(self.selected, played, ColorType::Normal);
            self.panel.refresh();
        }
    }
}


// TESTS -----------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_menu(n_row: i32, n_col: i32, top_row: i32, selected: i32) -> Menu<Episode> {
        let titles = vec![
            "A Very Cool Episode",
            "This is a very long episode title but we'll get through it together",
            "An episode with le Unicodé",
            "How does an episode with emoji sound? 😉",
            "Here's another title",
            "Un titre, c'est moi!",
            "One more just for good measure"
        ];
        let mut items = Vec::new();
        for (i, t) in titles.iter().enumerate() {
            let played = i % 2 == 0;
            items.push(Episode {
                id: None,
                pod_id: None,
                title: t.to_string(),
                url: String::new(),
                description: String::new(),
                pubdate: Some(Utc::now()),
                duration: Some(12345),
                path: None,
                played: played,
            });
        }

        let panel = Panel::new(
            crate::ui::colors::set_colors(),
            "Episodes".to_string(),
            1,
            n_row, n_col,
            0, 0
        );
        return Menu {
            panel: panel,
            items: LockVec::new(items),
            top_row: top_row,
            selected: selected,
        };
    }

    #[test]
    fn scroll_up() {
        let real_rows = 5;
        let real_cols = 65;
        let mut menu = create_menu(real_rows+2, real_cols+5, 2, 0);
        menu.update_items();

        menu.scroll(-1);

        let borrow = menu.items.borrow();
        let expected_top = borrow[1].get_title(real_cols as usize);
        let expected_bot = borrow[5].get_title(real_cols as usize);

        assert_eq!(menu.panel.get_row(0).0, expected_top);
        assert_eq!(menu.panel.get_row(4).0, expected_bot);
    }

    #[test]
    fn scroll_down() {
        let real_rows = 5;
        let real_cols = 65;
        let mut menu = create_menu(real_rows+2, real_cols+5, 0, 4);
        menu.update_items();

        menu.scroll(1);

        let borrow = menu.items.borrow();
        let expected_top = borrow[1].get_title(real_cols as usize);
        let expected_bot = borrow[5].get_title(real_cols as usize);

        assert_eq!(menu.panel.get_row(0).0, expected_top);
        assert_eq!(menu.panel.get_row(4).0, expected_bot);
    }

    #[test]
    fn resize_bigger() {
        let real_rows = 5;
        let real_cols = 65;
        let mut menu = create_menu(real_rows+2, real_cols+5, 0, 4);
        menu.update_items();

        menu.resize(real_rows+2+5, real_cols+5+5, 0, 0);
        menu.update_items();

        assert_eq!(menu.top_row, 0);
        assert_eq!(menu.selected, 4);

        let non_empty: Vec<String> = menu.panel.window.iter()
            .filter_map(|x| if x.0.is_empty() {
                    None
                } else {
                    Some(x.0.clone())
                }).collect();
        assert_eq!(non_empty.len(), menu.items.len());
    }

    #[test]
    fn resize_smaller() {
        let real_rows = 7;
        let real_cols = 65;
        let mut menu = create_menu(real_rows+2, real_cols+5, 0, 6);
        menu.update_items();

        menu.resize(real_rows+2-2, real_cols+5-5, 0, 0);
        menu.update_items();

        assert_eq!(menu.top_row, 2);
        assert_eq!(menu.selected, 4);

        let non_empty: Vec<String> = menu.panel.window.iter()
            .filter_map(|x| if x.0.is_empty() {
                    None
                } else {
                    Some(x.0.clone())
                }).collect();
        assert_eq!(non_empty.len(), (real_rows-2) as usize);
    }

    #[test]
    fn chop_accent() {
        let real_rows = 5;
        let real_cols = 25;
        let mut menu = create_menu(real_rows+2, real_cols+5, 0, 0);
        menu.update_items();

        let expected = "An episode with le Unicod".to_string();

        assert_eq!(menu.panel.get_row(2).0, expected);
    }

    #[test]
    fn chop_emoji() {
        let real_rows = 5;
        let real_cols = 38;
        let mut menu = create_menu(real_rows+2, real_cols+5, 0, 0);
        menu.update_items();

        let expected = "How does an episode with emoji sound? ".to_string();

        assert_eq!(menu.panel.get_row(3).0, expected);
    }
}