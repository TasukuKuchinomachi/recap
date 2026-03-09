use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunEntry {
    pub id: u64,
    pub cmd: String,
    pub exit: i32,
    pub at: DateTime<Utc>,
}

pub struct Storage {
    base_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .expect("Failed to find home directory")
            .join(".recap");
        Self { base_dir }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.logs_dir())
    }

    fn index_path(&self) -> PathBuf {
        self.base_dir.join("index.jsonl")
    }

    fn logs_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }

    fn log_path(&self, id: u64) -> PathBuf {
        self.logs_dir().join(format!("{:04}.log", id))
    }

    pub fn next_id(&self) -> std::io::Result<u64> {
        let entries = self.read_all_entries()?;
        Ok(entries.last().map_or(1, |e| e.id + 1))
    }

    pub fn append_entry(&self, entry: &RunEntry) -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.index_path())?;
        let line = serde_json::to_string(entry).expect("Failed to serialize entry");
        writeln!(file, "{}", line)?;
        Ok(())
    }

    pub fn save_log(&self, id: u64, output: &[u8]) -> std::io::Result<()> {
        fs::write(self.log_path(id), output)
    }

    pub fn read_log(&self, id: u64) -> std::io::Result<String> {
        fs::read_to_string(self.log_path(id))
    }

    pub fn read_all_entries(&self) -> std::io::Result<Vec<RunEntry>> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: RunEntry = serde_json::from_str(&line)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            entries.push(entry);
        }
        Ok(entries)
    }

    pub fn get_last(&self) -> std::io::Result<Option<RunEntry>> {
        let entries = self.read_all_entries()?;
        Ok(entries.into_iter().last())
    }

    pub fn get_last_n(&self, n: usize) -> std::io::Result<Vec<RunEntry>> {
        let entries = self.read_all_entries()?;
        let start = entries.len().saturating_sub(n);
        Ok(entries[start..].to_vec())
    }

    pub fn get_by_id(&self, id: u64) -> std::io::Result<Option<RunEntry>> {
        let entries = self.read_all_entries()?;
        Ok(entries.into_iter().find(|e| e.id == id))
    }
}
