use log::{Level, Log, Metadata, Record};

pub struct SimpleLogger {
    pub level: Level,
    pub disabled: bool,
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

        match record.level() {
            Level::Info => {
                print!("{}", record.args());
            }
            Level::Warn => {
                println!("\x1b[33m{}\x1b[0m", record.args());
            }
            Level::Error => {
                println!("\x1b[31m{}\x1b[0m", record.args());
            }
            _ => println!("{}", record.args()),
        }
    }

    fn flush(&self) {
        println!("flush called!")
    }
}
