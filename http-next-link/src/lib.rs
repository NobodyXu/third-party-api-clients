use std::str::FromStr;

use itertools::Itertools;

#[derive(Debug)]
pub struct NextLink(Option<String>);

impl Iterator for NextLink {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.take()
    }
}

fn error(msg: &str) -> anyhow::Error {
    anyhow::anyhow!("Invalid link segment: {}", msg)
}

fn strip_quotation(s: &str, quotation: (char, char)) -> Option<&str> {
    s.strip_prefix(quotation.0)
        .and_then(|s| s.strip_suffix(quotation.1))
}

impl FromStr for NextLink {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        enum Segment<'a> {
            LinkValue(&'a str),
            ParamRels { is_next: bool },
        }

        // Parse the segments
        let mut segments = s
            .trim()
            .split(&[';', ','])
            .map(str::trim)
            .filter_map(|segment| {
                let bail = |msg| Some(Err(error(msg)));

                if let Some(segment) = strip_quotation(segment, ('<', '>')) {
                    Some(Ok(Segment::LinkValue(segment.trim())))
                } else if segment.starts_with('<') || segment.ends_with('>') {
                    bail("Found incomplete Target IRI with unclosed '<' and '>'")
                } else if let Some((name, value)) = segment.split_once('=') {
                    // Parse relation type: `rel`.
                    // https://tools.ietf.org/html/rfc5988#section-5.3

                    if "rel".eq_ignore_ascii_case(name.trim()) {
                        let value = value.trim();

                        if value.is_empty() {
                            bail("Found paramter relations but its value is empty")
                        } else {
                            let rels = if let Some(rels) = strip_quotation(value, ('"', '"')) {
                                rels.trim()
                            } else if value.starts_with('"') || value.ends_with('"') {
                                return bail("Unclosed \" in parameters rel");
                            } else {
                                value
                            };

                            Some(Ok(Segment::ParamRels {
                                is_next: rels
                                    .split(' ')
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
                // Relation can only occur once and the parser is required to ignore
                // all but the first one.
                if matches!(x, Ok(Segment::ParamRels { .. }))
                    && matches!(y, Ok(Segment::ParamRels { .. }))
                {
                    Ok(x)
                } else {
                    Err((x, y))
                }
            })
            .peekable();

        // Find link_value with params rel=next
        while let Some(res) = segments.next() {
            let segment = res?;

            let Segment::LinkValue(link_value) = segment else {
                return Err(error("Expected Target IRI but found parameters"));
            };

            if let Some(res) = segments.next_if(|res| matches!(res, Ok(Segment::ParamRels { .. })))
            {
                let Segment::ParamRels{ is_next } = res.unwrap() else {
                    unreachable!("BUG: res can only be Ok(Segment::ParamRels(_))")
                };

                if is_next {
                    // Propagate errors
                    if let Some(err) = segments.find_map(Result::err) {
                        return Err(err);
                    } else {
                        return Ok(Self(Some(link_value.to_string())));
                    }
                }
            }
        }

        Ok(Self(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
