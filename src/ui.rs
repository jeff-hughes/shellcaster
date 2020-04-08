use std::fmt;
use std::convert;

extern crate pancurses;
use pancurses::{Window, newwin, Input};

#[derive(Debug)]
pub struct UiMessage {
    pub response: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct UI<'a, T> {
    stdscr: Window,
    n_row: i32,
    n_col: i32,
    pub left_menu: Menu<'a, T>,
}

impl<T> UI<'_, T>
    where T: fmt::Display + convert::AsRef<str> {

    pub fn getch(&mut self) -> UiMessage {
        match self.stdscr.getch() {
            Some(Input::KeyResize) => {
                pancurses::resize_term(0, 0);
                // (n_row, n_col) = stdscr.get_max_yx();
                // TODO: Need to handle increasing and decreasing rows
            },
            Some(Input::KeyDown) => {
                self.left_menu.scroll_down(1);
            },
            Some(Input::KeyUp) => {
                self.left_menu.scroll_up(1);
            },
            Some(Input::Character(c)) => {
                // QUIT PROGRAM
                if c == 'q' {
                    return UiMessage {
                        response: Some("quit".to_string()),
                        message: None,
                    };
                
                // ADD NEW PODCAST FEED
                } else if c == 'a' {
                    let prefix = String::from("Feed URL: ");
                    let ins_win = newwin(1, self.n_col, self.n_row-1, 0);
                    ins_win.overlay(&self.left_menu.window);
                    ins_win.mv(self.n_row-1, 0);
                    ins_win.printw(&prefix);
                    ins_win.keypad(true);
                    ins_win.refresh();
                    pancurses::curs_set(2);
                    
                    let mut inputs = String::new();
                    let mut cancelled = false;

                    let min_x = prefix.len() as i32;
                    let mut current_x = prefix.len() as i32;
                    let mut cursor_x = prefix.len() as i32;
                    loop {
                        match ins_win.getch() {
                            Some(Input::KeyExit) |
                            Some(Input::Character('\u{1b}')) => {
                                cancelled = true;
                                break;
                            },
                            Some(Input::KeyEnter) |
                            Some(Input::Character('\n')) => {
                                break;
                            },
                            Some(Input::KeyBackspace) |
                            Some(Input::Character('\u{7f}')) => {
                                if current_x > min_x {
                                    current_x -= 1;
                                    cursor_x -= 1;
                                    let _ = inputs.remove((cursor_x as usize) - prefix.len());
                                    ins_win.mv(0, cursor_x);
                                    ins_win.delch();
                                }
                            },
                            Some(Input::KeyDC) => {
                                if cursor_x < current_x {
                                    let _ = inputs.remove((cursor_x as usize) - prefix.len());
                                    ins_win.delch();
                                }
                            },
                            Some(Input::KeyLeft) => {
                                if cursor_x > min_x {
                                    cursor_x -= 1;
                                    ins_win.mv(0, cursor_x);
                                }
                            },
                            Some(Input::KeyRight) => {
                                if cursor_x < current_x {
                                    cursor_x += 1;
                                    ins_win.mv(0, cursor_x);
                                }
                            },
                            Some(Input::Character(c)) => {
                                current_x += 1;
                                cursor_x += 1;
                                ins_win.insch(c);
                                ins_win.mv(0, cursor_x);
                                inputs.push(c);
                            },
                            Some(_) => (),
                            None => (),
                        }
                        ins_win.refresh();
                    }

                    pancurses::curs_set(0);
                    ins_win.deleteln();
                    ins_win.refresh();
                    ins_win.delwin();

                    if !cancelled && inputs.len() > 0 {
                        return UiMessage {
                            response: Some("add_feed".to_string()),
                            message: Some(inputs),
                        }
                    }
                    return UiMessage {
                        response: Some("add_feed".to_string()),
                        message: None,
                    };
                }
            },
            Some(_) => (),
            None => (),
        };
        return UiMessage {
            response: None,
            message: None,
        };
    }
}

/* 
 * both `pad_selected` and `old_selected` are relative to the window,
 * i.e., they will be values between 0 and n_row - 1; `pad_top_row` is
 * relative to string_list index
 */
#[derive(Debug)]
pub struct Menu<'a, T> {
    window: Window,
    items: &'a Vec<T>,
    n_row: i32,
    n_col: i32,
    top_row: i32,  // top row of text shown in window
    selected: i32,  // which line of text is highlighted
    old_selected: i32,  // which line of text WAS highlighted
}

impl<T> Menu<'_, T>
    where T: fmt::Display + convert::AsRef<str> {

    pub fn init(&mut self) {
        // for visible rows, print strings from list
        for i in 0..self.n_row {
            if let Some(elem) = (*self.items).get(i as usize) {
                // self.window.mvprintw(i, 0, &elem.name);
                self.window.mvprintw(i, 0, format!("{}", elem));
            } else {
                break;
            }
        }

        self.window.refresh();
    }

    pub fn scroll_down(&mut self, lines: i32) {
        self.old_selected = self.selected;
        self.selected += lines;
        self.update();
    }

    pub fn scroll_up(&mut self, lines: i32) {
        self.old_selected = self.selected;
        self.selected -= lines;
        self.update();
    }

    pub fn update(&mut self) {
        // TODO: currently only handles scroll value of 1; need to extend
        // to be able to scroll multiple lines at a time

        // scroll list if necessary
        if self.selected > (self.n_row - 1) {
            self.selected = self.n_row - 1;
            if let Some(elem) = (*self.items).get((self.top_row + self.n_row) as usize) {
                self.top_row += 1;
                self.window.mv(0, 0);
                self.window.deleteln();
                self.old_selected -= 1;

                self.window.mv(self.n_row-1, 0);
                self.window.clrtoeol();
                self.window.printw(elem);
            }

        } else if self.selected < 0 {
            self.selected = 0;
            if let Some(elem) = (*self.items).get((self.top_row - 1) as usize) {
                self.top_row -= 1;
                self.window.mv(0, 0);
                self.window.insertln();
                self.old_selected += 1;

                self.window.mv(0, 0);
                self.window.printw(elem);
            }
        }

        self.window.mvchgat(self.old_selected, 0, -1, pancurses::A_NORMAL, 0);
        self.window.mvchgat(self.selected, 0, -1, pancurses::A_REVERSE, 0);
        self.window.refresh();
    }
}


pub fn init<'a, T>(items: &'a Vec<T>) -> UI<'a, T>
    where T: fmt::Display + convert::AsRef<str> {
    let stdscr = pancurses::initscr();

    // set some options
    pancurses::cbreak();  // allows characters to be read one by one
    pancurses::noecho();  // turns off automatic echoing of characters
                        // to the screen as they are input
    pancurses::start_color(); // allows colours if available
    pancurses::curs_set(0); // turn off cursor
    stdscr.keypad(true);  // returns special characters as single key codes

    let (n_row, n_col) = stdscr.get_max_yx();

    let left_menu_win = newwin(n_row, n_col / 2, 0, 0);
    let mut left_menu = Menu {
        window: left_menu_win,
        items: items,
        n_row: n_row,
        n_col: n_col / 2,
        top_row: 0,
        selected: 0,
        old_selected: 0,
    };

    stdscr.noutrefresh();
    left_menu.init();
    left_menu.window.mvchgat(left_menu.selected, 0, -1, pancurses::A_REVERSE, 0);
    left_menu.window.noutrefresh();
    pancurses::doupdate();

    return UI {
        stdscr,
        n_row,
        n_col,
        left_menu: left_menu,
    }
}

pub fn tear_down() {
    pancurses::endwin();
}