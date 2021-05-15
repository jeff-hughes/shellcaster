use std::cmp::max;
use std::cmp::min;
use std::collections::hash_map::Entry;

use super::ColorType;
use super::Panel;
use crate::types::*;

/// Holds a value for how much to scroll the menu up or down, without
/// having to deal with positive/negative values.
pub enum Scroll {
    Up(u16),
    Down(u16),
}

/// Generic struct holding details about a list menu. These menus are
/// contained by the UI, and hold the list of podcasts or podcast
/// episodes. They also hold the pancurses window used to display the menu
/// to the user.
///
/// * `screen_pos` stores the position of the window on the screen, from
///   left to right
/// * `start_row` indicates the first row that is used for the menu;
///   this will be 0 if there is no header; otherwise, `start_row` will
///   be the first row below the header. Calculated relative to the
///   panel, i.e., a value between 0 and (n_row - 1)
/// * `top_row` indicates the top line of text that is shown on screen
///   (since the list of items can be longer than the available size of
///   the screen). `top_row` is calculated relative to the `items` index,
///   i.e., it will be a value between 0 and items.len()
/// * `selected` indicates which item on screen is currently highlighted.
///   It is calculated relative to the panel, i.e., a value between
///   0 and (n_row - 1)
#[derive(Debug)]
pub struct Menu<T>
where T: Clone + Menuable
{
    pub panel: Panel,
    pub header: Option<String>,
    pub items: LockVec<T>,
    pub start_row: u16, // beginning of first row of menu
    pub top_row: u16,   // top row of text shown in window
    pub selected: u16,  // which line of text is highlighted
}

impl<T: Clone + Menuable> Menu<T> {
    /// Creates a new menu.
    pub fn new(panel: Panel, header: Option<String>, items: LockVec<T>) -> Self {
        return Self {
            panel: panel,
            header: header,
            items: items,
            start_row: 0,
            top_row: 0,
            selected: 0,
        };
    }

    /// Clears the terminal, and then prints the list of visible items
    /// to the terminal.
    pub fn redraw(&mut self) {
        self.panel.redraw();
        self.update_items();
    }

    /// Prints the list of visible items to the terminal.
    pub fn update_items(&mut self) {
        // self.start_row = self.print_header();
        // if self.selected < self.start_row {
        //     self.selected = self.start_row;
        // }

        // let (map, order) = self.items.borrow();
        // if !order.is_empty() {
        //     // update selected item if list has gotten shorter
        //     let current_selected = self.get_menu_idx(self.selected);
        //     let list_len = order.len();
        //     if current_selected >= list_len {
        //         self.selected = (self.selected as usize - (current_selected - list_len) - 1) as u16;
        //     }

        //     // for visible rows, print strings from list
        //     for i in self.start_row..self.panel.get_rows() {
        //         if let Some(elem_id) = order.get(self.get_menu_idx(i)) {
        //             let elem = map.get(&elem_id).expect("Could not retrieve menu item.");
        //             self.panel
        //                 .write_line(i, elem.get_title(self.panel.get_cols() as usize));

        //             // this is literally the same logic as
        //             // self.set_attrs(), but it's complaining about
        //             // immutable borrows, so...
        //             let attr = if elem.is_played() {
        //                 pancurses::A_NORMAL
        //             } else {
        //                 pancurses::A_BOLD
        //             };
        //             self.panel.change_attr(
        //                 i as i16,
        //                 -1,
        //                 self.panel.get_cols() + 3,
        //                 attr,
        //                 ColorType::Normal,
        //             );
        //         } else {
        //             break;
        //         }
        //     }
        // }
    }

    /// If a header exists, prints lines of text to the panel to appear
    /// above the menu.
    fn print_header(&mut self) -> u16 {
        if let Some(header) = &self.header {
            return self.panel.write_wrap_line(0, header) + 2;
        } else {
            return 0;
        }
    }

