use anyhow::{anyhow, Result};

use lazy_static::lazy_static;
use regex::Regex;

use crate::config::AppColors;

lazy_static! {
    /// Regex for parsing a color specified as hex code.
    static ref RE_COLOR_HEX: Regex = Regex::new(r"(?i)#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})").expect("Regex error");

    /// Regex for parsing a color specified as an rgb(x, y, z) value.
    static ref RE_COLOR_RGB: Regex = Regex::new(r"(?i)rgb\(([0-9]+), ?([0-9]+), ?([0-9]+)\)").expect("Regex error");
}

/// Stores information about a single color value, specified either as
/// a word in the set black, blue, cyan, green, magenta, red, white,
/// yellow, or terminal, or an RGB code with values from 0 to 255.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    Black,
    Blue,
    Cyan,
    Green,
    Magenta,
    Red,
    White,
    Yellow,
    Terminal,
    Rgb(u8, u8, u8),
}

impl ColorValue {
    /// Parses a string that specifies a color either in hex format
    /// (e.g., "#ff0000"), in RGB format (e.g., "rgb(255, 0, 0)"), or
    /// as one of a set of allowed color names.
    pub fn from_str(text: &str) -> Result<Self> {
        if text.starts_with('#') {
            if let Some(cap) = RE_COLOR_HEX.captures(text) {
                return Ok(Self::Rgb(
                    u8::from_str_radix(&cap[1], 16)?,
                    u8::from_str_radix(&cap[2], 16)?,
                    u8::from_str_radix(&cap[3], 16)?,
                ));
            }
            return Err(anyhow!("Invalid color hex code"));
        } else if text.starts_with("rgb") || text.starts_with("RGB") {
            if let Some(cap) = RE_COLOR_RGB.captures(text) {
                return Ok(Self::Rgb(
                    u8::from_str_radix(&cap[1], 10)?,
                    u8::from_str_radix(&cap[2], 10)?,
                    u8::from_str_radix(&cap[3], 10)?,
                ));
            }
            return Err(anyhow!("Invalid color RGB code"));
        } else {
            let text_lower = text.to_lowercase();
            return match &text_lower[..] {
                "black" => Ok(Self::Black),
                "blue" => Ok(Self::Blue),
                "cyan" => Ok(Self::Cyan),
                "green" => Ok(Self::Green),
                "magenta" => Ok(Self::Magenta),
                "red" => Ok(Self::Red),
                "white" => Ok(Self::White),
                "yellow" => Ok(Self::Yellow),
                "terminal" => Ok(Self::Terminal),
                _ => Err(anyhow!("Invalid color code")),
            };
        }
    }

    /// Converts a ColorValue to one of the built-in ncurses numeric
    /// color identifiers. Note that ColorValue::Rgb(_, _, _) returns
    /// None and must be handled separately.
    fn to_ncurses_val(&self) -> Option<i16> {
        return match self {
            Self::Black => Some(pancurses::COLOR_BLACK),
            Self::Blue => Some(pancurses::COLOR_BLUE),
            Self::Cyan => Some(pancurses::COLOR_CYAN),
            Self::Green => Some(pancurses::COLOR_GREEN),
            Self::Magenta => Some(pancurses::COLOR_MAGENTA),
            Self::Red => Some(pancurses::COLOR_RED),
            Self::White => Some(pancurses::COLOR_WHITE),
            Self::Yellow => Some(pancurses::COLOR_YELLOW),
            Self::Terminal => Some(-1),
            Self::Rgb(_, _, _) => None,
        };
    }

    /// Returns whether ColorValue is of variant Terminal.
    fn is_terminal(&self) -> bool {
        return matches!(self, Self::Terminal);
    }

    /// For variant ColorValue::Rgb, returns the RGB associated values.
    fn get_rgb(&self) -> Option<(u8, u8, u8)> {
        return match self {
            Self::Rgb(r, g, b) => Some((*r, *g, *b)),
            _ => None,
        };
    }
}


/// Enum identifying relevant text states that will be associated with
/// distinct colors.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum ColorType {
    // Colorpair 0 is reserved in ncurses for white text on black, and
    // can't be changed, so we just skip it
    Normal = 1,
    Highlighted = 2,
    HighlightedActive = 3,
    Error = 4,
}

