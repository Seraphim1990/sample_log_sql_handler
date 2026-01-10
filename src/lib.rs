use std::cell::RefCell;
use rusqlite::{params, Connection, Result};
use sample_logger::{LogHandler, LogRecord};

pub struct SqliteLogHandler {
    connection: Connection,
    level: i32,
    max_push: i32,
    buffer: Vec<(String, String, String)>,
    status: RefCell<bool>
}

pub struct SqliteLogBuilder {
    connection: String,
    level: i32,
    max_push: i32,
}

impl SqliteLogBuilder {
    pub fn new() -> Self {
        Self {
            connection: "log.sqlite".to_string(),
            level: i32::MIN,
            max_push: 10,
        }
    }
    pub fn level(&mut self, level: i32) -> &mut Self {
        self.level = level;
        self
    }
    pub fn max_push(&mut self, max_push: i32) -> &mut Self {
        self.max_push = max_push;
        self
    }
    pub fn connection(&mut self, connection: &str) -> &mut Self {
        self.connection = connection.to_string();
        self
    }
    pub fn build(&self) -> Result<SqliteLogHandler> {
        let _connection = Connection::open(&self.connection)?;
        _connection.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                heading TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;
        Ok(
            SqliteLogHandler {
                connection: _connection,
                level: self.level,
                max_push: self.max_push,
                buffer: Vec::with_capacity(self.max_push as usize),
                status: RefCell::new(true)
            }
        )
    }
}

impl SqliteLogHandler {
    pub fn new(path: &str) -> Result<Self> {
        let _connection = Connection::open(path)?;
        _connection.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                heading TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;
        Ok(
            Self {
                connection: _connection,
                level: i32::MIN,
                max_push: 10,
                buffer: Vec::with_capacity(10),
                status: RefCell::new(true)
            }
        )
    }
}

impl LogHandler for SqliteLogHandler {
    fn handle(&mut self, record: &LogRecord) {
        if record.lvl < self.level || !*self.status.borrow(){
            return;
        }
        self.buffer.push((record.heading.to_string(), record.msg.clone(), record.timestamp.to_string()));

        // Якщо досягли max_push - коміт
        if self.buffer.len() >= self.max_push as usize {
            self.flush();
        }
    }
    fn flush(&mut self) {
        if self.buffer.is_empty() || !*self.status.borrow() { return; }

        let tx = match self.connection.transaction() {
            Ok(tx) => tx,
            Err(e) => {
                *self.status.borrow_mut() = false;
                return;
            },
        };

        self.buffer.iter().for_each(|(heading, message, timestamp)| {
            tx.execute(
                "INSERT INTO logs (heading, message, timestamp) VALUES (?1, ?2, ?3)",
                params![heading, message, timestamp]
            ).unwrap();
        });
        tx.commit().unwrap();
        self.buffer.clear();
    }
}

impl Drop for SqliteLogHandler {
    fn drop(&mut self) {
        self.flush();
    }
}

#[cfg(test)]
mod tests;
mod write_test;
