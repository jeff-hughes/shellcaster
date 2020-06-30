# Shellcaster

Shellcaster is a terminal-based podcast manager, built in Rust. It is currently not yet in stable format, and is still in active development.

The app provides a terminal UI (i.e., ncurses) to allow users to subscribe to podcast feeds, and periodically check for new episodes. Podcasts and episodes are listed in a menu, and episodes may be downloaded locally, played (with an external media player, at least for now), and marked as played/unplayed. Keybindings and other options are configurable via a config file.

![shellcaster screenshot](https://raw.githubusercontent.com/jeff-hughes/shellcaster/master/img/screenshot.png)

## Current progress

Right now the program has most of the basic functionality, but has not yet been optimized, and there are still features that have not yet been implemented. See the keybindings below for the list of functions available.

## Keybindings

| Key     | Action         |
| ------- | -------------- |
| Arrow keys / h,j,k,l | Navigate menus |
| a       | Add new feed |
| q       | Quit program |
| s       | Synchronize selected feed |
| Shift+S | Synchronize all feeds |
| Enter / p | Play selected episode |
| m       | Mark selected episode as played/unplayed |
| Shift+M | Mark all episodes as played/unplayed |
| d       | Download selected episode |
| Shift+D | Download all episodes |
| x       | Delete downloaded file |
| Shift+X | Delete all downloaded files |
| r       | Remove selected feed/episode from list |
| Shift+R | Remove all feeds/episodes from list |

Keybindings can be modified in the config.toml file. Actions can be
mapped to more than one key, but a single key may not do more than one
action.

## Compiling shellcaster

To reiterate, shellcaster does *not* yet have a stable release. But if you're interested in compiling it yourself, you can build the binaries with the following commands.

**Note:** This assumes you already have Rust + cargo installed, and are using a Unix shell (e.g., bash, zsh, fish). You can probably compile it on Windows as well, but you're on your own for that right now.

```
git clone https://github.com/jeff-hughes/shellcaster.git
cd shellcaster
cargo build --release
sudo cp target/release/shellcaster /usr/local/bin/
shellcaster  # to run
```

If you want to change configuration settings:

```
# on Linux
mkdir -p ~/.config/shellcaster
cp config.toml ~/.config/shellcaster/

# on MacOS
mkdir -p ~/Library/Preferences/shellcaster
cp config.toml ~/Library/Preferences/shellcaster/
```

Or you can put `config.toml` in a place of your choosing, and specify the location at runtime:

```
shellcaster -c /path/to/config.toml
```

## Why "shellcaster"?

I was trying to come up with a play on the word "podcast", and I liked the use of the word "shell" for several reasons. "Shell" is a synonym for the word "pod". The terminal is also referred to as a shell (and shellcaster is a terminal-based program). In addition, the program is built on Rust, whose mascot is Ferris the crab. Finally, I just personally enjoy that "shellcaster" sounds a lot like "spellcaster", so you can feel like a wizard when you use the program...