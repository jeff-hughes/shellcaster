extern crate rusqlite;
use rusqlite::{Connection, params};

#[derive(Debug)]
pub struct Podcast {
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Debug)]
pub struct Database {
    conn: Option<Connection>,
}

impl Database {
    pub fn create(&self) {
        if let Some(conn) = &self.conn {
            // create podcasts table
            match conn.execute(
                "CREATE TABLE IF NOT EXISTS podcasts (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    url TEXT NOT NULL UNIQUE
                );",
                params![],
            ) {
                Ok(_) => (),
                Err(err) => panic!("Could not create podcasts database table: {}", err),
            }

            // create episodes table
            match conn.execute(
                "CREATE TABLE IF NOT EXISTS episodes (
                    id INTEGER PRIMARY KEY,
                    podcast_id INTEGER NOT NULL,
                    name TEXT NOT NULL,
                    url TEXT NOT NULL UNIQUE,
                    FOREIGN KEY(podcast_id) REFERENCES podcasts(id)
                );",
                params![],
            ) {
                Ok(_) => (),
                Err(err) => panic!("Could not create episodes database table: {}", err),
            }

            // create files table
            match conn.execute(
                "CREATE TABLE IF NOT EXISTS files (
                    id INTEGER PRIMARY KEY,
                    episode_id INTEGER NOT NULL,
                    path TEXT NOT NULL,
                    FOREIGN KEY (episode_id) REFERENCES episodes(id)
                );",
                params![],
            ) {
                Ok(_) => (),
                Err(err) => panic!("Could not create files database table: {}", err),
            }
        };
    }

    pub fn insert_podcast(&self, name: &str, url: &str) {
        if let Some(conn) = &self.conn {
            match conn.execute(
                "INSERT INTO podcasts (name, url)
                    VALUES (?, ?);",
                params![name, url]
            ) {
                Ok(_) => (),
                Err(err) => panic!("Could not insert data: {}", err),
            }
        }
    }

    pub fn get_podcasts(&self) -> Vec<Podcast> {
        if let Some(conn) = &self.conn {
            let mut stmt = conn.prepare(
                "SELECT * FROM podcasts;").unwrap();
            let podcast_iter = stmt.query_map(params![], |row| {
                Ok(Podcast {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                })
            }).unwrap();

            let mut podcasts = Vec::new();
            for pc in podcast_iter {
                podcasts.push(pc.unwrap());
            }
            return podcasts;
        } else {
            return Vec::new();
        }
    }
}

pub fn connect() -> Database {
    match Connection::open("data.db") {
        Ok(conn) => {
            let db_conn = Database {
                conn: Some(conn),
            };
            db_conn.create();
            return db_conn;
        },
        Err(err) => panic!("Could not open database: {}", err),
    };
}