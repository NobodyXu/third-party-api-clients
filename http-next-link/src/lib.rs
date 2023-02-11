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

impl FromStr for NextLink {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        enum Segment<'a> {
            LinkValue(&'a str),
            ParamRels(&'a str),
        }

        let mut segments = s
            .trim()
            .split(&[';', ','])
            .map(str::trim)
            .filter_map(|segment| {
                let bail = |msg| Some(Err(anyhow::anyhow!("Invalid link segment: {}", msg)));

                if let Some(segment) = segment.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
                    Some(Ok(Segment::LinkValue(segment.trim())))
                } else if segment.starts_with('<') || segment.ends_with('>') {
                    bail("Found incomplete Target IRI")
                } else if let Some((name, value)) = segment.split_once('=') {
                    // Parse relation type: `rel`.
                    // https://tools.ietf.org/html/rfc5988#section-5.3

                    if "rel".eq_ignore_ascii_case(name.trim()) {
                        let value = value.trim();

                        if value.is_empty() {
                            bail("Found paramter relations but its value is empty")
                        } else {
                            Some(Ok(Segment::ParamRels(value)))
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
                if matches!(x, Ok(Segment::ParamRels(_))) && matches!(y, Ok(Segment::ParamRels(_)))
                {
                    Ok(x)
                } else {
                    Err((x, y))
                }
            })
            .peekable();

        let bail = |msg| anyhow::bail!("Invalid link segment: {}", msg);

        // Loop over the splits parsing the Link header into
        // a `Vec<LinkValue>`
        while let Some(res) = segments.next() {
            let segment = res?;

            let Segment::LinkValue(link_value) = segment else {
                return bail("Expected Target IRI but found parameters");
            };

            if let Some(res) = segments.next_if(|res| matches!(res, Ok(Segment::ParamRels(_)))) {
                let Segment::ParamRels(rels) = res.unwrap() else {
                    unreachable!("BUG: res can only be Ok(Segment::ParamRels(_))")
                };

                let rels = if let Some(stripped_rels) = rels
                    .strip_prefix('"')
                    .and_then(|rels| rels.strip_suffix('"'))
                {
                    stripped_rels.trim()
                } else if rels.starts_with('"') || rels.ends_with('"') {
                    return bail("Unclose \" in relation parameters");
                } else {
                    rels
                };

                let is_next = rels.split(' ').any(|rel| "next".eq_ignore_ascii_case(rel));

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