    /// Scrolls the menu up or down by `lines` lines.
    ///
    /// This function examines the new selected value, ensures it does
    /// not fall out of bounds, and then updates the panel to
    /// represent the new visible list.
    pub fn scroll(&mut self, lines: Scroll) {
        match lines {
            Scroll::Up(v) => {
                if v <= self.selected {
                    self.selected -= v;
                } else {
                    let list_scroll_amount = v - self.selected;
                    self.top_row = min(0, self.top_row - list_scroll_amount);
                    self.selected = 0;
                }
            }
            Scroll::Down(v) => {
                let n_row = self.panel.get_rows();
                if v < (n_row - self.selected) {
                    self.selected += v;
                } else {
                    let list_len = self.items.len() as u16;
                    let list_scroll_amount = v - (n_row - self.selected - 1);
                    self.top_row = max(list_scroll_amount, list_len - n_row);
                    self.selected = n_row - 1;
                }
            }
        }

        // let mut old_selected;
        // let checked_lines;
        // let apply_color_played;
        // let get_titles;

        // let list_len = self.items.len();
        // if list_len == 0 {
        //     return;
        // }

        // let n_row = self.panel.get_rows();
        // let max_lines = list_len as u16 + self.start_row;
        // let check_max = |lines| min(lines, max_lines);

        // // check the bounds of lines and adjust accordingly
        // if lines.checked_add((self.top_row + n_row) as i32).is_some() {
        //     checked_lines = lines;
        // } else {
        //     checked_lines = lines - self.top_row - n_row;
        // }

        // old_selected = self.selected;
        // self.selected = self.selected.checked_add(checked_lines).unwrap();

        // // don't allow scrolling past last item in list (if shorter
        // // than self.panel.get_rows())
        // let abs_bottom = min(self.panel.get_rows(), list_len as u16 + self.start_row - 1);
        // if self.selected > abs_bottom {
        //     self.selected = abs_bottom;
        // }

        // // given a selection, apply correct play status and highlight
        // apply_color_played = |menu: &mut Menu<T>, selected, color: ColorType| {
        //     let played = menu
        //         .items
        //         .map_single_by_index(menu.get_menu_idx(selected), |el| el.is_played())
        //         .unwrap_or(false);
        //     menu.set_attrs(selected, played, color);
        // };

        // // return a vec with sorted titles in range start, end (exclusive)
        // get_titles = |menu: &mut Menu<T>, start, end| {
        //     menu.items.map_by_range(start, end, |el| {
        //         Some(el.get_title(menu.panel.get_cols() as usize))
        //     })
        // };

        // // scroll list if necessary:
        // // scroll down
        // if (self.selected) > (n_row - 1) {
        //     // for scrolls that don't start at the bottom
        //     apply_color_played(self, old_selected, ColorType::Normal);
        //     let delta = n_row - old_selected - 1;

        //     let titles = get_titles(
        //         self,
        //         (self.top_row + n_row) as usize,
        //         (check_max(checked_lines + self.top_row + n_row - delta)) as usize,
        //     );
        //     for title in titles.into_iter() {
        //         self.top_row += 1;
        //         self.panel.delete_line(self.start_row);
        //         old_selected -= 1;
        //         self.panel.delete_line(n_row - 1);
        //         self.panel.write_line(n_row - 1, title);
        //         apply_color_played(self, n_row - 1, ColorType::Normal);
        //     }
        //     self.selected = n_row - 1;

        // // scroll up
        // } else if self.selected < self.start_row {
        //     let titles = get_titles(
        //         self,
        //         max(0, self.top_row + self.selected) as usize,
        //         (self.top_row) as usize,
        //     );
        //     for title in titles.into_iter().rev() {
        //         self.top_row -= 1;
        //         self.panel.insert_line(self.start_row, title);
        //         apply_color_played(self, 1, ColorType::Normal);
        //         old_selected += 1;
        //     }
        //     self.selected = self.start_row;
        // }
        // apply_color_played(self, old_selected, ColorType::Normal);
        // apply_color_played(self, self.selected, ColorType::HighlightedActive);
    }

    /// Sets font style and color of menu item. `index` is the position
    /// of the menu item to be changed. `played` is an indicator of
    /// whether that item has been played or not. `color` is a ColorType
    /// representing the appropriate state of the item (e.g., Normal,
    /// Highlighted).
    pub fn set_attrs(&mut self, index: u16, played: bool, color: ColorType) {
        // let attr = if played {
        //     pancurses::A_NORMAL
        // } else {
        //     pancurses::A_BOLD
        // };
        // self.panel
        //     .change_attr(index, -1, self.panel.get_cols() + 3, attr, color);
    }

    /// Highlights the currently selected item in the menu, based on
    /// whether the menu is currently active or not.
    pub fn highlight_selected(&mut self, active_menu: bool) {
        // let is_played = self
        //     .items
        //     .map_single_by_index(self.get_menu_idx(self.selected), |el| el.is_played());

        // if let Some(played) = is_played {
        //     if active_menu {
        //         self.set_attrs(self.selected, played, ColorType::HighlightedActive);
        //     } else {
        //         self.set_attrs(self.selected, played, ColorType::Highlighted);
        //     }
        // }
    }

