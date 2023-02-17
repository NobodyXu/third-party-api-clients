mod error;
pub use error::Error;

mod parser;
use parser::parse_uri;

mod utils;
use utils::IterExt;

use std::{iter::FromIterator, str::FromStr, vec::IntoIter as VecIntoIter};

pub use url::Url;

/// All uri that contains rel "next".
#[derive(Debug)]
pub struct NextLinks(VecIntoIter<Url>);

impl From<NextLinks> for Vec<Url> {
    /// libstd contains specialisation for `VecIntoIter`, thus this conversion
    /// is O(1).
    fn from(next_links: NextLinks) -> Self {
        Self::from_iter(next_links.0)
    }
}

impl Iterator for NextLinks {
    type Item = Url;

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

            // Params rel can only occur once and the parser is required to ignore
            // all but the first one.
            let rels = params
                .try_find_map(|(name, value)| "rel".eq_ignore_ascii_case(name).then_some(value))
                .transpose()?;

            // Consume the iterator so that we can parse the next link uri
            // and propagate errors.
            //
            // Since params impls FusedIterator, we can do this even if
            // rels.is_none()
            if let Some(err) = params.find_map(Result::err) {
                return Err(err);
            }

            let is_next = rels
                .map(|rels| rels.split(' ').any(|rel| "next".eq_ignore_ascii_case(rel)))
                .unwrap_or(false);

            if is_next {
                next_links.push(uri);
            }

            s = params.into_next_uri().unwrap();
        }

        Ok(Self(next_links.into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, FromStr, NextLinks, Url};

    struct CaseSuccess {
        input: &'static str,
        expected_output: &'static [&'static str],
    }

    struct CaseFailure {
        input: &'static str,
        expected_err: fn() -> Error,
    }

    const SIMPLE_CASES_SUCCESS: &[CaseSuccess] = &[
        CaseSuccess {
            input: r#"<https://one.example.com>; rel="preconnect", <https://two.example.com>; rel="preconnect", <https://three.example.com>; rel="preconnect""#,
            expected_output: &[],
        },
        CaseSuccess {
            input: r#"<https://one.example.com>; rel="preconnect", <https://two.example.com>; rel="preconnect", <https://three.example.com>; rel="preconnect",    <https://link.example.com>; rel="next preconnect"; rel=preconnect a;    a=b"#,
            expected_output: &["https://link.example.com"],
        },
        CaseSuccess {
            input: r#"<https://one.example.com>; rel="preconnect", <https://two.example.com>; rel="preconnect", <https://three.example.com>; rel="preconnect",    <https://link.example.com>; rel="next preconnect"; rel=preconnect a;    a=b, <https://link2.example.com>; rel=next  a wecx; rel="a    ed s"; a=v"#,
            expected_output: &["https://link.example.com", "https://link2.example.com"],
        },
    ];

    const SIMPLE_CASES_FAILURE: &[CaseFailure] = &[
        CaseFailure {
            input: r#"https://one.example.com>; rel="preconnect", <https://two.example.com>; rel="preconnect", <https://three.example.com>; rel="preconnect""#,
            expected_err: || Error::msg("Expected '<' for uri"),
        },
        CaseFailure {
            input: r#"<https://one.example.com>, rel="preconnect"; <https://two.example.com>; rel="preconnect", <https://three.example.com>; rel="preconnect""#,
            expected_err: || Error::msg("Expected '<' for uri"),
        },
        CaseFailure {
            input: r#"<https://one.example.com, rel="preconnect"; <https://two.example.com; rel="preconnect", <https://three.example.com; rel="preconnect""#,
            expected_err: || Error::msg("Expected '>' for uri"),
        },
        CaseFailure {
            input: r#"<;,>; rel="preconnect", <https://two.example.com>; rel="preconnect", <https://three.example.com>; rel="preconnect""#,
            expected_err: || Error::url_parse_err(Url::parse(";,").unwrap_err()),
        },
        CaseFailure {
            input: r#"<https://one.example.com>; a"#,
            expected_err: || Error::msg("Expected param"),
        },
        CaseFailure {
            input: r#"<https://one.example.com>; rel="preconnect, <https://two.example.com>; rel=preconnect, <https://three.example.com>"#,
            expected_err: || Error::msg("Unclosed '\"' in param value"),
        },
        CaseFailure {
            input: r#"<https://one.example.com> bbbb"#,
            expected_err: || {
                Error::msg("Expected either ';' for next param, ',' for next uri or an empty string for termination")
            },
        },
    ];

    #[test]
    fn test_simple_cases() {
        SIMPLE_CASES_SUCCESS.iter().for_each(
            |CaseSuccess {
                 input,
                 expected_output,
             }| {
                let actual_output: Vec<_> = NextLinks::from_str(input).unwrap().into();

                let expected_output: Vec<_> = expected_output
                    .iter()
                    .copied()
                    .map(Url::parse)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();

                assert_eq!(actual_output, expected_output);
            },
        );

        SIMPLE_CASES_FAILURE.iter().for_each(
            |CaseFailure {
                 input,
                 expected_err,
             }| {
                let err = NextLinks::from_str(input).unwrap_err();

                assert_eq!(err, expected_err());
            },
        );
    }
}
