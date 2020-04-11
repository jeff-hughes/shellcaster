mod db;
mod ui;
mod types;
mod feeds;

use crate::types::{Podcast};

fn main() {
    let db_inst = db::connect();
    let podcast_list: Vec<Podcast> = db_inst.get_podcasts();
    let mut ui = ui::init(&podcast_list);

    loop {
        let mess = ui.getch();
        if let Some(res) = mess.response {
            if res == "quit" {
                break;
            } else if res == "add_feed" {
                if let Some(url) = mess.message {
                    match feeds::get_feed_data(url) {
                        Ok(pod) => {
                            if let Err(_err) = db_inst.insert_podcast(pod) {
                                // TODO: Print error somewhere to screen
                            }
                        },
                        Err(_err) => (),  // TODO: Print error somewhere to screen
                    }
                }
            }
        }
    }

    ui::tear_down();
}