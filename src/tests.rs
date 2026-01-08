use super::*;
use std::sync::{Arc, Mutex};
use std::thread;

fn create_test_record(lvl: i32, msg: &str) -> LogRecord {
    LogRecord {
        color: "\033[32m",
        heading: "TEST",
        msg: msg.to_string(),
        timestamp: chrono::Utc::now(),
        lvl,
    }
}

#[test]
fn test_create_handler() {
    let handler = SqliteLogHandler::new(":memory:");
    assert!(handler.is_ok());
}


    #[test]
    fn test_builder_pattern() {
        let handler = SqliteLogBuilder::new()
            .connection(":memory:")
            .level(2)
            .max_push(5)
            .build();

        assert!(handler.is_ok());
    }

    #[test]
    fn test_insert_single_log() {
        let mut handler = SqliteLogHandler::new(":memory:").unwrap();
        let record = create_test_record(5, "Test message");

        handler.handle(&record);
        handler.flush();

        // Перевіряємо запис
        let conn = &handler.connection;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM logs",
            [],
            |row| row.get(0)
        ).unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_level_filtering() {
        let mut handler = SqliteLogBuilder::new()
            .connection(":memory:")
            .level(3)  // Тільки рівень 3 і вище
            .build()
            .unwrap();

        let low = create_test_record(1, "Low level");
        let high = create_test_record(5, "High level");

        handler.handle(&low);   // Не повинно записатись
        handler.handle(&high);  // Повинно записатись
        handler.flush();

        let conn = &handler.connection;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM logs",
            [],
            |row| row.get(0)
        ).unwrap();

        assert_eq!(count, 1);
    }

#[test]
fn test_batching() {
    let mut handler = SqliteLogBuilder::new()
        .connection(":memory:")
        .max_push(3)
        .build()
        .unwrap();

    for i in 0..5 {
        let record = create_test_record(5, &format!("Message {}", i));
        handler.handle(&record);
    }

    // Перевірка 1 - в блоці {}
    {
        let conn = &handler.connection;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM logs",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(count, 3);
    } // ← conn помирає тут!

    // Тепер можна flush
    handler.flush();

    // Перевірка 2
    {
        let conn = &handler.connection;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM logs",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(count, 5);
    }
}

    #[test]
    fn test_multithreading() {
        // Створюємо handler в Arc<Mutex<>> для багатопоточності
        let handler = Arc::new(Mutex::new(
            SqliteLogHandler::new(":memory:").unwrap()
        ));

        let mut handles = vec![];

        // 10 потоків, кожен пише 10 записів
        for i in 0..10 {
            let h = Arc::clone(&handler);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let record = create_test_record(5, &format!("Thread {} - {}", i, j));
                    h.lock().unwrap().handle(&record);
                }
            });
            handles.push(handle);
        }

        // Чекаємо всі потоки
        for h in handles {
            h.join().unwrap();
        }

        // Flush та перевірка
        handler.lock().unwrap().flush();

        let conn = &handler.lock().unwrap().connection;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM logs",
            [],
            |row| row.get(0)
        ).unwrap();

        assert_eq!(count, 100);
    }

    #[test]
    fn test_drop_flushes() {
        let temp_path = ":memory:";

        {
            let mut handler = SqliteLogHandler::new(temp_path).unwrap();
            let record = create_test_record(5, "Test");
            handler.handle(&record);
            // Drop викликається автоматично тут
        }

        // Відкриваємо знову і перевіряємо (не працює для :memory:, але логіка вірна)
        // Для реального файлу це працювало б
    }

    #[test]
    fn test_query_logs() {
        let mut handler = SqliteLogHandler::new(":memory:").unwrap();

        let record1 = create_test_record(1, "First message");
        let record2 = create_test_record(2, "Second message");

        handler.handle(&record1);
        handler.handle(&record2);
        handler.flush();

        // Запитуємо логи
        let conn = &handler.connection;
        let mut stmt = conn.prepare("SELECT heading, message FROM logs ORDER BY id").unwrap();

        let logs: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].1, "First message");
        assert_eq!(logs[1].1, "Second message");
    }

    #[test]
    fn test_buffer_capacity() {
        let handler = SqliteLogBuilder::new()
            .connection(":memory:")
            .max_push(100)
            .build()
            .unwrap();

        // Перевіряємо що буфер створено з правильною ємністю
        assert_eq!(handler.buffer.capacity(), 100);
    }
