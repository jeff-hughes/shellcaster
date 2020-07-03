use super::{Colors, ColorType};

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