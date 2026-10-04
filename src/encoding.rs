use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Auto,
    Utf8,
    Cp932,
}

pub(crate) fn fail(s: &[u8], pos: usize, message: &str) -> Error {
    let mut line = 1;
    let limit = pos.min(s.len());
    let mut i = 0;
    while i < limit {
        if s[i] == b'\n' || (s[i] == b'\r' && (i + 1 >= s.len() || s[i + 1] != b'\n')) {
            line += 1;
        }
        i += 1;
    }
    Error::new(Some(line), message).with_offset(pos.min(s.len()))
}

fn check_utf8(s: &[u8]) -> (bool, usize, Option<usize>) {
    let mut first_non_ascii = None;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c < 128 {
            i += 1;
            continue;
        }
        if first_non_ascii.is_none() {
            first_non_ascii = Some(i);
        }
        let count = if (0xc2..=0xdf).contains(&c) {
            2
        } else if (0xe0..=0xef).contains(&c) {
            3
        } else if (0xf0..=0xf4).contains(&c) {
            4
        } else {
            0
        };
        if count == 0 || i + count > s.len() {
            return (false, i, first_non_ascii);
        }
        for j in 1..count {
            let b = s[i + j];
            if !(0x80..=0xbf).contains(&b) {
                return (false, i + j, first_non_ascii);
            }
        }
        let second = s[i + 1];
        if (c == 0xe0 && second < 0xa0)
            || (c == 0xed && second >= 0xa0)
            || (c == 0xf0 && second < 0x90)
            || (c == 0xf4 && second >= 0x90)
        {
            return (false, i, first_non_ascii);
        }
        i += count;
    }
    (true, 0, first_non_ascii)
}

fn check_cp932(s: &[u8]) -> (bool, usize, Option<usize>) {
    let mut first_non_ascii = None;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c < 128 {
            i += 1;
            continue;
        }
        if first_non_ascii.is_none() {
            first_non_ascii = Some(i);
        }
        if (0xa1..=0xdf).contains(&c) {
            i += 1;
            continue;
        }
        if ((0x81..=0x9f).contains(&c) || (0xe0..=0xfc).contains(&c)) && i + 1 < s.len() {
            let next = s[i + 1];
            if ((0x40..=0x7e).contains(&next) || (0x80..=0xfc).contains(&next)) && next != 0x7f {
                i += 2;
                continue;
            }
        }
        return (false, i, first_non_ascii);
    }
    (true, 0, first_non_ascii)
}

pub fn detect_encoding(s: &[u8]) -> Result<Encoding, Error> {
    // 1. UTF-8 BOM
    if s.starts_with(b"\xef\xbb\xbf") {
        let (valid, invalid_pos, _) = check_utf8(&s[3..]);
        if !valid {
            return Err(fail(
                s,
                3 + invalid_pos,
                "invalid UTF-8 byte sequence after BOM",
            ));
        }
        return Ok(Encoding::Utf8);
    }

    let (valid_utf8, utf8_invalid, utf8_first) = check_utf8(s);

    // 2. ASCII: no non-ASCII bytes
    if utf8_first.is_none() {
        return Ok(Encoding::Utf8);
    }

    let (valid_cp932, cp932_invalid, _) = check_cp932(s);

    // 3. Exactly one encoding is valid
    if valid_utf8 && !valid_cp932 {
        return Ok(Encoding::Utf8);
    }
    if !valid_utf8 && valid_cp932 {
        return Ok(Encoding::Cp932);
    }

    // 4. Undetermined (neither valid, or ambiguous between both)
    if !valid_utf8 && !valid_cp932 {
        return Err(fail(
            s,
            utf8_invalid.min(cp932_invalid),
            "cannot determine encoding: neither valid UTF-8 nor valid CP932; specify --encoding explicitly",
        ));
    }
    Err(fail(
        s,
        utf8_first.unwrap(),
        "cannot determine encoding: ambiguous between UTF-8 and CP932; specify --encoding explicitly",
    ))
}

// `encoding` must already be resolved; callers detect Auto once per source.
pub(crate) fn character_end(
    s: &[u8],
    i: usize,
    encoding: ResolvedEncoding,
) -> Result<usize, Error> {
    let c = s[i];
    if c < 128 {
        return Ok(i + 1);
    }
    if encoding == ResolvedEncoding::Cp932 {
        if (0xa1..=0xdf).contains(&c) {
            return Ok(i + 1);
        }
        if ((0x81..=0x9f).contains(&c) || (0xe0..=0xfc).contains(&c)) && i + 1 < s.len() {
            let next = s[i + 1];
            if ((0x40..=0x7e).contains(&next) || (0x80..=0xfc).contains(&next)) && next != 0x7f {
                return Ok(i + 2);
            }
        }
        return Err(fail(
            s,
            i,
            &format!("invalid CP932 byte sequence at byte {i}"),
        ));
    }
    let count = if (0xc2..=0xdf).contains(&c) {
        2
    } else if (0xe0..=0xef).contains(&c) {
        3
    } else if (0xf0..=0xf4).contains(&c) {
        4
    } else {
        0
    };
    if count == 0 || i + count > s.len() {
        return Err(fail(
            s,
            i,
            "invalid UTF-8; use --encoding=cp932 for CP932 input",
        ));
    }
    for j in 1..count {
        let b = s[i + j];
        if !(0x80..=0xbf).contains(&b) {
            return Err(fail(s, i, "invalid UTF-8 continuation"));
        }
    }
    let second = s[i + 1];
    if (c == 0xe0 && second < 0xa0)
        || (c == 0xed && second >= 0xa0)
        || (c == 0xf0 && second < 0x90)
        || (c == 0xf4 && second >= 0x90)
    {
        return Err(fail(s, i, "invalid UTF-8 code point"));
    }
    Ok(i + count)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedEncoding {
    Utf8,
    Cp932,
}

pub(crate) fn resolve_encoding(
    source: &[u8],
    encoding: Encoding,
) -> Result<ResolvedEncoding, Error> {
    let encoding = if encoding == Encoding::Auto {
        detect_encoding(source)?
    } else {
        encoding
    };
    Ok(if encoding == Encoding::Cp932 {
        ResolvedEncoding::Cp932
    } else {
        ResolvedEncoding::Utf8
    })
}
