use crate::encoding::{character_end, fail, resolve_encoding, ResolvedEncoding};
use crate::util::{alpha, digit, one_of};
use crate::{Encoding, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Space,
    Newline,
    Word,
    Number,
    String,
    Comment,
    Symbol,
    Bom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub begin: usize,
    pub end: usize,
}

pub fn lex(s: &[u8], encoding: Encoding) -> Result<Vec<Token>, Error> {
    lex_resolved(s, resolve_encoding(s, encoding)?)
}

pub(crate) fn lex_resolved(s: &[u8], enc: ResolvedEncoding) -> Result<Vec<Token>, Error> {
    scan(s, enc, true)
}

pub(crate) fn lex_fragment(s: &[u8], enc: ResolvedEncoding) -> Result<Vec<Token>, Error> {
    scan(s, enc, false)
}

fn scan(s: &[u8], enc: ResolvedEncoding, allow_bom: bool) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let begin = i;
        let mut kind = Kind::Symbol;
        if allow_bom && i == 0 && s.starts_with(b"\xef\xbb\xbf") {
            kind = Kind::Bom;
            i += 3;
        } else if s[i] == b' ' || s[i] == b'\t' {
            kind = Kind::Space;
            while i < s.len() && (s[i] == b' ' || s[i] == b'\t') {
                i += 1;
            }
        } else if s[i] == b'\r' || s[i] == b'\n' {
            kind = Kind::Newline;
            if s[i] == b'\r' && i + 1 < s.len() && s[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
        } else if s[i] == b';' || s[i..].starts_with(b"//") {
            kind = Kind::Comment;
            while i < s.len() && s[i] != b'\r' && s[i] != b'\n' {
                i = character_end(s, i, enc)?;
            }
        } else if s[i..].starts_with(b"/*") {
            kind = Kind::Comment;
            i += 2;
            while i < s.len() && !s[i..].starts_with(b"*/") {
                i = character_end(s, i, enc)?;
            }
            if i >= s.len() {
                return Err(fail(s, begin, "unterminated block comment"));
            }
            i += 2;
        } else if s[i] == b'"' || s[i] == b'\'' || s[i..].starts_with(b"{\"") {
            kind = Kind::String;
            let multiline = s[i] == b'{';
            let quote = if multiline { b'"' } else { s[i] };
            i += if multiline { 2 } else { 1 };
            let mut closed = false;
            while i < s.len() {
                if s[i] == quote && (!multiline || s[i..].starts_with(b"\"}")) {
                    i += if multiline { 2 } else { 1 };
                    closed = true;
                    break;
                }
                if s[i] == b'\\' {
                    i += 1;
                    if i < s.len() {
                        i = character_end(s, i, enc)?;
                    }
                } else {
                    if !multiline && (s[i] == b'\r' || s[i] == b'\n') {
                        return Err(fail(s, begin, "newline in quoted string"));
                    }
                    i = character_end(s, i, enc)?;
                }
            }
            if !closed {
                return Err(fail(s, begin, "unterminated string"));
            }
        } else if digit(s[i])
            || s[i] == b'$'
            || (s[i] == b'%' && i + 1 < s.len() && (digit(s[i + 1]) || s[i + 1] == b'_'))
        {
            kind = Kind::Number;
            let hex = s[i] == b'$' || s[i..].starts_with(b"0x");
            let bin = s[i] == b'%' || s[i..].starts_with(b"0b");
            i += if s[i..].starts_with(b"0x") || s[i..].starts_with(b"0b") {
                2
            } else {
                1
            };
            while i < s.len() {
                let c = s[i];
                if digit(c)
                    || c == b'_'
                    || (hex && c.is_ascii_hexdigit())
                    || (!hex && !bin && c == b'.' && i + 1 < s.len() && digit(s[i + 1]))
                {
                    i += 1;
                } else if !hex && !bin && (c == b'e' || c == b'E') {
                    i += 1;
                    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if i < s.len()
                && (s[i] == b'l'
                    || s[i] == b'L'
                    || s[i] == b'f'
                    || s[i] == b'F'
                    || (!hex && !bin && (s[i] == b'd' || s[i] == b'D')))
            {
                i += 1;
            }
        } else if alpha(s[i]) || s[i] >= 128 {
            kind = Kind::Word;
            loop {
                i = character_end(s, i, enc)?;
                if i >= s.len() || (!alpha(s[i]) && !digit(s[i]) && s[i] < 128) {
                    break;
                }
            }
        } else {
            if s[i] == 0 {
                return Err(fail(s, i, "NUL byte in source"));
            }
            i += 1;
            if i < s.len()
                && one_of(
                    &s[begin..begin + 2],
                    &[
                        b"==", b"!=", b"<=", b">=", b"<<", b">>", b"&&", b"||", b"++", b"--",
                        b"+=", b"-=", b"*=", b"/=", b"\\=", b"&=", b"|=", b"^=",
                    ],
                )
            {
                i += 1;
            }
        }
        tokens.push(Token {
            kind,
            begin,
            end: i,
        });
    }
    Ok(tokens)
}
