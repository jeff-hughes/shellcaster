<div align="center"><img alt="shellcaster logo: Ferris the crab with headphones" src="https://raw.githubusercontent.com/jeff-hughes/shellcaster/master/img/shellcaster-logo_smol.png"/></div>

# Note

This project is no longer maintained. I no longer have the time, nor the motivation, to maintain it. It may still work for you, but I make no guarantees. I may come back to it at some point in the future, but no guarantees on that either.

# Shellcaster

Shellcaster is a terminal-based podcast manager, built in Rust. It provides a terminal UI (i.e., an ncurses-like interface) to allow users to subscribe to podcast feeds, and sync feeds to check for new episodes. Episodes may be downloaded locally, played with an external media player, and marked as played/unplayed. Keybindings and other options are configurable via a config file.

<div align="center"><img alt="screenshot of shellcaster" src="https://raw.githubusercontent.com/jeff-hughes/shellcaster/master/img/screenshot.png"/></div>

## Installing shellcaster

### On Arch Linux

There are packages available for shellcaster in the Arch User Repository (AUR). Use `makepkg -si` ([see further details](https://wiki.archlinux.org/index.php/Arch_User_Repository#Installing_and_upgrading_packages)) or your favourite AUR helper program to install one of the following packages:

* [Stable source package](https://aur.archlinux.org/packages/shellcaster/)
* [Stable binary package](https://aur.archlinux.org/packages/shellcaster-bin/)
* [Latest development package](https://aur.archlinux.org/packages/shellcaster-git/)

### On other Linux distributions and MacOS

Currently the only option is to build from source.

First, ensure you have installed the necessary dependencies:

  * rust
  * gcc
  * pkg-config
  * libsqlite3-dev

**Notes:**

  * The names of these dependencies may be slightly different for your system. For `libsqlite3-dev`, you are looking for the development headers for SQLite, which may be separate from the runtime package (e.g., with a `-dev` suffix).
  * If you enable the "native_tls" feature of shellcaster (disabled by default), you will also need `libssl-dev`, the development headers for OpenSSL (not needed on MacOS).
  * If you enable the "sqlite-bundled" feature of shellcaster (disabled by default), `pkg-config` and `libsqlite3-dev` are not necessary.

Next, there are two options for compiling the program: 

1. You can install the latest version of the binary directly from crates.io with one command:

```bash
# for MacOS or Linux
sudo cargo install shellcaster --no-track --root "/usr/local"  # add or remove any features with --features

# or for Linux, without needing root permissions
cargo install shellcaster --no-track --root "$HOME/.local"
```

2. You can clone the Github repo and compile it yourself:

```bash
git clone https://github.com/jeff-hughes/shellcaster.git
cd shellcaster
cargo build --release  # add or remove any features with --features

# for MacOS or Linux
sudo cp target/release/shellcaster /usr/local/bin/

# or for Linux, no root permissions
cp target/release/shellcaster ~/.local/bin
```

See below for the list of available features when compiling.

### On Windows

Shellcaster is **not currently supported on Windows**, although some work has been done to try to get it working. Unicode support is weak, however, and there are issues when resizing the screen. You *might* have better luck using the new Windows Terminal, but this has not been tested. If you are a Windows user and want to help work out the bugs, pull requests are more than welcome!

### List of compile features

By default, only the `native_certs` feature is enabled. Here is the full list of features:

* `sqlite_bundled`: When disabled, Rust will try to link shellcaster with SQLite header files already present on your system. If enabled, Rust will instead build SQLite from source and bundle the program with shellcaster. Bundling results in a larger application size, but may be suitable if you wish to use a different version of SQLite than the one on your system, or if you are on a system where installing SQLite is more difficult.

* `native_tls`: By default, shellcaster uses the [rustls](https://crates.io/crates/rustls) crate to enable TLS support (i.e., URLs with https). This may cause issues with some podcast feeds that use earlier versions of TLS (below TLS v1.2). If you find that some feeds are unable to update, you can try enabling the `native_tls` feature, which will instead use the [native-tls](https://crates.io/crates/native-tls) crate -- which uses OpenSSL on Linux, Secure Transport on MacOS, and SChannel on Windows.

* `native_certs`: Shellcaster will use the trusted certificate roots from the trust store for your OS in order to validate TLS certificates. Turning this feature off will instead use a bundled copy of the Mozilla Root program, which will only be updated when you recompile shellcaster. Thus, leaving this feature enabled is recommended.

To specify different features when compiling, here is the format:

```bash
cargo install --no-track --no-default-features --features "<feature1>,<feature2>" --root "$HOME/.local"
```

The format is the same when using `cargo build` instead:

```bash
cargo build --release --no-default-features --features "<feature1>,<feature2>"
cp target/release/shellcaster ~/.local/bin/
```

## Running shellcaster

Easy peasy! In your terminal, run:

```bash
shellcaster
```

Note that if you installed shellcaster to a different location, ensure that this location has been added to your `$PATH`:

```bash
export PATH="/path/to/add:$PATH"
```

## Importing/exporting podcasts

Shellcaster supports importing OPML files from other podcast managers. If you can export to an OPML file from another podcast manager, you can import this file with:

```bash
shellcaster import -f /path/to/OPML/file.opml
```

If the `-r` flag is added to this command, it will overwrite any existing podcasts that are currently stored in shellcaster. You can also pipe in data to `shellcaster import` from stdin by not specifying the `-f <file>`.

You can export an OPML file from shellcaster with the following command:

```bash
shellcaster export -f /path/to/output/file.opml
```

You can also export to stdout by not specifying the `-f <file>`; for example, this command is equivalent:

```bash
shellcaster export > /path/to/output/file.opml
```

## Configuring shellcaster

If you want to change configuration settings, the sample `config.toml` file can be copied from [here](https://raw.githubusercontent.com/jeff-hughes/shellcaster/master/config.toml). Download it, edit it to your fancy, and place it in the following location:

```bash
# on Linux
mkdir -p ~/.config/shellcaster
cp config.toml ~/.config/shellcaster/

# on MacOS
mkdir -p ~/Library/Preferences/shellcaster
cp config.toml ~/Library/Preferences/shellcaster/
```

Or you can put `config.toml` in a place of your choosing, and specify the location at runtime:

```bash
shellcaster -c /path/to/config.toml
```

The sample file above provides comments that should walk you through all the available options. If any field does not appear in the config file, it will be filled in with the default value specified in those comments. The defaults are also listed below, for convenience.

### Configuration options

**download_path**:
* Specifies where podcast episodes that are downloaded will be stored.
* Defaults:
  * On Linux: $XDG_DATA_HOME/shellcaster/ or $HOME/.local/share/shellcaster/
  * On Mac: $HOME/Library/Application Support/shellcaster/
  * On Windows: C:\Users\\**username**\AppData\Local\shellcaster\

**play_command**:
* Command used to play episodes. Use "%s" to indicate where file/URL will be entered to the command. Note that shellcaster does *not* include a native media player -- it simply passes the file path/URL to the given command with no further checking as to its success or failure. This process is started *in the background*, so be sure to send it to a program that has GUI controls of some kind so you have control over the playback.
* Default: "vlc %s"

**download_new_episodes**:
* Configures what happens when new episodes are found as podcasts are synced. Valid options:
    * "always" will automatically download all new episodes;
    * "ask-selected" will open a popup window to let you select which episodes to download, with all of them selected by default;
    * "ask-unselected" will open a popup window to let you select with episodes to download, with none of them selected by default;
    * "never" will never automatically download new episodes.
* Default: "ask-unselected"

**simultaneous_downloads**:
* Maximum number of files to download simultaneously. Setting this too high could result in network requests being denied. A good general guide would be to set this to the number of processor cores on your computer.
* Default: 3

**max_retries**:
* Maximum number of times to retry connecting to a URL to sync a podcast or download an episode.
* Default: 3

#### Default keybindings

| Key     | Action         |
| ------- | -------------- |
| ?       | Open help window |
| Arrow keys / h,j,k,l | Navigate menus |
| Shift+K | Up 1/4 page |
| Shift+J | Down 1/4 page |
| PgUp    | Page up |
| PgDn    | Page down |
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
| 1       | Toggle played/unplayed filter |
| 2       | Toggle downloaded/undownloaded filter |

**Note:** Actions can be mapped to more than one key (e.g., "Enter" and "p" both play an episode), but a single key may not do more than one action (e.g., you can't set "d" to both download and delete episodes).

#### Customizable colors

You can set the colors in the app with either built-in terminal colors or (provided your terminal supports it) customizable colors as well. See the "colors" section in the [config.toml](https://github.com/jeff-hughes/shellcaster/blob/master/config.toml) for details about how to specify these colors!

## Syncing without the UI

Some users may wish to sync their podcasts automatically on a regular basis, e.g., every morning. The `shellcaster sync` subcommand can be used to do this without opening up the UI, and does a full sync of all podcasts in the database. This could be used to set up a cron job or systemd timer, for example. Please refer to the relevant documentation for these systems for setting it up on the schedule of your choice.

## Contributing

Contributions from others are welcome! If you wish to contribute, feel free to clone the repo and submit pull requests. **Please ensure you are on the `develop` branch when making your edits**, as this is where the continued development of the app is taking place. Pull requests will only be merged to the `develop` branch, so you can help to avoid merge conflicts by doing your work on that branch in the first place.

Thanks to these fine folks who have made contributions: [a-kenji](https://github.com/a-kenji), [dougli1sqrd](https://github.com/dougli1sqrd), [dwvisser](https://github.com/dwvisser), [thunderbiscuit](https://github.com/thunderbiscuit)

## Why "shellcaster"?

I was trying to come up with a play on the word "podcast", and I liked the use of the word "shell" for several reasons. "Shell" is a synonym for the word "pod". The terminal is also referred to as a shell (and shellcaster is a terminal-based program). In addition, the program is built on Rust, whose mascot is Ferris the crab. Finally, I just personally enjoy that "shellcaster" sounds a lot like "spellcaster", so you can feel like a wizard when you use the program...