use std::fmt::{Debug, Display};
use std::io;

const DEFAULT_MODE_ARG: &str = "--default";
const QUIET_MODE_ARG: &str = "--quiet";

pub enum Error {
    NoSudo,
    NoExecName,
    WrongArgs(String),
    ErrRead(io::Error),
    IOErr(io::Error),
    ErrWrite,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NoSudo => write!(f, "Run with sudo!"),
            Self::NoExecName => write!(f, "No executable name found in args for some reason"),
            Self::WrongArgs(s) => write!(f, "Usage:\n{s} {DEFAULT_MODE_ARG}|{QUIET_MODE_ARG}"),
            Self::ErrRead(e) => write!(f, "Error on reading: {e}"),
            Self::IOErr(e) => write!(f, "IO error: {e}"),
            Self::ErrWrite => write!(f, "Error on writing."),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::IOErr(e)
    }
}
impl std::error::Error for Error {}
