use std::fmt::Display;
use bitflags::bitflags;

bitflags! {
    pub struct OpenFlag: u32 {
        const O_RNONLY = 0b0001;
        const O_WRONLY = 0b0010;
        const O_APPEND  = 0b0100;
        const O_CREAT  = 0b1000;
    }
}

#[derive(Debug)]
pub struct MemFSErr {
    pub message: String,
    pub err_type: MemFSErrType,
}

#[derive(Debug)]
enum MemFSErrType {
    PoisonedLock,
    ENOENT,
    Misc
}

impl Display for MemFSErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = &self.message;
        write!(f, "{message}")
    }
}

impl MemFSErr {
    pub fn with_message(message: &str) -> Self {
        Self {
            message: message.to_string(),
            err_type: MemFSErrType::Misc,
        }
    }

    pub fn no_such_file_or_directory() -> Self {
        Self {
            message: "No such file or directory".to_string(),
            err_type: MemFSErrType::ENOENT,
        }
    }

    pub fn poisoned_lock() -> Self {
        Self {
            message: "Lock poison error".to_string(),
            err_type: MemFSErrType::PoisonedLock,
        }
    }
}

pub type Result<T> = std::result::Result<T, MemFSErr>;