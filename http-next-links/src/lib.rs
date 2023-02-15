mod error;
pub use error::Error;

mod parser;
use parser::parse_uri;

use std::{iter::FromIterator, str::FromStr, vec::IntoIter as VecIntoIter};

/// All uri that contains rel "next".
#[derive(Debug)]
pub struct NextLinks(VecIntoIter<String>);

impl From<NextLinks> for Vec<String> {
    /// libstd contains specialisation for `VecIntoIter`, thus this conversion
    /// is O(1).
    fn from(next_links: NextLinks) -> Self {
        Self::from_iter(next_links.0)
    }
}

impl Iterator for NextLinks {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl FromStr for NextLinks {
    type Err = Error;

    /// Parses all uri that contains rel "next".
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
