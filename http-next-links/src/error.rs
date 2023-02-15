use std::{error, fmt};

#[derive(Debug, Eq, PartialEq)]
pub struct Error(&'static str);

impl Error {
    pub(super) const fn msg(msg: &'static str) -> Self {
        Self(msg)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid http header link: {}", self.0)
    }
}

impl error::Error for Error {}
