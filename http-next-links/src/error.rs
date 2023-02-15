use std::{error, fmt};

use url::ParseError;

#[derive(Debug, Eq, PartialEq)]
enum Inner {
    Msg(&'static str),
    UrlParseError(ParseError),
}

impl fmt::Display for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inner::Msg(msg) => f.write_str(msg),
            Inner::UrlParseError(err) => fmt::Display::fmt(err, f),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Error(Inner);

impl Error {
    pub(super) const fn msg(msg: &'static str) -> Self {
        Self(Inner::Msg(msg))
    }

    pub(super) const fn url_parse_err(err: ParseError) -> Self {
        Self(Inner::UrlParseError(err))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid http header link: {}", self.0)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let Inner::UrlParseError(err) = &self.0 {
            Some(err)
        } else {
            None
        }
    }
}
