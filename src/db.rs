/**
 * BlissMixer: Use Bliss analysis results to create music mixes
 *
 * Copyright (c) 2022-2026 Craig Drummond <craig.p.drummond@gmail.com>
 * GPLv3 license.
 *
 **/

use crate::tree;
use rusqlite::Connection;
use std::collections::HashSet;

pub static mut WEIGHTS: [f32;tree::DIMENSIONS] = [1.0;tree::DIMENSIONS];

pub struct Metadata {
    pub file: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub duration: Option<u32>,
    pub tempo: Option<f32>,
}

pub struct Db {
    pub conn: Connection,
}

pub fn init_weights(weights_str: &String) {
    let vals = weights_str.split(",");
    let mut pos = 0;
    unsafe {
        for val in vals {
            if pos<tree::DIMENSIONS {
                WEIGHTS[pos] = val.parse::<f32>().unwrap();
            }
            pos+=1;
        }
        log::debug!("Weights: {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}", 
            WEIGHTS[0], WEIGHTS[1], WEIGHTS[2], WEIGHTS[3], WEIGHTS[4],
            WEIGHTS[5], WEIGHTS[6], WEIGHTS[7], WEIGHTS[8], WEIGHTS[9],
            WEIGHTS[10], WEIGHTS[11], WEIGHTS[12], WEIGHTS[13], WEIGHTS[14],
            WEIGHTS[15], WEIGHTS[16], WEIGHTS[17], WEIGHTS[18], WEIGHTS[19],
            WEIGHTS[20], WEIGHTS[21], WEIGHTS[22]);
    }
}

fn adjust(vals: [f32;tree::DIMENSIONS]) -> [f32;tree::DIMENSIONS] {
    let mut adjusted: [f32;tree::DIMENSIONS] = [0.0;tree::DIMENSIONS];
    unsafe {
        for (i, x) in vals.iter().enumerate() {
            adjusted[i] = x * WEIGHTS[i];
        }
    }
    adjusted
}

impl Db {
    pub fn new(path: &String) -> Self {
        Self {
            conn: Connection::open(path).unwrap(),
        }
    }

    pub fn close(self) {
        if let Err(e) = self.conn.close() {
            log::debug!("Error closing database: {:?}", e);
        }
    }

    pub fn load(&self) -> tree::AnalysisDetails {
        log::debug!("Load tree");
        let mut details = tree::AnalysisDetails::new();
        match self.conn.prepare("SELECT Tempo, Zcr, MeanSpectralCentroid, StdDevSpectralCentroid, MeanSpectralRolloff, StdDevSpectralRolloff, MeanSpectralFlatness, StdDevSpectralFlatness, MeanLoudness, StdDevLoudness, Chroma1, Chroma2, Chroma3, Chroma4, Chroma5, Chroma6, Chroma7, Chroma8, Chroma9, Chroma10, Chroma11, Chroma12, Chroma13, rowid FROM TracksV2 WHERE Ignore IS NOT 1") {
            Ok(mut stmt) => {
                let track_iter = stmt.query_map([], |row| {
                    Ok((row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                        row.get(13)?,
                        row.get(14)?,
                        row.get(15)?,
                        row.get(16)?,
                        row.get(17)?,
                        row.get(18)?,
                        row.get(19)?,
                        row.get(20)?,
                        row.get(21)?,
                        row.get(22)?,
                        row.get(23)?
                    ))
                }).unwrap();
                let mut num_loaded = 0;
                for tr in track_iter {
                    let track = tr.unwrap();
                    let vals:[f32;tree::DIMENSIONS] = [
                                track.0,
                                track.1,
                                track.2,
                                track.3,
                                track.4,
                                track.5,
                                track.6,
                                track.7,
                                track.8,
                                track.9,
                                track.10,
                                track.11,
                                track.12,
                                track.13,
                                track.14,
                                track.15,
                                track.16,
                                track.17,
                                track.18,
                                track.19,
                                track.20,
                                track.21,
                                track.22];
                    num_loaded += 1;
                    details.values.push(adjust(vals));
                    details.ids.push(track.23);
                }
                log::debug!("Tree loaded {} track(s)", num_loaded);
            }
            Err(e) => { log::error!("Failed to load tree from DB. {}", e); }
        }
        details
    }

