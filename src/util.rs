pub(crate) fn digit(c: u8) -> bool {
    c.is_ascii_digit()
}

pub(crate) fn alpha(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

pub(crate) fn lower(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

pub(crate) fn one_of(s: &[u8], values: &[&[u8]]) -> bool {
    values.contains(&s)
}