    /// Controls how the window changes when it is active (i.e., available
    /// for user input to modify state).
    pub fn activate(&mut self) {
        // if list is empty, will return None
        if let Some(played) = self
            .items
            .map_single_by_index(self.get_menu_idx(self.selected), |el| el.is_played())
        {
            self.set_attrs(self.selected, played, ColorType::HighlightedActive);
        }
    }

    /// Updates window size
    pub fn resize(&mut self, n_row: u16, n_col: u16, start_y: u16, start_x: u16) {
        self.panel.resize(n_row, n_col, start_y, start_x);
        let n_row = self.panel.get_rows();

        // if resizing moves selected item off screen, scroll the list
        // upwards to keep same item selected
        if self.selected > (n_row - 1) {
            self.top_row = self.top_row + self.selected - (n_row - 1);
            self.selected = n_row - 1;
        }
    }

    /// Given a row on the panel, this translates it into the
    /// corresponding menu item it represents. Note that this does not
    /// do any checks to ensure `screen_y` is between 0 and `n_rows`,
    /// or that the resulting menu index is between 0 and `n_items`.
    /// It's merely a straight translation.
    pub fn get_menu_idx(&self, screen_y: u16) -> usize {
        return (self.top_row + screen_y - self.start_row) as usize;
    }
}


impl Menu<Podcast> {
    /// Returns a cloned reference to the list of episodes from the
    /// currently selected podcast.
    pub fn get_episodes(&self) -> LockVec<Episode> {
        let index = self.get_menu_idx(self.selected);
        let (borrowed_map, borrowed_order) = self.items.borrow();
        let pod_id = borrowed_order
            .get(index)
            .expect("Could not retrieve podcast.");
        return borrowed_map
            .get(pod_id)
            .expect("Could not retrieve podcast info.")
            .episodes
            .clone();
    }

    /// Controls how the window changes when it is inactive (i.e., not
    /// available for user input to modify state).
    pub fn deactivate(&mut self) {
        // if list is empty, will return None
        if let Some(played) = self
            .items
            .map_single_by_index(self.get_menu_idx(self.selected), |el| el.is_played())
        {
            self.set_attrs(self.selected, played, ColorType::Highlighted);
        }
    }
}

impl Menu<Episode> {
    /// Controls how the window changes when it is inactive (i.e., not
    /// available for user input to modify state).
    pub fn deactivate(&mut self) {
        // if list is empty, will return None
        if let Some(played) = self
            .items
            .map_single_by_index(self.get_menu_idx(self.selected), |el| el.is_played())
        {
            self.set_attrs(self.selected, played, ColorType::Normal);
        }
    }
}

impl Menu<NewEpisode> {
    /// Changes the status of the currently highlighted episode -- if it
    /// was selected to be downloaded, it will be unselected, and vice
    /// versa.
    pub fn select_item(&mut self) {
        let changed = self.change_item_selections(vec![self.get_menu_idx(self.selected)], None);
        if changed {
            self.update_items();
            self.highlight_selected(true);
        }
    }

    /// Changes the status of all items in the list. If there are any
    /// unselected episodes, this will convert all episodes to be
    /// selected; if all are selected already, only then will it convert
    /// all to unselected.
    pub fn select_all_items(&mut self) {
        let all_selected = self.items.map(|ep| ep.selected).iter().all(|x| *x);
        let changed =
            self.change_item_selections((0..self.items.len()).collect(), Some(!all_selected));
        if changed {
            self.update_items();
            self.highlight_selected(true);
        }
    }

    /// Given a list of index values in the menu, this changes the status
    /// of these episode -- if they were selected to be downloaded, they
    /// will be unselected, and vice versa. If `selection` is a boolean,
    /// however, it will be set to this value explicitly rather than just
    /// being reversed.
    fn change_item_selections(&mut self, indexes: Vec<usize>, selection: Option<bool>) -> bool {
        let mut changed = false;
        {
            let (mut borrowed_map, borrowed_order) = self.items.borrow();
            for idx in indexes {
                if let Some(ep_id) = borrowed_order.get(idx) {
                    if let Entry::Occupied(mut ep) = borrowed_map.entry(*ep_id) {
                        let ep = ep.get_mut();
                        match selection {
                            Some(sel) => ep.selected = sel,
                            None => ep.selected = !ep.selected,
                        }
                        changed = true;
                    }
                }
            }
        }
        return changed;
    }
}


// TESTS ----------------------------------------------------------------
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use chrono::Utc;

