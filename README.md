# Shellcaster

Shellcaster is a terminal-based podcast manager, built in Rust. It provides a terminal UI (i.e., ncurses) to allow users to subscribe to podcast feeds, and sync feeds to check for new episodes. Episodes may be downloaded locally, played (with an external media player, at least for now), and marked as played/unplayed. Keybindings and other options are configurable via a config file.

Note that shellcaster is not yet in stable format, and is still in active development. However, the basic functionality is present, and it should generally be usable (with some bugs and irritations still to be worked out!).

![shellcaster screenshot](https://raw.githubusercontent.com/jeff-hughes/shellcaster/master/img/screenshot.png)

## Installing shellcaster

There are currently a couple of ways to install shellcaster. The following assumes you already have Rust + cargo installed.

1. You can install the latest version of the binary directly from crates.io with one command:

```bash
cargo install shellcaster
```

2. You can clone the Github repo and compile it yourself:

```bash
git clone https://github.com/jeff-hughes/shellcaster.git
cd shellcaster
cargo build --release
sudo cp target/release/shellcaster /usr/local/bin/
shellcaster  # to run
```

If you want to change configuration settings:

```bash
# on Linux
mkdir -p ~/.config/shellcaster
cp config.toml ~/.config/shellcaster/

# on MacOS
mkdir -p ~/Library/Preferences/shellcaster
cp config.toml ~/Library/Preferences/shellcaster/
```

(If you installed directly with cargo, the sample `config.toml` file can be copied from [here](https://raw.githubusercontent.com/jeff-hughes/shellcaster/master/config.toml). Place it in the same location as noted above.)

Or you can put `config.toml` in a place of your choosing, and specify the location at runtime:

```bash
shellcaster -c /path/to/config.toml
```

**Note:** Shellcaster has currently only been tested on Linux x86. Earlier versions were tested on MacOS, but not extensively. You may be able to compile and use it on Windows as well, but you're on your own for that right now. If you are on Windows, your best bet at this point is to use Windows Subsystem for Linux (WSL) to compile and use it.

## Default Keybindings

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

## Why "shellcaster"?

I was trying to come up with a play on the word "podcast", and I liked the use of the word "shell" for several reasons. "Shell" is a synonym for the word "pod". The terminal is also referred to as a shell (and shellcaster is a terminal-based program). In addition, the program is built on Rust, whose mascot is Ferris the crab. Finally, I just personally enjoy that "shellcaster" sounds a lot like "spellcaster", so you can feel like a wizard when you use the program...