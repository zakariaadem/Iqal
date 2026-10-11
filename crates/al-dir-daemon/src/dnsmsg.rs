//! Just enough of the DNS wire format (RFC 1035) to read a query's name and
//! type, and to build the few answers the filter gives itself. Everything
//! else is relayed to the upstream resolver byte for byte.

pub const TYPE_A: u16 = 1;
pub const TYPE_AAAA: u16 = 28;
pub const TYPE_ANY: u16 = 255;
const CLASS_IN: u16 = 1;
const HEADER: usize = 12;
/// Short TTL for our own answers, so lifting a block takes effect quickly.
const BLOCK_TTL: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub id: u16,
    /// Lower-case, without the trailing dot.
    pub name: String,
    pub qtype: u16,
    /// Offset just past the question section.
    question_end: usize,
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]))
}

/// Parses a standard query with exactly one question.
pub fn parse_query(pkt: &[u8]) -> Option<Query> {
    let flags = u16_at(pkt, 2)?;
    let is_response = flags & 0x8000 != 0;
    let opcode = (flags >> 11) & 0xf;
    if is_response || opcode != 0 || u16_at(pkt, 4)? != 1 {
        return None;
    }
    let mut i = HEADER;
    let mut name = String::new();
    loop {
        let len = *pkt.get(i)? as usize;
        i += 1;
        if len == 0 {
            break;
        }
        if len > 63 {
            return None; // compression is not used in questions
        }
        let label = pkt.get(i..i + len)?;
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(&String::from_utf8_lossy(label).to_ascii_lowercase());
        i += len;
        if name.len() > 253 {
            return None;
        }
    }
    let qtype = u16_at(pkt, i)?;
    u16_at(pkt, i + 2)?; // class
    Some(Query { id: u16_at(pkt, 0)?, name, qtype, question_end: i + 4 })
}

/// A response to `query` (taken from `pkt`) with the given answers.
fn respond(pkt: &[u8], q: &Query, rcode: u16, answers: &[(u16, u32, Vec<u8>)]) -> Vec<u8> {
    let rd = u16_at(pkt, 2).unwrap_or(0) & 0x0100;
    let flags: u16 = 0x8000 | rd | 0x0080 | (rcode & 0xf); // QR, RD copied, RA
    let mut out = Vec::with_capacity(q.question_end + answers.len() * 28);
    out.extend_from_slice(&q.id.to_be_bytes());
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    out.extend_from_slice(&(answers.len() as u16).to_be_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]); // NSCOUNT, ARCOUNT
    out.extend_from_slice(&pkt[HEADER..q.question_end]);
    for (rtype, ttl, data) in answers {
        out.extend_from_slice(&[0xc0, 0x0c]); // pointer to the question name
        out.extend_from_slice(&rtype.to_be_bytes());
        out.extend_from_slice(&CLASS_IN.to_be_bytes());
        out.extend_from_slice(&ttl.to_be_bytes());
        out.extend_from_slice(&(data.len() as u16).to_be_bytes());
        out.extend_from_slice(data);
    }
    out
}

/// "This name points nowhere": 0.0.0.0 / :: so connections fail at once.
pub fn blocked(pkt: &[u8], q: &Query) -> Vec<u8> {
    let mut answers = Vec::new();
    if matches!(q.qtype, TYPE_A | TYPE_ANY) {
        answers.push((TYPE_A, BLOCK_TTL, vec![0; 4]));
    }
    if matches!(q.qtype, TYPE_AAAA | TYPE_ANY) {
        answers.push((TYPE_AAAA, BLOCK_TTL, vec![0; 16]));
    }
    respond(pkt, q, 0, &answers)
}

/// "No such name".
pub fn nxdomain(pkt: &[u8], q: &Query) -> Vec<u8> {
    respond(pkt, q, 3, &[])
}

/// A standalone query for `name` (used to look up SafeSearch hosts).
pub fn build_query(id: u16, name: &str, qtype: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + name.len() + 6);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&0x0100u16.to_be_bytes()); // RD
    out.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0]);
    for label in name.split('.').filter(|l| !l.is_empty()) {
        out.push(label.len().min(63) as u8);
        out.extend_from_slice(&label.as_bytes()[..label.len().min(63)]);
    }
    out.push(0);
    out.extend_from_slice(&qtype.to_be_bytes());
    out.extend_from_slice(&CLASS_IN.to_be_bytes());
    out
}

