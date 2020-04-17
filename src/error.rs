use thiserror::Error;

#[derive(Error, Debug)]
pub enum CreationError {
    #[error("failed to open '/dev/net/tun': file does not exists")]
    FileNotFound,
    #[error("failed to open '/dev/net/tun': permission denied")]
    PermissionDenied,
    #[error("failed to open '/dev/net/tun': {0}")]
    UnableToOpenFile(#[from] std::io::Error),
    #[error("failed to modify tun/tap device: {0}")]
    IoctlError(#[from] nix::Error),
    #[error("failed to create tun/tap device: {0}")]
    InvalidName(#[from] StringError),

}

#[derive(Error, Debug)]
pub enum StringError {
    #[error("c_string too long, can be at most {0} characters")]
    StringTooLong(usize),
    #[error("rust string contains null at position {0}")]
    UnexpectedNull(usize),
    #[error("string contains invalid character at position {0}")]
    InvalidCharacter(usize),
    #[error("c_string does not contains null terminator")]
    MangledString,
}