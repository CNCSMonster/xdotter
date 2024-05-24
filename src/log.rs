#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Verbose = 3,
    Warn = 4,
    Error = 5,
    Fatal = 6,
    Off = 7,
}
impl PartialOrd for LogLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as i32).cmp(&(*other as i32))
    }
}

pub fn set_level(level: LogLevel) {
    unsafe {
        LOG_LEVEL = level;
    }
}
static mut LOG_LEVEL: LogLevel = LogLevel::Info;
pub fn get_level() -> LogLevel {
    unsafe { LOG_LEVEL }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        if $level >= $crate::get_level() {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::log!(LogLevel::Trace, $($arg)*);
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::log!(LogLevel::Debug, $($arg)*);
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log!(LogLevel::Info, $($arg)*);
    };
}

#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => {
        $crate::log!(LogLevel::Verbose, $($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log!(LogLevel::Warn, $($arg)*);
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log!(LogLevel::Error, $($arg)*);
    };
}

#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => {
        $crate::log!(LogLevel::Fatal, $($arg)*);
    };
}