/// Skips a (possibly compressed) name, returning the offset after it.
fn skip_name(b: &[u8], mut i: usize) -> Option<usize> {
    loop {
        let len = *b.get(i)?;
        match len {
            0 => return Some(i + 1),
            l if l & 0xc0 == 0xc0 => return Some(i + 2),
            l if l < 64 => i += 1 + l as usize,
            _ => return None,
        }
    }
}

/// The A/AAAA records (type, ttl, data) in a response.
pub fn address_records(resp: &[u8]) -> Option<Vec<(u16, u32, Vec<u8>)>> {
    let qd = u16_at(resp, 4)?;
    let an = u16_at(resp, 6)?;
    let mut i = HEADER;
    for _ in 0..qd {
        i = skip_name(resp, i)? + 4;
    }
    let mut out = Vec::new();
    for _ in 0..an {
        i = skip_name(resp, i)?;
        let rtype = u16_at(resp, i)?;
        let ttl = u32::from_be_bytes(resp.get(i + 4..i + 8)?.try_into().ok()?);
        let len = u16_at(resp, i + 8)? as usize;
        let data = resp.get(i + 10..i + 10 + len)?;
        if rtype == TYPE_A || rtype == TYPE_AAAA {
            out.push((rtype, ttl, data.to_vec()));
        }
        i += 10 + len;
    }
    Some(out)
}

/// Answers `q` with the addresses of another name (SafeSearch: the
/// addresses of forcesafesearch.google.com under the name www.google.com).
pub fn with_addresses(pkt: &[u8], q: &Query, records: &[(u16, u32, Vec<u8>)]) -> Vec<u8> {
    let wanted: Vec<_> = records
        .iter()
        .filter(|(t, _, _)| q.qtype == TYPE_ANY || *t == q.qtype)
        .cloned()
        .collect();
    respond(pkt, q, 0, &wanted)
}

/// An empty, successful answer (no records of that type).
pub fn no_data(pkt: &[u8], q: &Query) -> Vec<u8> {
    respond(pkt, q, 0, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_query() {
        let pkt = build_query(0x1234, "WWW.Example.COM", TYPE_AAAA);
        let q = parse_query(&pkt).unwrap();
        assert_eq!(q, Query { id: 0x1234, name: "www.example.com".into(), qtype: TYPE_AAAA, question_end: pkt.len() });
    }

    #[test]
    fn rejects_non_queries() {
        let mut pkt = build_query(1, "a.com", TYPE_A);
        pkt[2] |= 0x80; // response bit
        assert!(parse_query(&pkt).is_none());
        assert!(parse_query(&[0; 5]).is_none());
        let mut truncated = build_query(1, "example.com", TYPE_A);
        truncated.truncate(16);
        assert!(parse_query(&truncated).is_none());
    }

    #[test]
    fn blocked_answers() {
        let pkt = build_query(7, "facebook.com", TYPE_A);
        let q = parse_query(&pkt).unwrap();
        let resp = blocked(&pkt, &q);
        assert_eq!(&resp[..2], &7u16.to_be_bytes());
        assert_eq!(resp[3] & 0x0f, 0, "NOERROR");
        assert_eq!(address_records(&resp).unwrap(), vec![(TYPE_A, BLOCK_TTL, vec![0, 0, 0, 0])]);

        let pkt = build_query(8, "facebook.com", 65); // HTTPS record
        let q = parse_query(&pkt).unwrap();
        assert!(address_records(&blocked(&pkt, &q)).unwrap().is_empty());

        let resp = nxdomain(&pkt, &q);
        assert_eq!(resp[3] & 0x0f, 3);
    }

    #[test]
    fn rewrites_addresses() {
        // Upstream answer for forcesafesearch.google.com with one A record.
        let up_q = build_query(9, "forcesafesearch.google.com", TYPE_A);
        let up_parsed = parse_query(&up_q).unwrap();
        let up = respond(&up_q, &up_parsed, 0, &[(TYPE_A, 300, vec![216, 239, 38, 120])]);
        let records = address_records(&up).unwrap();

        let pkt = build_query(10, "www.google.com", TYPE_A);
        let q = parse_query(&pkt).unwrap();
        let resp = with_addresses(&pkt, &q, &records);
        assert_eq!(parse_query(&{
            let mut r = resp.clone();
            r[2] &= 0x7f; // make it look like a query to reuse the parser
            r[6] = 0;
            r[7] = 0;
            r
        })
        .unwrap()
        .name, "www.google.com");
        assert_eq!(address_records(&resp).unwrap(), vec![(TYPE_A, 300, vec![216, 239, 38, 120])]);
    }
}