//     fn create_menu(n_row: u16, n_col: u16, top_row: u16, selected: u16) -> Menu<Episode> {
//         let titles = vec![
//             "A Very Cool Episode",
//             "This is a very long episode title but we'll get through it together",
//             "An episode with le Unicodé",
//             "How does an episode with emoji sound? 😉",
//             "Here's another title",
//             "Un titre, c'est moi!",
//             "One more just for good measure",
//         ];
//         let mut items = Vec::new();
//         for (i, t) in titles.iter().enumerate() {
//             let played = i % 2 == 0;
//             items.push(Episode {
//                 id: i as _,
//                 pod_id: 1,
//                 title: t.to_string(),
//                 url: String::new(),
//                 description: String::new(),
//                 pubdate: Some(Utc::now()),
//                 duration: Some(12345),
//                 path: None,
//                 played: played,
//             });
//         }

//         let panel = Panel::new("Episodes".to_string(), 1, n_row, n_col, 0);
//         return Menu {
//             panel: panel,
//             header: None,
//             items: LockVec::new(items),
//             start_row: 0,
//             top_row: top_row,
//             selected: selected,
//         };
//     }

//     #[test]
//     fn scroll_up() {
//         let real_rows = 5;
//         let real_cols = 65;
//         let mut menu = create_menu(real_rows + 2, real_cols + 5, 2, 0);
//         menu.update_items();

//         menu.scroll(Scroll::Up(1));

//         let expected_top = menu
//             .items
//             .map_single_by_index(1, |ep| ep.get_title(real_cols as usize))
//             .unwrap();
//         let expected_bot = menu
//             .items
//             .map_single_by_index(5, |ep| ep.get_title(real_cols as usize))
//             .unwrap();

//         assert_eq!(menu.panel.get_row(0).0, expected_top);
//         assert_eq!(menu.panel.get_row(4).0, expected_bot);
//     }

//     #[test]
//     fn scroll_down() {
//         let real_rows = 5;
//         let real_cols = 65;
//         let mut menu = create_menu(real_rows + 2, real_cols + 5, 0, 4);
//         menu.update_items();

//         menu.scroll(Scroll::Down(1));

//         let expected_top = menu
//             .items
//             .map_single_by_index(1, |ep| ep.get_title(real_cols as usize))
//             .unwrap();
//         let expected_bot = menu
//             .items
//             .map_single_by_index(5, |ep| ep.get_title(real_cols as usize))
//             .unwrap();

//         assert_eq!(menu.panel.get_row(0).0, expected_top);
//         assert_eq!(menu.panel.get_row(4).0, expected_bot);
//     }

//     #[test]
//     fn resize_bigger() {
//         let real_rows = 5;
//         let real_cols = 65;
//         let mut menu = create_menu(real_rows + 2, real_cols + 5, 0, 4);
//         menu.update_items();

//         menu.resize(real_rows + 2 + 5, real_cols + 5 + 5, 0, 0);
//         menu.update_items();

//         assert_eq!(menu.top_row, 0);
//         assert_eq!(menu.selected, 4);

//         let non_empty: Vec<String> = menu
//             .panel
//             .window
//             .iter()
//             .filter_map(|x| {
//                 if x.0.is_empty() {
//                     None
//                 } else {
//                     Some(x.0.clone())
//                 }
//             })
//             .collect();
//         assert_eq!(non_empty.len(), menu.items.len());
//     }

//     #[test]
//     fn resize_smaller() {
//         let real_rows = 7;
//         let real_cols = 65;
//         let mut menu = create_menu(real_rows + 2, real_cols + 5, 0, 6);
//         menu.update_items();

//         menu.resize(real_rows + 2 - 2, real_cols + 5 - 5, 0, 0);
//         menu.update_items();

//         assert_eq!(menu.top_row, 2);
//         assert_eq!(menu.selected, 4);

//         let non_empty: Vec<String> = menu
//             .panel
//             .window
//             .iter()
//             .filter_map(|x| {
//                 if x.0.is_empty() {
//                     None
//                 } else {
//                     Some(x.0.clone())
//                 }
//             })
//             .collect();
//         assert_eq!(non_empty.len(), (real_rows - 2) as usize);
//     }

//     #[test]
//     fn chop_accent() {
//         let real_rows = 5;
//         let real_cols = 25;
//         let mut menu = create_menu(real_rows + 2, real_cols + 5, 0, 0);
//         menu.update_items();

//         let expected = "An episode with le Unicod".to_string();

//         assert_eq!(menu.panel.get_row(2).0, expected);
//     }

//     #[test]
//     fn chop_emoji() {
//         let real_rows = 5;
//         let real_cols = 38;
//         let mut menu = create_menu(real_rows + 2, real_cols + 5, 0, 0);
//         menu.update_items();

//         let expected = "How does an episode with emoji sound? ".to_string();

//         assert_eq!(menu.panel.get_row(3).0, expected);
//     }
// }
