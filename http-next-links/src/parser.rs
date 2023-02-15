use std::iter;

use crate::Error;

trait StrExt {
    const HTTP_WHITESPACE: [char; 2] = [' ', '\t'];

    fn trim_start_http_whitespaces(&self) -> &Self;
    fn trim_end_http_whitespaces(&self) -> &Self;
}

impl StrExt for str {
    fn trim_start_http_whitespaces(&self) -> &Self {
        self.trim_start_matches(&Self::HTTP_WHITESPACE[..])
    }
    fn trim_end_http_whitespaces(&self) -> &Self {
        self.trim_end_matches(&Self::HTTP_WHITESPACE[..])
    }
}

pub(super) enum ParamsIter<'a> {
    Params(&'a str),
    NextUri(&'a str),
}

impl<'a> ParamsIter<'a> {
    fn new(rest: &'a str) -> Result<Self, Error> {
        Self::new_without_trim(rest.trim_start_http_whitespaces())
    }

    fn new_without_trim(rest: &'a str) -> Result<Self, Error> {
        if let Some(params) = rest.strip_prefix(';') {
            Ok(ParamsIter::Params(params.trim_start_http_whitespaces()))
        } else if let Some(next_uri) = rest.strip_prefix(',') {
            Ok(ParamsIter::NextUri(next_uri.trim_start_http_whitespaces()))
        } else if rest.is_empty() {
            Ok(ParamsIter::NextUri(rest))
        } else {
            Err(Error(
                "Expected either ';' for next param, ',' for next uri or an empty string for termination",
            ))
        }
    }

    /// Only call this when `<Self as Iterator>::next` returns `None`.
    pub(super) fn into_next_uri(self) -> Option<&'a str> {
        if let ParamsIter::NextUri(next_uri) = self {
            Some(next_uri)
        } else {
            None
        }
    }
}

impl iter::FusedIterator for ParamsIter<'_> {}

impl<'a> Iterator for ParamsIter<'a> {
    type Item = Result<(&'a str, &'a str), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let ParamsIter::Params(params) = *self else { return None };

        let mut f = || -> Result<_, Error> {
            let (name, rest) = params.split_once('=').ok_or(Error("Expected param"))?;

            let name = name.trim_end_http_whitespaces();

            let rest = rest.trim_start_http_whitespaces();
            let value = if let Some(rest) = rest.strip_prefix('"') {
                // Parse quoted value
                let (value, rest) = rest
                    .split_once('"')
                    .ok_or(Error("Unclosed '\"' in param value"))?;

                *self = Self::new(rest)?;

                value
            } else if let Some(delimiter_index) = rest.find([',', ';']) {
                // Find next delimiter

                // We know that at index delimiter_index there must be either
                // ',' or ';', so we use unwrap `str::get` and then call
                // new_without_trim here.
                *self = ParamsIter::new_without_trim(rest.get(delimiter_index..).unwrap())?;

                rest.get(..delimiter_index).unwrap()
            } else {
                // There is no delimiter left, everything left is part of
                // the value

                *self = ParamsIter::NextUri("");

                rest
            };

            Ok((name, value))
        };

        Some(f())
    }
}

/// Return (uri, params iterator).
///
/// Precondition: `s` must not be empty.
pub(super) fn parse_uri(s: &str) -> Result<(&str, ParamsIter<'_>), Error> {
    let s = s
        .trim_start_http_whitespaces()
        .strip_prefix('<')
        .ok_or(Error("Expected '<' for uri"))?;

    let (uri, rest) = s.split_once('>').ok_or(Error("Expected '>' for uri"))?;

    Ok((uri, ParamsIter::new(rest)?))
}
