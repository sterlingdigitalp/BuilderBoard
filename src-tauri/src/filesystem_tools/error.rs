use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesystemError {
    NotConfigured,
    PathEscape(String),
    NotFound(String),
    NotAFile(String),
    NotADirectory(String),
    BinaryFile(String),
    FileTooLarge {
        path: String,
        size_bytes: u64,
        limit_bytes: u64,
    },
    InvalidInput(String),
    Io(String),
}

impl fmt::Display for FilesystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "filesystem approved root is not configured"),
            Self::PathEscape(path) => write!(f, "path escapes approved root: {path}"),
            Self::NotFound(path) => write!(f, "path not found: {path}"),
            Self::NotAFile(path) => write!(f, "path is not a file: {path}"),
            Self::NotADirectory(path) => write!(f, "path is not a directory: {path}"),
            Self::BinaryFile(path) => write!(f, "binary file cannot be read as text: {path}"),
            Self::FileTooLarge {
                path,
                size_bytes,
                limit_bytes,
            } => write!(
                f,
                "file exceeds size limit: {path} ({size_bytes} bytes > {limit_bytes} bytes)"
            ),
            Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
            Self::Io(message) => write!(f, "io error: {message}"),
        }
    }
}

impl std::error::Error for FilesystemError {}

pub type FilesystemResult<T> = Result<T, FilesystemError>;
