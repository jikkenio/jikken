use log::{Level, Log, Metadata, Record};

pub struct SimpleLogger {
    pub level: Level,
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        match record.level() {
            Level::Info => {
                println!("{}", record.args());
            }
            Level::Error => {
                println!("{}", record.args());
            }
            Level::Trace => {
                println!("{}", record.args());
            }
            _ => {}
        }
    }

    fn flush(&self) {
        println!("flush called!")
    }
}
