use std::{
    str::{FromStr, Split},
    vec::IntoIter as VecIntoIter,
};

use itertools::Itertools;

#[derive(Debug)]
pub struct NextLinks(VecIntoIter<String>);

impl Iterator for NextLinks {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

fn error(msg: &str) -> anyhow::Error {
    anyhow::anyhow!("Invalid link segment: {}", msg)
}

trait StrExt {
    const HTTP_WHITESPACE: [char; 2] = [' ', '\t'];

    fn strip_quotation(&self, quotation: (char, char)) -> Option<&Self>;

    fn trim_http_whitespaces(&self) -> &Self;

    fn split_http_whitespaces(&self) -> Split<'_, [char; 2]>;
}

impl StrExt for str {
    fn strip_quotation(&self, quotation: (char, char)) -> Option<&Self> {
        self.strip_prefix(quotation.0)
            .and_then(|s| s.strip_suffix(quotation.1))
    }

    fn trim_http_whitespaces(&self) -> &Self {
        self.trim_matches(&Self::HTTP_WHITESPACE[..])
    }

    fn split_http_whitespaces(&self) -> Split<'_, [char; 2]> {
        self.split(Self::HTTP_WHITESPACE)
    }
}

impl FromStr for NextLinks {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        enum Segment<'a> {
            LinkValue(&'a str),
            ParamRels { is_next: bool },
        }

        // Parse the segments
        let segments = s
            .trim_http_whitespaces()
            .split(&[';', ','])
            .map(str::trim_http_whitespaces)
            .filter_map(|segment| {
                let bail = |msg| Some(Err(error(msg)));

                if let Some(segment) = segment.strip_quotation(('<', '>')) {
                    Some(Ok(Segment::LinkValue(segment.trim_http_whitespaces())))
                } else if segment.starts_with('<') || segment.ends_with('>') {
                    bail("Found incomplete Target IRI with unclosed '<' and '>'")
                } else if let Some((name, value)) = segment.split_once('=') {
                    // Parse relation type: `rel`.
                    // https://tools.ietf.org/html/rfc5988#section-5.3

                    if "rel".eq_ignore_ascii_case(name.trim_http_whitespaces()) {
                        let value = value.trim_http_whitespaces();

                        if value.is_empty() {
                            bail("Found paramter rels but its value is empty")
                        } else {
                            let rels = if let Some(rels) = value.strip_quotation(('"', '"')) {
                                rels.trim_http_whitespaces()
                            } else if value.starts_with('"') || value.ends_with('"') {
                                return bail("Unclosed \" in parameters rel");
                            } else {
                                value
                            };

                            Some(Ok(Segment::ParamRels {
                                is_next: rels
                                    .split_http_whitespaces()
                                    .any(|rel| "next".eq_ignore_ascii_case(rel)),
                            }))
                        }
                    } else {
                        None
                    }
                } else {
                    bail("Neither Target IRI nor parameters")
                }
            })
            .coalesce(|x, y| {
                let is_param_rels =
                    |val: &Result<_, _>| matches!(val, Ok(Segment::ParamRels { .. }));
                let is_link_value = |val: &Result<_, _>| matches!(val, Ok(Segment::LinkValue(_)));

                if is_param_rels(&x) && is_param_rels(&y) {
                    // Params rel can only occur once and the parser is required to ignore
                    // all but the first one.
                    Ok(x)
                } else if is_link_value(&x) && is_link_value(&y) {
                    // Filter out link_value that does not have a rel parameter,
                    // except for the last one.
                    Ok(y)
                } else {
                    Err((x, y))
                }
            });

        // Find link values with params rel=next
        let next_links: Vec<_> = segments
            .tuples()
            .filter_map(|(x, y)| {
                // Expected tuples like (Segment::LinkValue(_), Segment::ParamRels { .. })
                // Having (Segment::LinkValue(_), Segment::LinkValue(_)) would cause it to panic
                // since that should not happen.
                // Anything else would cause this clousre to return an `Some(Err(_))`.

                (|| -> Result<Option<String>, anyhow::Error> {
                    let Segment::LinkValue(link_value) = x? else {
                        return Err(error("Expected Target IRI but found parameter rel"))
                    };

                    let Segment::ParamRels { is_next } = y? else {
                        unreachable!("segments.tuples() should only contain link_value with param rel")
                    };

                    Ok(is_next.then(|| link_value.to_string()))
                })()
                .transpose()
            })
            .try_collect()?;

        Ok(Self(next_links.into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
