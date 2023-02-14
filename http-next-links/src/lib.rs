use std::{error, fmt, iter, str::FromStr, vec::IntoIter as VecIntoIter};

#[derive(Debug)]
pub struct Error(&'static str);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid link segment: {}", self.0)
    }
}

impl error::Error for Error {}

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

enum ParamsIter<'a> {
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
    fn into_next_uri(self) -> Option<&'a str> {
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
            let (name, rest) = params
                .split_once('=')
                .ok_or_else(|| Error("Expected param"))?;

            let name = name.trim_end_http_whitespaces();

            let rest = rest.trim_start_http_whitespaces();
            let value = if let Some(rest) = rest.strip_prefix('"') {
                // Parse quoted value
                let (value, rest) = rest
                    .split_once('"')
                    .ok_or_else(|| Error("Unclosed '\"' in param value"))?;

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
fn parse_uri(s: &str) -> Result<(&str, ParamsIter<'_>), Error> {
    let s = s
        .trim_start_http_whitespaces()
        .strip_prefix('<')
        .ok_or_else(|| Error("Expected '<' for uri"))?;

    let (uri, rest) = s
        .split_once('>')
        .ok_or_else(|| Error("Expected '>' for uri"))?;

    Ok((uri, ParamsIter::new(rest)?))
}

#[derive(Debug)]
pub struct NextLinks(VecIntoIter<String>);

impl Iterator for NextLinks {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl FromStr for NextLinks {
    type Err = Error;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let mut next_links = Vec::new();

        while !s.is_empty() {
            let (uri, mut params) = parse_uri(s)?;

            let mut rels = None;

            while let Some((name, value)) = params.next().transpose()? {
                // Params rel can only occur once and the parser is required to ignore
                // all but the first one.
                if "rel".eq_ignore_ascii_case(name) && rels.is_none() {
                    rels = Some(value);
                }
            }

            let is_next = rels
                .map(|rels| rels.split(' ').any(|rel| "next".eq_ignore_ascii_case(rel)))
                .unwrap_or(false);

            if is_next {
                next_links.push(uri.to_string());
            }

            s = params.into_next_uri().unwrap();
        }

        Ok(Self(next_links.into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
