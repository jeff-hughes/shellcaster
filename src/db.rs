use std::path::PathBuf;
use std::collections::HashMap;

use rusqlite::{Connection, params};
use chrono::{NaiveDateTime, DateTime, Utc};

use crate::types::*;

/// Struct holding a sqlite database connection, with methods to interact
/// with this connection.
#[derive(Debug)]
pub struct Database {
    conn: Option<Connection>,
}

impl Database {
    /// Creates a new connection to the database (and creates database if
    /// it does not already exist). Panics if database cannot be accessed.
    pub fn connect(path: &PathBuf) -> Database {
        let mut db_path = path.clone();
        if std::fs::create_dir_all(&db_path).is_err() {
            panic!("Unable to create subdirectory for database.");
        }
        db_path.push("data.db");
        match Connection::open(db_path) {
            Ok(conn) => {
                let db_conn = Database {
                    conn: Some(conn),
                };
                db_conn.create();

                // SQLite defaults to foreign key support off
                db_conn.conn.as_ref().unwrap().execute("PRAGMA foreign_keys=ON;", params![]).unwrap();

                return db_conn;
            },
            Err(err) => panic!("Could not open database: {}", err),
        };
    }

    /// Creates the necessary database tables, if they do not already
    /// exist. Panics if database cannot be accessed, or if tables cannot
    /// be created.
    pub fn create(&self) {
        let conn = self.conn.as_ref().unwrap();

        // create podcasts table
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS podcasts (
                id INTEGER PRIMARY KEY NOT NULL,
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
                id INTEGER PRIMARY KEY NOT NULL,
                podcast_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                description TEXT,
                pubdate INTEGER,
                duration INTEGER,
                played INTEGER,
                hidden INTEGER,
                FOREIGN KEY(podcast_id) REFERENCES podcasts(id) ON DELETE CASCADE
            );",
            params![],
        ) {
            Ok(_) => (),
            Err(err) => panic!("Could not create episodes database table: {}", err),
        }

        // create files table
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY NOT NULL,
                episode_id INTEGER NOT NULL,
                path TEXT NOT NULL UNIQUE,
                FOREIGN KEY (episode_id) REFERENCES episodes(id) ON DELETE CASCADE
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

        let conn = self.conn.as_ref().unwrap();
        let _ = conn.execute(
            "INSERT INTO podcasts (title, url, description, author,
                explicit, last_checked)
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
            .query_row::<i64,_,_>(params![podcast.url], |row| row.get(0))
            .unwrap();
        let num_episodes;
        {
            let borrow = podcast.episodes.borrow();
            num_episodes = borrow.len();

            for ep in borrow.iter().rev() {
                let _ = &self.insert_episode(pod_id, &ep)?;
            }
        }

