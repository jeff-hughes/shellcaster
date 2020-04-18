# Shellcaster

Shellcaster is a terminal-based podcast manager, built in Rust. It is currently absolutely not in stable format in any way, and is still early in the development stage.

The end goal is to provide a terminal UI (i.e., ncurses) to allow users to subscribe to podcast feeds, and periodically check for new episodes. Podcasts and episodes will be listed in a menu, and episodes may be downloaded locally, played (probably with an external media player, at least for now), and marked as played/unplayed. Keybindings and other options will be configurable via a config file.

## Current progress

Right now the program only has the barest of functionality. You can add a new podcast feed by typing "a", then typing the URL of the feed at the prompt, followed by &lt;Enter&gt;. (&lt;Esc&gt; will get you out of the prompt.) Data about the podcast and its episodes will be stored in a sqlite database, and the current list of all podcasts will be presented in an ncurses menu on the screen. Navigating with the arrow keys will let you select different podcasts, with the episode list changing accordingly. Pressing "q" will quit the program.

## Keybindings (currently implemented functions are in bold)

| Key     | Action         |
| ------- | -------------- |
| Arrow keys / h,j,k,l | **Navigate menus** |
| a       | **Add new feed** |
| q       | **Quit program** |
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
| /       | Search episodes |

Keybindings can be modified in the config.toml file. Actions can be
mapped to more than one key, but a single key may not do more than one
action.

## Why "shellcaster"?

I was trying to come up with a play on the word "podcast", and I liked the use of the word "shell" for several reasons. "Shell" is a synonym for the word "pod". The terminal is also referred to as a shell (and shellcaster is a terminal-based program). In addition, the program is built on Rust, whose mascot is Ferris the crab. Finally, I just personally enjoy that "shellcaster" sounds a lot like "spellcaster", so you can feel like a wizard when you use the program...