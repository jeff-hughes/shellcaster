use std::cmp;

extern crate pancurses;
use pancurses::{Input};

const N_OPTS: usize = 100;

fn main() {
    let stdscr = pancurses::initscr();

    // set some options
    pancurses::cbreak();  // allows characters to be read one by one
    pancurses::noecho();  // turns off automatic echoing of characters
                          // to the screen as they are input
    pancurses::start_color(); // allows colours if available
    pancurses::curs_set(0); // turn off cursor
    stdscr.keypad(true);  // returns special characters as single key codes

    let (n_row, n_col) = stdscr.get_max_yx();
    let left_pad = pancurses::newwin(n_row, n_col / 2, 0, 0);

    // make list of strings (probably) larger than available window
    let mut string_list: Vec<String> = Vec::with_capacity(N_OPTS);
        // if capacity unknown, use Vec::new()
    for i in 0..N_OPTS {
        string_list.push(i.to_string());
    }

    // for visible rows, print strings from list
    for i in 0..n_row {
        if let Some(elem) = string_list.get(i as usize) {
            left_pad.mvprintw(i, 0, elem);
        } else {
            break;
        }
    }

    /* 
     * both `pad_selected` and `old_selected` are relative to the window,
     * i.e., they will be values between 0 and n_row - 1; `pad_top_row` is
     * relative to string_list index
     */
    let mut pad_top_row: i32 = 0;  // top row of text shown in window
    let mut pad_selected: i32 = 0;  // which line of text is highlighted
    let mut old_selected: i32 = 0;  // which line of text WAS highlighted

    stdscr.noutrefresh();
    left_pad.mvchgat(pad_selected, 0, -1, pancurses::A_REVERSE, 0);
    left_pad.noutrefresh();
    pancurses::doupdate();

    loop {
        match stdscr.getch() {
            Some(Input::KeyResize) => {
                pancurses::resize_term(0, 0);
                // (n_row, n_col) = stdscr.get_max_yx();
                // TODO: Need to handle increasing and decreasing rows
            },
            Some(Input::KeyDown) => {
                old_selected = pad_selected;
                pad_selected += 1;
            },
            Some(Input::KeyUp) => {
                old_selected = pad_selected;
                pad_selected -= 1;
            },
            Some(Input::Character(c)) => {
                if c == 'q' {
                    break;
                }
            },
            Some(input) => (),
            None => (),
        };

        // scroll list if necessary
        if pad_selected > (n_row - 1) {
            pad_selected = n_row - 1;
            if let Some(elem) = string_list.get((pad_top_row + n_row) as usize) {
                pad_top_row += 1;
                left_pad.mv(0, 0);
                left_pad.deleteln();
                old_selected -= 1;

                left_pad.mv(n_row-1, 0);
                left_pad.clrtoeol();
                left_pad.printw(elem);
            }
        } else if pad_selected < 0 {
            pad_selected = 0;
            if let Some(elem) = string_list.get((pad_top_row - 1) as usize) {
                pad_top_row -= 1;
                left_pad.mv(0, 0);
                left_pad.insertln();
                old_selected += 1;

                left_pad.mv(0, 0);
                left_pad.printw(elem);
            }
        }

        left_pad.mvchgat(old_selected, 0, -1, pancurses::A_NORMAL, 0);
        left_pad.mvchgat(pad_selected, 0, -1, pancurses::A_REVERSE, 0);
        left_pad.refresh();
    }

    pancurses::endwin();
}