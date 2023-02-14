use std::{error, fmt};

#[derive(Debug)]
pub struct Error(pub(super) &'static str);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid link segment: {}", self.0)
    }
}

impl error::Error for Error {}