    pub fn load_artist_tree(&self, artist: &str) -> tree::AnalysisDetails {
        log::debug!("Load artist '{}' tree", artist);
        let mut details = tree::AnalysisDetails::new();
        match self.conn.prepare("SELECT Tempo, Zcr, MeanSpectralCentroid, StdDevSpectralCentroid, MeanSpectralRolloff, StdDevSpectralRolloff, MeanSpectralFlatness, StdDevSpectralFlatness, MeanLoudness, StdDevLoudness, Chroma1, Chroma2, Chroma3, Chroma4, Chroma5, Chroma6, Chroma7, Chroma8, Chroma9, Chroma10, Chroma11, Chroma12, Chroma13, rowid FROM TracksV2 WHERE Artist=:artist;") {
            Ok(mut stmt) => {
                let track_iter = stmt.query_map(&[(":artist", &artist)], |row| {
                    Ok((row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                        row.get(13)?,
                        row.get(14)?,
                        row.get(15)?,
                        row.get(16)?,
                        row.get(17)?,
                        row.get(18)?,
                        row.get(19)?,
                        row.get(20)?,
                        row.get(21)?,
                        row.get(22)?,
                        row.get(23)?
                    ))
                }).unwrap();
                let mut num_loaded = 0;
                for tr in track_iter {
                    let track = tr.unwrap();
                    let vals:[f32;tree::DIMENSIONS] = [
                                track.0,
                                track.1,
                                track.2,
                                track.3,
                                track.4,
                                track.5,
                                track.6,
                                track.7,
                                track.8,
                                track.9,
                                track.10,
                                track.11,
                                track.12,
                                track.13,
                                track.14,
                                track.15,
                                track.16,
                                track.17,
                                track.18,
                                track.19,
                                track.20,
                                track.21,
                                track.22];
                    num_loaded += 1;
                    details.values.push(adjust(vals));
                    details.ids.push(track.23);
                }
                log::debug!("Tree loaded {} track(s)", num_loaded);
            }
            Err(e) => { log::error!("Failed to load tree from DB. {}", e); }
        }
        details
    }

    pub fn get_rowid(&self, path: &str) -> u64 {
        let mut id: u64 = 0;
        if let Ok(mut stmt) = self.conn.prepare("SELECT rowid FROM TracksV2 WHERE File=:path;") {
            if let Ok(val) = stmt.query_row(&[(":path", &path)], |row| row.get(0)) {
                id = val;
            }
        }
        id
    }

    pub fn get_all_genres(&self) -> HashSet<String> {
        log::debug!("getting genres from db.");
        let mut all_available_genres = HashSet::new();

        match self.conn.prepare("SELECT DISTINCT Genre FROM TracksV2 WHERE ignore IS NOT 1;") {
            Ok(mut stmt) => match stmt.query_map([], |row| Ok(row.get::<_, Option<String>>(0)?)) {
                Ok(column) => {
                    for item in column {
                        let item_content = item.unwrap().unwrap();
                        let item_genres: Vec<&str> = item_content.split(";").collect();
                        for genre in item_genres {
                            let trimmed_genre = genre.trim();
                            if !trimmed_genre.is_empty() {
                                all_available_genres.insert(String::from(trimmed_genre));
                            }
                        }
                    }
                }
                Err(e) => { log::debug!("Failed to read all genres: {}", e); }
            }
            Err(e) => { log::debug!("Failed to read all genres: {}", e); }
        }
        all_available_genres
    }

