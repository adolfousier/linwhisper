use rusqlite::{Connection, Result, params};
use std::path::Path;

pub struct Db {
    conn: Connection,
}

pub struct Transcription {
    pub _id: i64,
    pub text: String,
    pub created_at: String,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transcriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn insert(&self, text: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO transcriptions (text) VALUES (?1)",
            params![text],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<Transcription>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, created_at FROM transcriptions ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(Transcription {
                _id: row.get(0)?,
                text: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        rows.collect()
    }
}