/// Sets up hashmap for ColorTypes in app, initiates color palette, and
/// sets up ncurses color pairs.
pub fn set_colors(config: &AppColors) {
    // if the user has specified any colors to be "terminal" (i.e., to
    // use their terminal's default foreground and background colors),
    // then we must tell ncurses to allow the use of those colors.
    if check_for_terminal(config) {
        pancurses::use_default_colors();
    }

    // check if we have any RGB-specified values
    // if count_app_colors(config, ColorValue::Rgb(0, 0, 0)) > 0 {
    // let replace_color_order = vec![ColorValue::Cyan, ColorValue::Magenta, ColorValue::Blue, ColorValue::Green, ColorValue::Yellow, ColorValue::Red, ColorValue::Black, ColorValue::White];
    // }
    let mut replace_counter = 8;
    replace_counter = set_color_pair(ColorType::Normal as u8, &config.normal, replace_counter);
    replace_counter = set_color_pair(
        ColorType::HighlightedActive as u8,
        &config.highlighted_active,
        replace_counter,
    );
    replace_counter = set_color_pair(
        ColorType::Highlighted as u8,
        &config.highlighted,
        replace_counter,
    );
    let _ = set_color_pair(ColorType::Error as u8, &config.error, replace_counter);
}

/// Check for any app colors that are set to "Terminal", which means that
/// we should attempt to use the terminal's default foreground/background
/// colors.
fn check_for_terminal(app_colors: &AppColors) -> bool {
    if app_colors.normal.0.is_terminal() {
        return true;
    }
    if app_colors.normal.1.is_terminal() {
        return true;
    }
    if app_colors.highlighted_active.0.is_terminal() {
        return true;
    }
    if app_colors.highlighted_active.1.is_terminal() {
        return true;
    }
    if app_colors.highlighted.0.is_terminal() {
        return true;
    }
    if app_colors.highlighted.1.is_terminal() {
        return true;
    }
    if app_colors.error.0.is_terminal() {
        return true;
    }
    if app_colors.error.1.is_terminal() {
        return true;
    }
    return false;
}


/// Helper function that takes a set of ColorValues indicating foreground
/// and background colors, initiates customized colors if necessary, and
/// adds the pair to ncurses with the key of `pair_index`.
fn set_color_pair(
    pair_index: u8,
    config: &(ColorValue, ColorValue),
    mut replace_index: i16,
) -> i16 {
    let mut c1 = config.0.to_ncurses_val();
    let mut c2 = config.1.to_ncurses_val();

    if c1.is_none() {
        let rgb = config.0.get_rgb().unwrap();
        pancurses::init_color(
            replace_index,
            u8_to_i16(rgb.0),
            u8_to_i16(rgb.1),
            u8_to_i16(rgb.2),
        );
        c1 = Some(replace_index);
        replace_index += 1;
    }
    if c2.is_none() {
        let rgb = config.1.get_rgb().unwrap();
        pancurses::init_color(
            replace_index,
            u8_to_i16(rgb.0),
            u8_to_i16(rgb.1),
            u8_to_i16(rgb.2),
        );
        c2 = Some(replace_index);
        replace_index += 1;
    }

    pancurses::init_pair(pair_index as i16, c1.unwrap(), c2.unwrap());
    return replace_index;
}

/// Converts a value from 0 to 255 to a value from 0 to 1000, because
/// ncurses has a weird color format.
fn u8_to_i16(val: u8) -> i16 {
    return (val as f32 / 255.0 * 1000.0) as i16;
}


// TESTS -----------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_hex() {
        let color = String::from("#ff0000");
        let parsed = ColorValue::from_str(&color);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), ColorValue::Rgb(255, 0, 0));
    }

    #[test]
    fn color_invalid_hex() {
        let color = String::from("#gg0000");
        assert!(ColorValue::from_str(&color).is_err());
    }

    #[test]
    fn color_invalid_hex2() {
        let color = String::from("#ff000");
        assert!(ColorValue::from_str(&color).is_err());
    }

    #[test]
    fn color_rgb() {
        let color = String::from("rgb(255, 0, 0)");
        let parsed = ColorValue::from_str(&color);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), ColorValue::Rgb(255, 0, 0));
    }

    #[test]
    fn color_rgb_upper() {
        let color = String::from("RGB(255, 0, 0)");
        let parsed = ColorValue::from_str(&color);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), ColorValue::Rgb(255, 0, 0));
    }

    #[test]
    fn color_rgb_no_space() {
        let color = String::from("rgb(255,0,0)");
        let parsed = ColorValue::from_str(&color);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), ColorValue::Rgb(255, 0, 0));
    }
}
