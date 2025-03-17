use bitflags::bitflags;
use rand::Rng;
use std::fmt::Display;

bitflags! {
    #[derive(Clone)]
    pub struct OpenFlag: u32 {
        const O_RDONLY = 0b1;
        const O_WRONLY = 0b10;
        const O_RDWR = 0b100;
        const O_CREAT  = 0b1000;
        const O_EXCL = 0b10000;
    }
}

impl OpenFlag {
    pub fn check_mode_exclusiveness(&self) -> bool {
        let exclusive_flags = Self::O_RDONLY.bits() | Self::O_RDWR.bits() | Self::O_WRONLY.bits();
        let and_flag = self.bits() & exclusive_flags;

        and_flag.count_ones() == 1
    }
}

#[allow(non_camel_case_types)]
pub enum SeekFlag {
    SEEK_CUR,
    SEEK_END,
    SEEK_SET,
}

#[derive(Debug)]
pub struct MemFSErr {
    pub message: String,
    pub err_type: MemFSErrType,
}

#[derive(Debug)]
pub enum MemFSErrType {
    /// Used on poisoned lock error.
    PoisonedLock,

    /// Used when there is no entry with the given name.
    ENOENT,

    /// Used when there is already an entry with the given name.
    EEXIST,

    /// Used when the given usize integer is not a valid file descriptor,
    /// or the file descriptor is not opened with appropriate read/write privilege.
    EBADF,

    /// Used when the target should be a file, but is a directory.
    EISDIR,

    /// Used when the target should be a directory, but is a file.
    ENOTDIR,

    /// Used on memory fault, such as out of bound error.
    EFAULT,

    /// Used when the provided value is invalid.
    /// It is used on open flag.
    EINVAL,

    /// Used when directory is not empty.
    ENOTEMPTY,

    /// Miscellaneous
    Misc,
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

    pub fn bad_file_descriptor() -> Self {
        Self {
            message: "Not a file descriptor".to_string(),
            err_type: MemFSErrType::EBADF,
        }
    }

    pub fn is_directory() -> Self {
        Self {
            message: "Is a directory".to_string(),
            err_type: MemFSErrType::EISDIR,
        }
    }

    pub fn is_not_directory() -> Self {
        Self {
            message: "Is not a directory".to_string(),
            err_type: MemFSErrType::ENOTDIR,
        }
    }

    pub fn bad_memory_access() -> Self {
        Self {
            message: "Bad memory access".to_string(),
            err_type: MemFSErrType::EFAULT,
        }
    }

    pub fn already_exists() -> Self {
        Self {
            message: "An entry with name already exists".to_string(),
            err_type: MemFSErrType::EEXIST,
        }
    }

    pub fn invalid_value() -> Self {
        Self {
            message: "Invalid value".to_string(),
            err_type: MemFSErrType::EINVAL,
        }
    }

    pub fn is_not_empty() -> Self {
        Self {
            message: "Directory is not empty".to_string(),
            err_type: MemFSErrType::ENOTEMPTY,
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

pub fn generate_random_vector(capacity: usize) -> Vec<u8> {
    let mut rng = rand::rng();
    (0..capacity).map(|_| rng.random::<u8>()).collect()
}
