mod error;
pub use error::Error;

mod parser;
use parser::parse_uri;

use std::{str::FromStr, vec::IntoIter as VecIntoIter};

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
