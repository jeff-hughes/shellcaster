use std::path::PathBuf;

use chrono::{DateTime, NaiveDateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use rusqlite::{params, Connection};
use semver::Version;

use crate::types::*;

lazy_static! {
    /// Regex for removing "A", "An", and "The" from the beginning of
    /// podcast titles
    static ref RE_ARTICLES: Regex = Regex::new(r"^(a|an|the) ").unwrap();
}


pub struct SyncResult {
    pub added: Vec<NewEpisode>,
    pub updated: Vec<i64>,
}

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

                {
                    let conn = db_conn.conn.as_ref().unwrap();

                    // SQLite defaults to foreign key support off
                    conn.execute("PRAGMA foreign_keys=ON;", params![]).unwrap();

                    // get version number stored in database
                    let mut stmt = conn
                        .prepare("SELECT version FROM version WHERE id = 1;")
                        .unwrap();
                    let db_version = stmt.query_row(params![], |row| {
                        let vstr: String = row.get("version")?;
                        Ok(Version::parse(&vstr).unwrap())
                    });

                    // compare to current app version
                    let curr_ver = Version::parse(crate::VERSION).unwrap();
                    // (db_version exists, needs update)
                    let to_update = match db_version {
                        Ok(dbv) => {
                            if dbv < curr_ver {
                                (true, true)
                            } else {
                                (true, false)
                            }
                        }
                        Err(_) => (false, true),
                    };

                    if to_update.1 {
                        // any version checks for DB migrations should go
                        // here first, before we update the version

                        db_conn.update_version(curr_ver, to_update.0);
                    }
                }

                return db_conn;
            }
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
                url TEXT NOT NULL,
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

        match conn.execute(
            "CREATE TABLE IF NOT EXISTS version (
                id INTEGER PRIMARY KEY NOT NULL,
                version TEXT NOT NULL
            );",
            params![],
        ) {
            Ok(_) => (),
            Err(err) => panic!("Could not create version database table: {}", err),
        }
    }

    /// If version stored in database is less than the current version
    /// of the app, this updates the value stored in the database to
    /// match.
    fn update_version(&self, current_version: Version, update: bool) {
        let conn = self.conn.as_ref().unwrap();

        if update {
            let _ = conn.execute(
                "UPDATE version SET version = ?
                WHERE id = ?;",
                params![current_version.to_string(), 1],
            );
        } else {
            let _ = conn.execute(
                "INSERT INTO version (id, version)
                VALUES (?, ?)",
                params![1, current_version.to_string()],
            );
        }
    }

    /// Inserts a new podcast and list of podcast episodes into the
    /// database.
    pub fn insert_podcast(
        &self,
        podcast: PodcastNoId,
    ) -> Result<SyncResult, Box<dyn std::error::Error>> {
        let conn = self.conn.as_ref().unwrap();
        let mut stmt = conn.prepare_cached(
            "INSERT INTO podcasts (title, url, description, author,
                explicit, last_checked)
                VALUES (?, ?, ?, ?, ?, ?);",
        )?;
        stmt.execute(params![
            podcast.title,
            podcast.url,
            podcast.description,
            podcast.author,
            podcast.explicit,
            podcast.last_checked.timestamp()
        ])?;

        let mut stmt = conn.prepare_cached("SELECT id FROM podcasts WHERE url = ?")?;
        let pod_id = stmt.query_row::<i64, _, _>(params![podcast.url], |row| row.get(0))?;
        let mut ep_ids = Vec::new();
        for ep in podcast.episodes.iter().rev() {
            let id = self.insert_episode(pod_id, &ep)?;
            let new_ep = NewEpisode {
                id: id,
                pod_id: pod_id,
                title: ep.title.clone(),
                pod_title: podcast.title.clone(),
                selected: false,
            };
            ep_ids.push(new_ep);
        }

        return Ok(SyncResult {
            added: ep_ids,
            updated: Vec::new(),
        });
    }

    /// Inserts a podcast episode into the database.
    pub fn insert_episode(
        &self,
        podcast_id: i64,
        episode: &EpisodeNoId,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.conn.as_ref().unwrap();

        let pubdate = match episode.pubdate {
            Some(dt) => Some(dt.timestamp()),
            None => None,
        };

        let mut stmt = conn.prepare_cached(
            "INSERT INTO episodes (podcast_id, title, url,
                description, pubdate, duration, played, hidden)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?);",
        )?;
        stmt.execute(params![
            podcast_id,
            episode.title,
            episode.url,
            episode.description,
            pubdate,
            episode.duration,
            false,
            false,
        ])?;
        return Ok(conn.last_insert_rowid());
    }

    /// Inserts a filepath to a downloaded episode.
    pub fn insert_file(
        &self,
        episode_id: i64,
        path: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.as_ref().unwrap();

        let mut stmt = conn.prepare_cached(
            "INSERT INTO files (episode_id, path)
                VALUES (?, ?);",
        )?;
        stmt.execute(params![episode_id, path.to_str(),])?;
        return Ok(());
    }

    /// Removes a file listing for an episode from the database when the
    /// user has chosen to delete the file.
    pub fn remove_file(&self, episode_id: i64) {
        let conn = self.conn.as_ref().unwrap();
        let mut stmt = conn
            .prepare_cached("DELETE FROM files WHERE episode_id = ?;")
            .unwrap();
        stmt.execute(params![episode_id]).unwrap();
    }

    /// Removes all file listings for the selected episode ids.
    pub fn remove_files(&self, episode_ids: &[i64]) {
        let conn = self.conn.as_ref().unwrap();

        // convert list of episode ids into a comma-separated String
        let episode_list: Vec<String> = episode_ids.iter().map(|x| x.to_string()).collect();
        let episodes = episode_list.join(", ");

        let mut stmt = conn
            .prepare_cached("DELETE FROM files WHERE episode_id = (?);")
            .unwrap();
        stmt.execute(params![episodes]).unwrap();
    }

    /// Removes a podcast, all episodes, and files from the database.
    pub fn remove_podcast(&self, podcast_id: i64) {
        let conn = self.conn.as_ref().unwrap();
        // Note: Because of the foreign key constraints on `episodes`
        // and `files` tables, all associated episodes for this podcast
        // will also be deleted, and all associated file entries for
        // those episodes as well.
        let mut stmt = conn
            .prepare_cached("DELETE FROM podcasts WHERE id = ?;")
            .unwrap();
        stmt.execute(params![podcast_id]).unwrap();
    }

    /// Updates an existing podcast in the database, where metadata is
    /// changed if necessary, and episodes are updated (modified episodes
    /// are updated, new episodes are inserted).
    pub fn update_podcast(
        &self,
        pod_id: i64,
        podcast: PodcastNoId,
    ) -> Result<SyncResult, Box<dyn std::error::Error>> {
        let conn = self.conn.as_ref().unwrap();
        let mut stmt = conn.prepare_cached(
            "UPDATE podcasts SET title = ?, url = ?, description = ?,
            author = ?, explicit = ?, last_checked = ?
            WHERE id = ?;",
        )?;
        stmt.execute(params![
            podcast.title,
            podcast.url,
            podcast.description,
            podcast.author,
            podcast.explicit,
            podcast.last_checked.timestamp(),
            pod_id,
        ])?;

        let result = self.update_episodes(pod_id, podcast.title, podcast.episodes);
        return Ok(result);
    }

    /// Updates metadata about episodes that already exist in database,
    /// or inserts new episodes.
    ///
    /// Episodes are checked against the URL and published data in
    /// order to determine if they already exist. As such, an existing
    /// episode that has changed either of these fields will show up as
    /// a "new" episode. The old version will still remain in the
    /// database.
    fn update_episodes(
        &self,
        podcast_id: i64,
        podcast_title: String,
        episodes: Vec<EpisodeNoId>,
    ) -> SyncResult {
        let conn = self.conn.as_ref().unwrap();

        let old_episodes = self.get_episodes(podcast_id);

        let mut insert_ep = Vec::new();
        let mut update_ep = Vec::new();
        for new_ep in episodes.iter().rev() {
            let new_pd = match new_ep.pubdate {
                Some(dt) => Some(dt.timestamp()),
                None => None,
            };

            // for each existing episode, check the title, url, and
            // pubdate -- if two of the three match, we count it as an
            // existing episode; otherwise, we add it as a new episode
            let mut existing_id = None;
            let mut update = false;
            for old_ep in old_episodes.iter().rev() {
                let mut matching = 0;
                matching += (new_ep.title == old_ep.title) as i32;
                matching += (new_ep.url == old_ep.url) as i32;

                let mut pd_match = false;
                if let Some(pd) = new_pd {
                    if let Some(old_pd) = old_ep.pubdate {
                        matching += (pd == old_pd.timestamp()) as i32;
                        pd_match = true;
                    }
                }

                if matching >= 2 {
                    existing_id = Some(old_ep.id);

                    // if we have a matching episode, check whether there
                    // are details to update
                    if !(new_ep.title == old_ep.title
                        && new_ep.url == old_ep.url
                        && new_ep.description == old_ep.description
                        && new_ep.duration == old_ep.duration
                        && pd_match)
                    {
                        update = true;
                    }
                    break;
                }
            }

            match existing_id {
                Some(id) => {
                    if update {
                        let mut stmt = conn
                            .prepare_cached(
                                "UPDATE episodes SET title = ?, url = ?,
                                description = ?, pubdate = ?, duration = ?
                                WHERE id = ?;",
                            )
                            .unwrap();
                        stmt.execute(params![
                            new_ep.title,
                            new_ep.url,
                            new_ep.description,
                            new_pd,
                            new_ep.duration,
                            id,
                        ])
                        .unwrap();
                        update_ep.push(id);
                    }
                }
                None => {
                    let id = self.insert_episode(podcast_id, &new_ep).unwrap();
                    let new_ep = NewEpisode {
                        id: id,
                        pod_id: podcast_id,
                        title: new_ep.title.clone(),
                        pod_title: podcast_title.clone(),
                        selected: false,
                    };
                    insert_ep.push(new_ep);
                }
            }
        }
        return SyncResult {
            added: insert_ep,
            updated: update_ep,
        };
    }

    /// Updates an episode to mark it as played or unplayed.
    pub fn set_played_status(&self, episode_id: i64, played: bool) {
        let conn = self.conn.as_ref().unwrap();

        let mut stmt = conn
            .prepare_cached("UPDATE episodes SET played = ? WHERE id = ?;")
            .unwrap();
        stmt.execute(params![played, episode_id]).unwrap();
    }

    /// Updates an episode to "remove" it by hiding it. "Removed"
    /// episodes need to stay in the database so that they don't get
    /// re-added when the podcast is synced again.
    pub fn hide_episode(&self, episode_id: i64, hide: bool) {
        let conn = self.conn.as_ref().unwrap();

        let mut stmt = conn
            .prepare_cached("UPDATE episodes SET hidden = ? WHERE id = ?;")
            .unwrap();
        stmt.execute(params![hide, episode_id]).unwrap();
    }

    /// Generates list of all podcasts in database.
    /// TODO: This should probably use a JOIN statement instead.
    pub fn get_podcasts(&self) -> Vec<Podcast> {
        if let Some(conn) = &self.conn {
            let mut stmt = conn.prepare_cached("SELECT * FROM podcasts;").unwrap();
            let podcast_iter = stmt
                .query_map(params![], |row| {
                    let pod_id = row.get("id")?;
                    let episodes = self.get_episodes(pod_id);

                    // create a sort title that is lowercased and removes
                    // articles from the beginning
                    let title: String = row.get("title")?;
                    let title_lower = title.to_lowercase();
                    let sort_title = RE_ARTICLES.replace(&title_lower, "").to_string();

                    Ok(Podcast {
                        id: pod_id,
                        title: title,
                        sort_title: sort_title,
                        url: row.get("url")?,
                        description: row.get("description")?,
                        author: row.get("author")?,
                        explicit: row.get("explicit")?,
                        last_checked: convert_date(row.get("last_checked")).unwrap(),
                        episodes: LockVec::new(episodes),
                    })
                })
                .unwrap();
            let mut podcasts = Vec::new();
            for pc in podcast_iter {
                podcasts.push(pc.unwrap());
            }
            podcasts.sort_unstable();

            return podcasts;
        } else {
            return Vec::new();
        }
    }

    /// Generates list of episodes for a given podcast.
    pub fn get_episodes(&self, pod_id: i64) -> Vec<Episode> {
        if let Some(conn) = &self.conn {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT * FROM episodes
                    LEFT JOIN files ON episodes.id = files.episode_id
                    WHERE episodes.podcast_id = ?
                    AND episodes.hidden = 0
                    ORDER BY pubdate DESC;",
                )
                .unwrap();
            let episode_iter = stmt
                .query_map(params![pod_id], |row| {
                    let path = match row.get::<&str, String>("path") {
                        Ok(val) => Some(PathBuf::from(val)),
                        Err(_) => None,
                    };
                    Ok(Episode {
                        id: row.get("id")?,
                        pod_id: row.get("podcast_id")?,
                        title: row.get("title")?,
                        url: row.get("url")?,
                        description: row.get("description")?,
                        pubdate: convert_date(row.get("pubdate")),
                        duration: row.get("duration")?,
                        path: path,
                        played: row.get("played")?,
                    })
                })
                .unwrap();
            let mut episodes = Vec::new();
            for ep in episode_iter {
                episodes.push(ep.unwrap());
            }
            return episodes;
        } else {
            return Vec::new();
        }
    }

    /// Deletes all rows in all tables
    pub fn clear_db(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.as_ref().unwrap();
        conn.execute("DELETE FROM files;", params![])?;
        conn.execute("DELETE FROM episodes;", params![])?;
        conn.execute("DELETE FROM podcasts;", params![])?;
        Ok(())
    }
}

/// Helper function converting an (optional) Unix timestamp to a
/// DateTime<Utc> object
fn convert_date(result: Result<i64, rusqlite::Error>) -> Option<DateTime<Utc>> {
    return match result {
        Ok(timestamp) => match NaiveDateTime::from_timestamp_opt(timestamp, 0) {
            Some(ndt) => Some(DateTime::from_utc(ndt, Utc)),
            None => None,
        },
        Err(_) => None,
    };
}
