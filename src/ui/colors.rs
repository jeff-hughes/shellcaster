use std::collections::HashMap;

/// Enum identifying relevant text states that will be associated with
/// distinct colors.
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum ColorType {
    Normal,
    Highlighted,
    HighlightedActive,
    Error,
}

/// Keeps a hashmap associating ColorTypes with ncurses color pairs.
#[derive(Debug, Clone)]
pub struct Colors {
    map: HashMap<ColorType, i16>,
}

impl Colors {
    pub fn new() -> Colors {
        return Colors {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, color: ColorType, num: i16) {
        self.map.insert(color, num);
    }

    pub fn get(&self, color: ColorType) -> i16 {
        return *self.map.get(&color).unwrap();
    }
}


/// Sets up hashmap for ColorTypes in app, initiates color palette, and
/// sets up ncurses color pairs.
pub fn set_colors() -> Colors {
    // set up a hashmap for easier reference
    let mut colors = Colors::new();
    colors.insert(ColorType::Normal, 0);
    colors.insert(ColorType::Highlighted, 1);
    colors.insert(ColorType::HighlightedActive, 2);
    colors.insert(ColorType::Error, 3);

    // specify some colors by RGB value
    pancurses::init_color(pancurses::COLOR_WHITE, 680, 680, 680);
    pancurses::init_color(pancurses::COLOR_YELLOW, 820, 643, 0);

    // instantiate curses color pairs
    pancurses::init_pair(colors.get(ColorType::Normal),
        pancurses::COLOR_WHITE,
        pancurses::COLOR_BLACK);
    pancurses::init_pair(colors.get(ColorType::Highlighted),
        pancurses::COLOR_BLACK,
        pancurses::COLOR_WHITE);
    pancurses::init_pair(colors.get(ColorType::HighlightedActive),
        pancurses::COLOR_BLACK,
        pancurses::COLOR_YELLOW);
    pancurses::init_pair(colors.get(ColorType::Error),
        pancurses::COLOR_RED,
        pancurses::COLOR_BLACK);

    return colors;
}