    pub fn get_metadata(&self, id: u64) -> Result<Metadata, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT File, Title, Artist, AlbumArtist, Album, Genre, Duration, Tempo FROM TracksV2 WHERE rowid=:rowid;")?;
        let row = stmt.query_row(&[(":rowid", &id)], |row| {
                Ok(Metadata {
                    file: row.get(0)?,
                    title: row.get(1)?,
                    artist: row.get(2)?,
                    album_artist: row.get(3)?,
                    album: row.get(4)?,
                    genre: row.get(5)?,
                    duration: row.get(6)?,
                    tempo: row.get(7)?,
                })
            }).unwrap();
        Ok(row)
    }

    pub fn get_metrics(&self, id: u64) -> Result<[f32; tree::DIMENSIONS], rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT Tempo, Zcr, MeanSpectralCentroid, StdDevSpectralCentroid, MeanSpectralRolloff, StdDevSpectralRolloff, MeanSpectralFlatness, StdDevSpectralFlatness, MeanLoudness, StdDevLoudness, Chroma1, Chroma2, Chroma3, Chroma4, Chroma5, Chroma6, Chroma7, Chroma8, Chroma9, Chroma10, Chroma11, Chroma12, Chroma13 FROM TracksV2 WHERE rowid=:rowid;").unwrap();
        let row = stmt.query_row(&[(":rowid", &id)], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                ))
            }).unwrap();
        let metrics: [f32; tree::DIMENSIONS] = [
            row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10, row.11,
            row.12, row.13, row.14, row.15, row.16, row.17, row.18, row.19, row.20, row.21, row.22
        ];
        Ok(adjust(metrics))
    }

    pub fn get_raw_metrics(&self, id: u64) -> Result<[f32; tree::DIMENSIONS], rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT Tempo, Zcr, MeanSpectralCentroid, StdDevSpectralCentroid, MeanSpectralRolloff, StdDevSpectralRolloff, MeanSpectralFlatness, StdDevSpectralFlatness, MeanLoudness, StdDevLoudness, Chroma1, Chroma2, Chroma3, Chroma4, Chroma5, Chroma6, Chroma7, Chroma8, Chroma9, Chroma10, Chroma11, Chroma12, Chroma13 FROM TracksV2 WHERE rowid=:rowid;").unwrap();
        let row = stmt.query_row(&[(":rowid", &id)], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                ))
            }).unwrap();
        let metrics: [f32; tree::DIMENSIONS] = [
            row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10, row.11,
            row.12, row.13, row.14, row.15, row.16, row.17, row.18, row.19, row.20, row.21, row.22
        ];
        Ok(metrics)
    }

    pub fn get_all_raw_metrics(&self) -> Vec<(u64, [f32; tree::DIMENSIONS])> {
        let mut results: Vec<(u64, [f32; tree::DIMENSIONS])> = Vec::new();
        match self.conn.prepare("SELECT Tempo, Zcr, MeanSpectralCentroid, StdDevSpectralCentroid, MeanSpectralRolloff, StdDevSpectralRolloff, MeanSpectralFlatness, StdDevSpectralFlatness, MeanLoudness, StdDevLoudness, Chroma1, Chroma2, Chroma3, Chroma4, Chroma5, Chroma6, Chroma7, Chroma8, Chroma9, Chroma10, Chroma11, Chroma12, Chroma13, rowid FROM TracksV2 WHERE ignore IS NOT 1;") {
            Ok(mut stmt) => {
                let track_iter = stmt.query_map([], |row| {
                    Ok((
                        row.get(0)?,  row.get(1)?,  row.get(2)?,  row.get(3)?,
                        row.get(4)?,  row.get(5)?,  row.get(6)?,  row.get(7)?,
                        row.get(8)?,  row.get(9)?,  row.get(10)?, row.get(11)?,
                        row.get(12)?, row.get(13)?, row.get(14)?, row.get(15)?,
                        row.get(16)?, row.get(17)?, row.get(18)?, row.get(19)?,
                        row.get(20)?, row.get(21)?, row.get(22)?, row.get::<_, u64>(23)?,
                    ))
                }).unwrap();
                for tr in track_iter {
                    let t = tr.unwrap();
                    let metrics: [f32; tree::DIMENSIONS] = [
                        t.0,  t.1,  t.2,  t.3,  t.4,  t.5,  t.6,  t.7,
                        t.8,  t.9,  t.10, t.11, t.12, t.13, t.14, t.15,
                        t.16, t.17, t.18, t.19, t.20, t.21, t.22,
                    ];
                    results.push((t.23, metrics));
                }
            }
            Err(e) => { log::error!("Failed to load all raw metrics: {}", e); }
        }
        log::debug!("Loaded {} raw metrics for full scan", results.len());
        results
    }

    pub fn get_raw_metrics_by_artist(
        &self,
        artist: &str,
    ) -> Vec<(u64, [f32; tree::DIMENSIONS])> {
        let mut results: Vec<(u64, [f32; tree::DIMENSIONS])> = Vec::new();
        match self.conn.prepare("SELECT Tempo, Zcr, MeanSpectralCentroid, StdDevSpectralCentroid, MeanSpectralRolloff, StdDevSpectralRolloff, MeanSpectralFlatness, StdDevSpectralFlatness, MeanLoudness, StdDevLoudness, Chroma1, Chroma2, Chroma3, Chroma4, Chroma5, Chroma6, Chroma7, Chroma8, Chroma9, Chroma10, Chroma11, Chroma12, Chroma13, rowid FROM TracksV2 WHERE Artist=:artist;") {
            Ok(mut stmt) => {
                let track_iter = stmt.query_map(&[(":artist", &artist)], |row| {
                    Ok((
                        row.get(0)?,  row.get(1)?,  row.get(2)?,  row.get(3)?,
                        row.get(4)?,  row.get(5)?,  row.get(6)?,  row.get(7)?,
                        row.get(8)?,  row.get(9)?,  row.get(10)?, row.get(11)?,
                        row.get(12)?, row.get(13)?, row.get(14)?, row.get(15)?,
                        row.get(16)?, row.get(17)?, row.get(18)?, row.get(19)?,
                        row.get(20)?, row.get(21)?, row.get(22)?, row.get::<_, u64>(23)?,
                    ))
                }).unwrap();
                for tr in track_iter {
                    let t = tr.unwrap();
                    let metrics: [f32; tree::DIMENSIONS] = [
                        t.0,  t.1,  t.2,  t.3,  t.4,  t.5,  t.6,  t.7,
                        t.8,  t.9,  t.10, t.11, t.12, t.13, t.14, t.15,
                        t.16, t.17, t.18, t.19, t.20, t.21, t.22,
                    ];
                    results.push((t.23, metrics));
                }
            }
            Err(e) => {
                log::error!("Failed to load raw metrics for artist '{}': {}", artist, e);
            }
        }
        log::debug!(
            "Loaded {} raw metrics for artist '{}'",
            results.len(),
            artist
        );
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_db() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE TracksV2 (
                File TEXT, Title TEXT, Artist TEXT, AlbumArtist TEXT, Album TEXT,
                Genre TEXT, Duration INTEGER,
                Tempo REAL, Zcr REAL, MeanSpectralCentroid REAL,
                StdDevSpectralCentroid REAL, MeanSpectralRolloff REAL,
                StdDevSpectralRolloff REAL, MeanSpectralFlatness REAL,
                StdDevSpectralFlatness REAL, MeanLoudness REAL,
                StdDevLoudness REAL, Chroma1 REAL, Chroma2 REAL, Chroma3 REAL,
                Chroma4 REAL, Chroma5 REAL, Chroma6 REAL, Chroma7 REAL,
                Chroma8 REAL, Chroma9 REAL, Chroma10 REAL, Chroma11 REAL,
                Chroma12 REAL, Chroma13 REAL, Ignore INTEGER
            );
            INSERT INTO TracksV2 VALUES (
                'fixture.flac', 'Fixture title', 'Fixture artist',
                'Fixture album artist', 'Fixture album', 'Rock; Electronic', 181,
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                18, 19, 20, 21, 22, 23, 0
            );
            INSERT INTO TracksV2 VALUES (
                'ignored.flac', 'Ignored', 'Ignored artist', 'Ignored artist',
                'Ignored album', 'Ignored', 182,
                101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
                113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 1
            );
            "#,
        )
        .unwrap();
        Db { conn }
    }

    #[test]
    fn raw_metrics_follow_the_canonical_twenty_three_column_order() {
        let db = fixture_db();
        let metrics = db.get_raw_metrics(1).unwrap();
        for (index, value) in metrics.iter().enumerate() {
            assert_eq!(*value, (index + 1) as f32);
        }
    }

    #[test]
    fn metadata_and_row_identity_match_tracksv2() {
        let db = fixture_db();
        assert_eq!(db.get_rowid("fixture.flac"), 1);
        assert_eq!(db.get_rowid("missing.flac"), 0);
        let metadata = db.get_metadata(1).unwrap();
        assert_eq!(metadata.file, "fixture.flac");
        assert_eq!(metadata.title.as_deref(), Some("Fixture title"));
        assert_eq!(metadata.artist.as_deref(), Some("Fixture artist"));
        assert_eq!(metadata.album_artist.as_deref(), Some("Fixture album artist"));
        assert_eq!(metadata.album.as_deref(), Some("Fixture album"));
        assert_eq!(metadata.genre.as_deref(), Some("Rock; Electronic"));
        assert_eq!(metadata.duration, Some(181));
        assert_eq!(metadata.tempo, Some(1.0));
    }

    #[test]
    fn full_scans_and_genres_exclude_ignored_rows() {
        let db = fixture_db();
        let metrics = db.get_all_raw_metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].0, 1);
        let genres = db.get_all_genres();
        assert_eq!(genres.len(), 2);
        assert!(genres.contains("Rock"));
        assert!(genres.contains("Electronic"));
    }

    #[test]
    fn artist_raw_scan_uses_exact_artist_and_preserves_list_semantics() {
        let db = fixture_db();
        let fixture = db.get_raw_metrics_by_artist("Fixture artist");
        assert_eq!(fixture.len(), 1);
        assert_eq!(fixture[0].0, 1);
        assert_eq!(fixture[0].1[0], 1.0);

        let ignored = db.get_raw_metrics_by_artist("Ignored artist");
        assert_eq!(ignored.len(), 1);
        assert_eq!(ignored[0].0, 2);
        assert_eq!(ignored[0].1[0], 101.0);
        assert!(db.get_raw_metrics_by_artist("fixture artist").is_empty());
    }
}
