# Changelog

## v0.8.1 (2020-07-01)
- Can now remove one or more episodes from the list of episodes, effectively hiding them so they will not be re-synced
- Can also remove podcasts entirely
- Removing podcasts or episodes will also prompt whether you wish to delete any local files associated with them

## v0.8.0 (2020-06-30)
- Can now delete downloaded files
- Bug fixes related to SQLite database constraints

## v0.7.4 (2020-06-29)
- Numerous small bug fixes:
  - Marking the first episode in the list as played/unplayed no longer crashes
  - Marking episodes as played/unplayed updates the metadata in the podcast title
  - Highlighted podcast/episode should now stay highlighted when the menus are refreshed
  - No longer will re-download files if episode has already been downloaded

## v0.7.3 (2020-06-29)
- Fixes fatal error when podcast or episode lists are empty
- Adds some extra styling to welcome screen for first-time users

## v0.7.2 (2020-06-29)
- Adds extra metadata to titles:
  - Podcasts now show (number of unplayed episodes / total number of episodes)
  - Episodes now show the publication date and the total duration of the episode
  - These are flexibly turned on and off based on the size of the terminal window, to ensure readability of the titles of podcasts and episodes

## v0.7.1 (2020-06-28)
- Overhaul to look and feel of UI
- Fix issues with possibility of invalid filenames
- Lots of under-the-hood improvements in the code structure

## v0.7.0 (2020-06-25)
- Major overhaul to download system to ensure multiple downloads do not pause app, nor overload requests to servers
- Set number of simultaneous downloads in config file

## v0.6.0 (2020-06-24)
- Functionality to mark episodes as played/unplayed, or mark all episodes for a podcast as played/unplayed

## v0.5.3 (2020-06-24)
- Adds multi-threading to allow long-running tasks to run in background

## v0.5.2 (2020-05-05)
- Adds welcome screen for when podcast list is empty
- Handles resizing of terminal window
- Messages at bottom of screen no longer pause app

## v0.5.1 (2020-05-01)
- Specify path to config file via command line argument

## v0.5.0 (2020-05-01)
- App officially named "shellcaster"
- Synchronize one podcast feed, or all feeds

## v0.4.1 (2020-04-29)
- Play file with external media player
- Can specify command to use when playing file

## v0.4.0 (2020-04-29)
- Can specify custom paths for config and data files
- Can download single episodes, or all episodes for a podcast

## v0.3.0 (2020-04-18)
- Reads config settings from config.toml file
- Customizable keybindings

## v0.2.1 (2020-04-13)
- Displays list of episodes in menu, and change between podcast and episode menus

## v0.2.0 (2020-04-13)
- Saving data in SQLite database
- Functionality to add new podcast feed

## v0.1.0 (2020-04-02)
- Bare-bones functionality, of a scrolling menu with pancurses