use rusqlite::{Connection, params};
use chrono::{NaiveDateTime, DateTime, Utc};

use crate::types::{Podcast, Episode};

/// Struct holding a sqlite database connection, with methods to interact
/// with this connection.
#[derive(Debug)]
pub struct Database {
    conn: Option<Connection>,
}

impl Database {
    /// Creates the necessary database tables, if they do not already
    /// exist. Panics if database cannot be accessed, or if tables cannot
    /// be created.
    pub fn create(&self) {
        let conn = &self.conn.as_ref().unwrap();

        // create podcasts table
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS podcasts (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                description TEXT,
                author TEXT,
                explicit INTEGER,
                last_checked INTEGER
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
                duration INTEGER,
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
    }

    /// Inserts a new podcast and list of podcast episodes into the
    /// database.
    pub fn insert_podcast(&self, podcast: Podcast) ->
        Result<usize, Box<dyn std::error::Error>> {

        let conn = &self.conn.as_ref().unwrap();
        let _ = conn.execute(
            "INSERT INTO podcasts (title, url, description, author, explicit, last_checked)
                VALUES (?, ?, ?, ?, ?, ?);",
            params![
                podcast.title,
                podcast.url,
                podcast.description,
                podcast.author,
                podcast.explicit,
                podcast.last_checked.timestamp()
            ]
        )?;

        let mut stmt = conn.prepare(
            "SELECT id FROM podcasts WHERE url = ?").unwrap();
        let pod_id = stmt
            .query_row::<i32,_,_>(params![podcast.url], |row| row.get(0))
            .unwrap();
        let num_episodes = podcast.episodes.len();

        for ep in podcast.episodes.into_iter().rev() {
            let _ = &self.insert_episode(&pod_id, &ep)?;
        }

        return Ok(num_episodes);
    }

    /// Inserts a podcast episode into the database.
    pub fn insert_episode(&self, podcast_id: &i32, episode: &Episode) ->
        Result<(), Box<dyn std::error::Error>> {

        let conn = &self.conn.as_ref().unwrap();

        let pubdate = match episode.pubdate {
            Some(dt) => Some(dt.timestamp()),
            None => None,
        };

        let _ = conn.execute(
            "INSERT INTO episodes (podcast_id, title, url, description, pubdate, duration, played)
                VALUES (?, ?, ?, ?, ?, ?, ?);",
            params![
                podcast_id,
                episode.title,
                episode.url,
                episode.description,
                pubdate,
                episode.duration,
                false,
            ]
        )?;
        return Ok(());
    }

    /// Generates list of all podcasts in database.
    /// TODO: Currently does not pull list of episodes for each podcast.
    pub fn get_podcasts(&self) -> Vec<Podcast> {
        if let Some(conn) = &self.conn {
            let mut stmt = conn.prepare(
                "SELECT * FROM podcasts ORDER BY title;").unwrap();
            let podcast_iter = stmt.query_map(params![], |row| {
                let naivedt = NaiveDateTime::from_timestamp(row.get(6)?, 0);
                Ok(Podcast {
                    id: Some(row.get(0)?),
                    title: row.get(1)?,
                    url: row.get(2)?,
                    description: row.get(3)?,
                    author: row.get(4)?,
                    explicit: row.get(5)?,
                    last_checked: DateTime::from_utc(naivedt, Utc),
                    episodes: Vec::new(),
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

/// Creates a new connection to the database (and creates database if it
/// does not already exist). Panics if database cannot be accessed.
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