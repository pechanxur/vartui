#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        use std::io::Write;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("debug.log") {
            let _ = writeln!(file, "[{}] {}", now, format!($($arg)*));
        }
    })
}