        return Ok(num_episodes);
    }

    /// Inserts a podcast episode into the database.
    pub fn insert_episode(&self, podcast_id: i64, episode: &Episode) ->
        Result<(), Box<dyn std::error::Error>> {

        let conn = self.conn.as_ref().unwrap();

        let pubdate = match episode.pubdate {
            Some(dt) => Some(dt.timestamp()),
            None => None,
        };

        let _ = conn.execute(
            "INSERT INTO episodes (podcast_id, title, url,
                description, pubdate, duration, played, hidden)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?);",
            params![
                podcast_id,
                episode.title,
                episode.url,
                episode.description,
                pubdate,
                episode.duration,
                false,
                false,
            ]
        )?;
        return Ok(());
    }

    /// Inserts a filepath to a downloaded episode.
    pub fn insert_file(&self, episode_id: i64, path: &PathBuf) -> 
        Result<(), Box<dyn std::error::Error>> {

        let conn = self.conn.as_ref().unwrap();

        let _ = conn.execute(
            "INSERT INTO files (episode_id, path)
                VALUES (?, ?);",
            params![
                episode_id,
                path.to_str(),
            ]
        )?;
        return Ok(());
    }

    /// Removes a file listing for an episode from the database when the
    /// user has chosen to delete the file.
    pub fn remove_file(&self, episode_id: i64) {
        let conn = self.conn.as_ref().unwrap();
        let _ = conn.execute(
            "DELETE FROM files WHERE episode_id = ?;",
            params![episode_id]
        ).unwrap();
    }

    /// Removes all file listings for the selected episode ids.
    pub fn remove_files(&self, episode_ids: &[i64]) {
        let conn = self.conn.as_ref().unwrap();

        // convert list of episode ids into a comma-separated String
        let episode_list: Vec<String> = episode_ids.iter()
            .map(|x| x.to_string())
            .collect();
        let episodes = episode_list.join(", ");

        let _ = conn.execute(
            "DELETE FROM files WHERE episode_id = (?);",
            params![episodes]
        ).unwrap();
    }

    /// Removes a podcast, all episodes, and files from the database.
    pub fn remove_podcast(&self, podcast_id: i64) {
        let conn = self.conn.as_ref().unwrap();
        // Note: Because of the foreign key constraints on `episodes`
        // and `files` tables, all associated episodes for this podcast
        // will also be deleted, and all associated file entries for
        // those episodes as well.
        let _ = conn.execute(
            "DELETE FROM podcasts WHERE id = ?;",
            params![podcast_id]
        ).unwrap();
    }

    /// Updates an existing podcast in the database, where metadata is
    /// changed if necessary, and episodes are updated (modified episodes
    /// are updated, new episodes are inserted).
    pub fn update_podcast(&self, podcast: Podcast) -> Result<usize, Box<dyn std::error::Error>> {
        let conn = self.conn.as_ref().unwrap();
        let _ = conn.execute(
            "UPDATE podcasts SET title = ?, url = ?, description = ?,
            author = ?, explicit = ?, last_checked = ?
            WHERE id = ?;",
            params![
                podcast.title,
                podcast.url,
                podcast.description,
                podcast.author,
                podcast.explicit,
                podcast.last_checked.timestamp(),
                podcast.id,
            ]
        )?;

        let num_episodes = podcast.episodes.borrow().len();
        self.update_episodes(podcast.id.unwrap(), podcast.episodes);

        return Ok(num_episodes);
    }

    /// Updates metadata about episodes that already exist in database,
    /// or inserts new episodes.
    ///
    /// Episodes are checked against the URL and published data in
    /// order to determine if they already exist. As such, an existing
    /// episode that has changed either of these fields will show up as
    /// a "new" episode. The old version will still remain in the
    /// database.
    fn update_episodes(&self, podcast_id: i64, episodes: LockVec<Episode>) {
        let conn = self.conn.as_ref().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, url, pubdate FROM episodes
                WHERE podcast_id = ?;").unwrap();
        let episode_iter = stmt.query_map(params![podcast_id], |row| {
            Ok((row.get("id")?, row.get("url")?, row.get("pubdate")?))
        }).unwrap();

        // create hashmap of all episodes, indexed by URL and pub date
        let mut ep_map: HashMap<(String, i64), i64> = HashMap::new();
        for ep in episode_iter {
            let epuw = ep.unwrap();
            ep_map.insert((epuw.1, epuw.2), epuw.0);
        }

        for ep in episodes.borrow().iter().rev() {
            match ep_map.get(&(ep.url.clone(), ep.pubdate.unwrap().timestamp())) {
                // update existing episode
                Some(id) => {
                    let pubdate = match ep.pubdate {
                        Some(dt) => Some(dt.timestamp()),
                        None => None,
                    };
                    let _ = conn.execute(
                        "UPDATE episodes SET title = ?, url = ?,
                            description = ?, pubdate = ?, duration = ?
                            WHERE id = ?;",
                        params![
                            ep.title,
                            ep.url,
                            ep.description,
                            pubdate,
                            ep.duration,
                            id,
                        ]
                    ).unwrap();
                },

                // insert new episode
                None => {
                    let _ = &self.insert_episode(podcast_id, &ep).unwrap();
                }
            }
        }
    }

    /// Updates an episode to mark it as played or unplayed.
    pub fn set_played_status(&self, episode_id: i64, played: bool) {
        let conn = self.conn.as_ref().unwrap();

        let _ = conn.execute(
            "UPDATE episodes SET played = ? WHERE id = ?;",
            params![played, episode_id]
        ).unwrap();
    }

    /// Updates an episode to "remove" it by hiding it. "Removed"
    /// episodes need to stay in the database so that they don't get
    /// re-added when the podcast is synced again.
    pub fn hide_episode(&self, episode_id: i64, hide: bool) {
        let conn = self.conn.as_ref().unwrap();

        let _ = conn.execute(
            "UPDATE episodes SET hidden = ? WHERE id = ?;",
            params![hide, episode_id]
        ).unwrap();
    }

    /// Generates list of all podcasts in database.
    /// TODO: This should probably use a JOIN statement instead.
    pub fn get_podcasts(&self) -> Vec<Podcast> {
        if let Some(conn) = &self.conn {
            let mut stmt = conn.prepare(
                "SELECT * FROM podcasts ORDER BY title;").unwrap();
            let podcast_iter = stmt.query_map(params![], |row| {
                let pod_id = row.get("id")?;
                let episodes = self.get_episodes(pod_id);
                let num_unplayed = episodes.iter()
                    .fold(0, |acc, x| acc + (!x.is_played() as usize));
                Ok(Podcast {
                    id: Some(pod_id),
                    title: row.get("title")?,
                    url: row.get("url")?,
                    description: row.get("description")?,
                    author: row.get("author")?,
                    explicit: row.get("explicit")?,
                    last_checked: convert_date(row.get("last_checked")).unwrap(),
                    episodes: LockVec::new(episodes),
                    num_unplayed: num_unplayed,
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

    /// Generates list of episodes for a given podcast.
    pub fn get_episodes(&self, pod_id: i64) -> Vec<Episode> {
        if let Some(conn) = &self.conn {
            let mut stmt = conn.prepare(
                "SELECT * FROM episodes
                    LEFT JOIN files ON episodes.id = files.episode_id
                    WHERE episodes.podcast_id = ?
                    AND episodes.hidden = 0
                    ORDER BY pubdate DESC;").unwrap();
            let episode_iter = stmt.query_map(params![pod_id], |row| {
                let path = match row.get::<&str, String>("path") {
                    Ok(val) => Some(PathBuf::from(val)),
                    Err(_) => None,
                };
                Ok(Episode {
                    id: Some(row.get("id")?),
                    pod_id: Some(row.get("podcast_id")?),
                    title: row.get("title")?,
                    url: row.get("url")?,
                    description: row.get("description")?,
                    pubdate: convert_date(row.get("pubdate")),
                    duration: row.get("duration")?,
                    path: path,
                    played: row.get("played")?,
                })
            }).unwrap();
            let mut episodes = Vec::new();
            for ep in episode_iter {
                episodes.push(ep.unwrap());
            }
            return episodes;
        } else {
            return Vec::new();
        }
    }
}


/// Helper function converting an (optional) Unix timestamp to a
/// DateTime<Utc> object
fn convert_date(result: Result<i64, rusqlite::Error>) ->
    Option<DateTime<Utc>> {

    return match result {
        Ok(timestamp) => {
            match NaiveDateTime::from_timestamp_opt(timestamp, 0) {
                Some(ndt) => Some(DateTime::from_utc(ndt, Utc)),
                None => None,
            }
        },
        Err(_) => None,
    };
}