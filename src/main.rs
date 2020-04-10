mod db;
mod ui;
mod types;
mod feeds;

use crate::types::{Podcast};

const N_OPTS: usize = 100;

fn main() {
    let db_inst = db::connect();
    
    // some test data
    // db_inst.insert_podcast("test", "https://www.test.com", "description", "author", true);
    // db_inst.insert_podcast("test2", "https://www.test2.com", "description", "author", true);
    // db_inst.insert_podcast("test3", "https://www.test3.com", "description", "author", true);

    let podcast_list: Vec<Podcast> = db_inst.get_podcasts();
    
    // make list of strings (probably) larger than available window
    let mut string_list: Vec<String> = Vec::with_capacity(N_OPTS);
    // if capacity unknown, use Vec::new()
    for i in 0..N_OPTS {
        string_list.push(i.to_string());
    }
    
    let mut ui = ui::init(&podcast_list);
    // ui.left_menu.init(&string_list);

    loop {
        let mess = ui.getch();
        if let Some(res) = mess.response {
            if res == "quit" {
                break;
            } else if res == "add_feed" {
                if let Some(url) = mess.message {
                    match feeds::get_feed_data(url) {
                        Ok(pod) => {
                            if let Err(_err) = db_inst.insert_podcast(&pod) {
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