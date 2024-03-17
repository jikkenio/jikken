use log::{Level, Log, Metadata, Record};
use std::sync::Mutex;

pub struct SimpleLogger {
    pub level: Level,
    pub disabled: bool,
    pub buffered: bool,
    queue: Mutex<Vec<(Level, String)>>,
}

impl SimpleLogger {
    pub fn new(level: Level, disabled: bool, buffered: bool) -> SimpleLogger {
        SimpleLogger {
            level: level,
            disabled: disabled,
            buffered: buffered,
            queue: Mutex::new(vec![]),
        }
    }

    fn print(&self, level: &Level, message: &String) {
        match level {
            Level::Info => {
                print!("{}", message);
            }
            Level::Warn => {
                println!("\x1b[33m{}\x1b[0m", message);
            }
            Level::Error => {
                println!("\x1b[31m{}\x1b[0m", message);
            }
            _ => println!("{}", message),
        }
    }
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.disabled {
            return;
        }

        if !self.enabled(record.metadata()) {
            return;
        }

        if !self.buffered {
            self.print(&record.level(), &format!("{}", record.args()));
            return;
        }

        match record.level() {
            Level::Info => {
                self.print(&Level::Info, &format!("{}", record.args()));
            }
            _ => {
                let mut q = self.queue.lock().unwrap();
                q.push((record.level(), format!("{}", record.args())));
            }
        }
    }

    fn flush(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.iter().for_each(|(level, message)| {
            self.print(level, message);
        });
        queue.clear();
    }
}
