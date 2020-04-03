use std::fmt;
use std::convert;

extern crate rusqlite;
use rusqlite::{Connection, params};

#[derive(Debug)]
pub struct Podcast {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub description: String,
    pub author: String,
    pub explicit: bool,
    // pub episodes: Vec<Episode>,
}

impl fmt::Display for Podcast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}

impl convert::AsRef<str> for Podcast {
    fn as_ref(&self) -> &str {
        return &self.title[..];
    }
}

#[derive(Debug)]
pub struct Episode {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub description: String,
    pub pubdate: String,
    pub path: String,
    pub played: bool,
}

impl fmt::Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}

impl convert::AsRef<str> for Episode {
    fn as_ref(&self) -> &str {
        return &self.title[..];
    }
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
                    title TEXT NOT NULL,
                    url TEXT NOT NULL UNIQUE,
                    description TEXT,
                    author TEXT,
                    explicit INTEGER
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
                    title TEXT NOT NULL,
                    url TEXT NOT NULL UNIQUE,
                    description TEXT,
                    pubdate INTEGER,
                    played INTEGER,
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

    pub fn insert_podcast(&self, name: &str, url: &str, description: &str,
                          author: &str, explicit: bool) {
        if let Some(conn) = &self.conn {
            match conn.execute(
                "INSERT INTO podcasts (title, url, description, author, explicit)
                    VALUES (?, ?, ?, ?, ?);",
                params![name, url, description, author, explicit]
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
                    title: row.get(1)?,
                    url: row.get(2)?,
                    description: row.get(3)?,
                    author: row.get(4)?,
                    explicit: row.get(5)?,
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