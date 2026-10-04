//! hspfmt library - Pre-macro HSP formatter in Rust.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Auto,
    Utf8,
    Cp932,
}

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
pub enum CommentStyle {
    Preserve,
    Semicolon,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockComments {
    Preserve,
    Lines,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parentheses {
    Preserve,
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorStyle {
    Preserve,
    Hsp,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullWidthSpaces {
    Preserve,
    Normalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spacing {
    Preserve,
    Space,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub begin: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: usize,
    pub source: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub encoding: Encoding,
    pub full_width_spaces: FullWidthSpaces,
    pub indent_width: usize,
    pub base_indent: usize,
    pub loop_indent: usize,
    pub preserve_indent: bool,
    pub tabs: bool,
    pub operator_spacing: Spacing,
    pub comma_spacing: Spacing,
    pub colon_spacing: Spacing,
    pub comment_spacing: Spacing,
    pub hsp_numeric_prefixes: bool,
    pub short_if: bool,
    pub line_width: usize,
    pub indent_labels: bool,
    pub comment_style: CommentStyle,
    pub block_comments: BlockComments,
    pub condition_parens: Parentheses,
    pub repeat_parens: Parentheses,
    pub blank_lines_before_module: i32,
    pub blank_lines_before_deffunc: i32,
    pub blank_lines_before_defcfunc: i32,
    pub operator_style: OperatorStyle,
    pub increment_style: OperatorStyle,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            encoding: Encoding::Auto,
            full_width_spaces: FullWidthSpaces::Preserve,
            indent_width: 4,
            base_indent: 0,
            loop_indent: 1,
            preserve_indent: false,
            tabs: false,
            operator_spacing: Spacing::Space,
            comma_spacing: Spacing::Space,
            colon_spacing: Spacing::Space,
            comment_spacing: Spacing::Space,
            hsp_numeric_prefixes: false,
            short_if: false,
            line_width: 100,
            indent_labels: false,
            comment_style: CommentStyle::Preserve,
            block_comments: BlockComments::Preserve,
            condition_parens: Parentheses::Preserve,
            repeat_parens: Parentheses::Preserve,
            blank_lines_before_module: -1,
            blank_lines_before_deffunc: -1,
            blank_lines_before_defcfunc: -1,
            operator_style: OperatorStyle::Preserve,
            increment_style: OperatorStyle::Preserve,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
    line: usize,
}

impl Error {
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line,
        }
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn line(&self) -> usize {
        self.line
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for Error {}

// --- Internal Helpers ---

fn digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn alpha(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn lower(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

fn one_of(s: &[u8], values: &[&[u8]]) -> bool {
    values.iter().any(|&v| v == s)
}

fn fail(s: &[u8], pos: usize, message: &str) -> Error {
    let mut line = 1;
    let limit = pos.min(s.len());
    let mut i = 0;
    while i < limit {
        if s[i] == b'\n' {
            line += 1;
        } else if s[i] == b'\r' {
            if i + 1 >= s.len() || s[i + 1] != b'\n' {
                line += 1;
            }
        }
        i += 1;
    }
    Error::new(line, message)
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

fn character_end(s: &[u8], i: usize, encoding: Encoding) -> Result<usize, Error> {
    let enc = if encoding == Encoding::Auto {
        detect_encoding(s)?
    } else {
        encoding
    };
    let c = s[i];
    if c < 128 {
        return Ok(i + 1);
    }
    if enc == Encoding::Cp932 {
        if (0xa1..=0xdf).contains(&c) {
            return Ok(i + 1);
        }
        if (0x81..=0x9f).contains(&c) || (0xe0..=0xfc).contains(&c) {
            if i + 1 < s.len() {
                let next = s[i + 1];
                if ((0x40..=0x7e).contains(&next) || (0x80..=0xfc).contains(&next)) && next != 0x7f
                {
                    return Ok(i + 2);
                }
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
enum Marker {
    None,
    Off,
    On,
    Ignore,
}

fn marker_kind(mut comment: &[u8]) -> Marker {
    while !comment.is_empty()
        && (comment[comment.len() - 1] == b' ' || comment[comment.len() - 1] == b'\t')
    {
        comment = &comment[..comment.len() - 1];
    }
    if comment.starts_with(b"; ") {
        comment = &comment[2..];
    } else if comment.starts_with(b"// ") {
        comment = &comment[3..];
    } else {
        return Marker::None;
    }
    if comment == b"hspfmt: off" {
        Marker::Off
    } else if comment == b"hspfmt: on" {
        Marker::On
    } else if comment == b"hspfmt: ignore" {
        Marker::Ignore
    } else {
        Marker::None
    }
}

#[derive(Debug, Clone)]
struct Item {
    kind: Kind,
    text: Vec<u8>,
    gap: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    close: Vec<u8>,
    case_body: bool,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct State {
    blocks: Vec<Frame>,
    function: usize,
    label: usize,
    chsp_module: usize,
    chsp_function: usize,
}

impl State {
    fn same_structure(&self, other: &State) -> bool {
        if self.blocks.len() != other.blocks.len() {
            return false;
        }
        for (a, b) in self.blocks.iter().zip(other.blocks.iter()) {
            if a.close != b.close || a.case_body != b.case_body {
                return false;
            }
        }
        (self.chsp_module != 0) == (other.chsp_module != 0)
            && (self.chsp_function != 0) == (other.chsp_function != 0)
    }

    fn depth(&self, options: &Options) -> usize {
        let mut n = options.base_indent.max(self.function).max(self.label);
        for frame in &self.blocks {
            n += if one_of(&frame.close, &[b"loop", b"wend", b"next", b"until"]) {
                options.loop_indent
            } else if frame.case_body {
                2
            } else {
                1
            };
        }
        n
    }
}

#[derive(Debug, Clone)]
struct Conditional {
    before: State,
    branch: State,
    has_branch: bool,
    has_else: bool,
    line: usize,
}

#[derive(Debug, Clone)]
struct SourceLine<'a> {
    begin: usize,
    end: usize,
    first: usize,
    last: usize,
    newline: &'a [u8],
    opaque: bool,
    directive: bool,
    multiline: bool,
}

fn source_lines<'a>(source: &'a [u8], tokens: &[Token]) -> Vec<SourceLine<'a>> {
    let mut lines = Vec::new();
    let mut begin = 0;
    let mut first = 0;
    let mut disabled = false;
    let mut continuation = false;
    let mut ignored = false;

    let mut append = |begin: usize, end: usize, first: usize, last: usize, newline: &'a [u8]| {
        let mut start = first;
        while start < last && (tokens[start].kind == Kind::Space || tokens[start].kind == Kind::Bom)
        {
            start += 1;
        }
        let mut actual_last = last;
        while actual_last > start && tokens[actual_last - 1].kind == Kind::Space {
            actual_last -= 1;
        }
        let text = |i: usize| &source[tokens[i].begin..tokens[i].end];
        let kind = if start + 1 == actual_last && tokens[start].kind == Kind::Comment {
            marker_kind(text(start))
        } else {
            Marker::None
        };
        let marker = kind != Marker::None;
        let next = start < actual_last && text(actual_last - 1) == b"\\";
        let mut multiline = false;
        for i in start..actual_last {
            let t = text(i);
            if t.contains(&b'\r') || t.contains(&b'\n') {
                multiline = true;
                break;
            }
        }
        lines.push(SourceLine {
            begin,
            end,
            first: start,
            last: actual_last,
            newline,
            opaque: disabled || ignored || continuation || next || marker,
            directive: start < actual_last && text(start) == b"#",
            multiline,
        });
        if marker && kind != Marker::Ignore {
            disabled = kind == Marker::Off;
        }
        ignored = kind == Marker::Ignore || (ignored && next);
        continuation = next;
    };

    for i in 0..tokens.len() {
        if tokens[i].kind != Kind::Newline {
            continue;
        }
        let nl = &source[tokens[i].begin..tokens[i].end];
        append(begin, tokens[i].begin, first, i, nl);
        begin = tokens[i].end;
        first = i + 1;
    }
    if begin < source.len() {
        append(begin, source.len(), first, tokens.len(), b"");
    }
    lines
}

pub fn normalize_full_width_spaces(source: &[u8], encoding: Encoding) -> Result<Vec<u8>, Error> {
    let tokens = lex(source, encoding)?;
    let lines = source_lines(source, &tokens);
    let full_space: &[u8] = if encoding == Encoding::Cp932 {
        b"\x81\x40"
    } else {
        b"\xe3\x80\x80"
    };
    let mut out = Vec::new();
    let mut copied = 0;
    let mut disabled = false;
    let mut ignored = false;

    for line in &lines {
        let mut first = line.first;
        while first < line.last {
            if tokens[first].kind == Kind::Space {
                first += 1;
                continue;
            }
            if tokens[first].kind != Kind::Word {
                break;
            }
            let token = &tokens[first];
            let mut text = &source[token.begin..token.end];
            while text.starts_with(full_space) {
                text = &text[full_space.len()..];
            }
            if !text.is_empty() {
                break;
            }
            first += 1;
        }
        let mut kind = Marker::None;
        let mut next_disabled = disabled;
        if first + 1 == line.last && tokens[first].kind == Kind::Comment {
            let token = &tokens[first];
            kind = marker_kind(&source[token.begin..token.end]);
            if kind == Marker::Off || kind == Marker::On {
                next_disabled = kind == Marker::Off;
            }
        }
        let marker = kind != Marker::None;
        let preserve = (disabled || ignored) && !marker;
        let ignore_marker = kind == Marker::Ignore;
        let next = line.first < line.last
            && &source[tokens[line.last - 1].begin..tokens[line.last - 1].end] == b"\\";
        ignored = ignore_marker || (ignored && next);
        disabled = next_disabled;
        if preserve {
            continue;
        }
        for t in line.first..line.last {
            let token = &tokens[t];
            if token.kind != Kind::Word {
                continue;
            }
            out.extend_from_slice(&source[copied..token.begin]);
            let mut pos = token.begin;
            while pos < token.end {
                let end = character_end(source, pos, encoding)?;
                let character = &source[pos..end];
                if character == full_space {
                    out.push(b' ');
                } else {
                    out.extend_from_slice(character);
                }
                pos = end;
            }
            copied = token.end;
        }
    }
    out.extend_from_slice(&source[copied..]);
    Ok(out)
}

pub fn rewrite_comments(source: &[u8], options: &Options) -> Result<Vec<u8>, Error> {
    if options.comment_style == CommentStyle::Preserve
        && options.block_comments == BlockComments::Preserve
    {
        return Ok(source.to_vec());
    }
    let tokens = lex(source, options.encoding)?;
    let lines = source_lines(source, &tokens);
    let text = |i: usize| &source[tokens[i].begin..tokens[i].end];
    let standalone = |line: &SourceLine| {
        !line.opaque
            && !line.directive
            && line.first + 1 == line.last
            && tokens[line.first].kind == Kind::Comment
            && !text(line.first).windows(7).any(|w| w == b"hspfmt:")
    };
    let line_comment = |line: &SourceLine| standalone(line) && !text(line.first).starts_with(b"/*");
    let body = |line: &SourceLine| {
        let comment = text(line.first);
        if comment.starts_with(b";") {
            &comment[1..]
        } else {
            &comment[2..]
        }
    };
    let prefix: &[u8] = if options.comment_style == CommentStyle::C {
        b"//"
    } else {
        b";"
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if options.block_comments == BlockComments::Block
            && line_comment(line)
            && !body(line).windows(2).any(|w| w == b"*/")
            && !body(line).windows(2).any(|w| w == b"/*")
        {
            out.extend_from_slice(&source[line.begin..tokens[line.first].begin]);
            out.extend_from_slice(b"/*");
            out.extend_from_slice(body(line));
            while i + 1 < lines.len()
                && line_comment(&lines[i + 1])
                && !body(&lines[i + 1]).windows(2).any(|w| w == b"*/")
                && !body(&lines[i + 1]).windows(2).any(|w| w == b"/*")
            {
                out.extend_from_slice(lines[i].newline);
                i += 1;
                out.extend_from_slice(body(&lines[i]));
            }
            out.extend_from_slice(b"*/");
            out.extend_from_slice(&source[tokens[lines[i].first].end..lines[i].end]);
            out.extend_from_slice(lines[i].newline);
        } else if options.block_comments == BlockComments::Lines
            && standalone(line)
            && text(line.first).starts_with(b"/*")
        {
            let comment = text(line.first);
            let content = &comment[2..comment.len() - 2];
            let mut indent = &source[line.begin..tokens[line.first].begin];
            if indent.starts_with(b"\xef\xbb\xbf") {
                out.extend_from_slice(b"\xef\xbb\xbf");
                indent = &indent[3..];
            }
            out.extend_from_slice(indent);
            out.extend_from_slice(prefix);
            let mut pos = 0;
            while pos < content.len() {
                let end = content[pos..]
                    .iter()
                    .position(|&b| b == b'\r' || b == b'\n')
                    .map(|idx| pos + idx);
                let end = match end {
                    Some(e) => e,
                    None => {
                        out.extend_from_slice(&content[pos..]);
                        break;
                    }
                };
                out.extend_from_slice(&content[pos..end]);
                let mut next = end + 1;
                if content[end] == b'\r' && next < content.len() && content[next] == b'\n' {
                    next += 1;
                }
                out.extend_from_slice(&content[end..next]);
                out.extend_from_slice(indent);
                out.extend_from_slice(prefix);
                pos = next;
            }
            out.extend_from_slice(&source[tokens[line.first].end..line.end]);
            out.extend_from_slice(line.newline);
        } else {
            let mut pos = line.begin;
            if !line.opaque
                && !line.directive
                && !line.multiline
                && options.comment_style != CommentStyle::Preserve
            {
                for t in line.first..line.last {
                    if tokens[t].kind != Kind::Comment
                        || text(t).starts_with(b"/*")
                        || text(t).windows(7).any(|w| w == b"hspfmt:")
                    {
                        continue;
                    }
                    out.extend_from_slice(&source[pos..tokens[t].begin]);
                    out.extend_from_slice(prefix);
                    let comment_text = text(t);
                    out.extend_from_slice(if comment_text.starts_with(b";") {
                        &comment_text[1..]
                    } else {
                        &comment_text[2..]
                    });
                    pos = tokens[t].end;
                }
            }
            out.extend_from_slice(&source[pos..line.end]);
            out.extend_from_slice(line.newline);
        }
        i += 1;
    }
    Ok(out)
}

pub fn declaration_layout(source: &[u8], options: &Options) -> Result<Vec<u8>, Error> {
    if options.preserve_indent
        && options.blank_lines_before_module < 0
        && options.blank_lines_before_deffunc < 0
        && options.blank_lines_before_defcfunc < 0
    {
        return Ok(source.to_vec());
    }
    let tokens = lex(source, options.encoding)?;
    let lines = source_lines(source, &tokens);
    let mut out = Vec::new();
    let mut copied = 0;
    for i in 0..lines.len() {
        let line = &lines[i];
        if line.opaque || line.multiline || line.first + 1 >= line.last {
            continue;
        }
        let token = &tokens[line.first + 1];
        let first = &tokens[line.first];
        let label = first.kind == Kind::Symbol
            && &source[first.begin..first.end] == b"*"
            && token.kind == Kind::Word;
        if !line.directive && !label {
            continue;
        }
        let directive = if line.directive {
            lower(&source[token.begin..token.end])
        } else {
            Vec::new()
        };
        let mut count = -1;
        if directive == b"module" || directive == b"chsp_module" {
            count = options.blank_lines_before_module;
        } else if directive == b"deffunc" || directive == b"chsp_deffunc" {
            count = options.blank_lines_before_deffunc;
        } else if directive == b"defcfunc" || directive == b"chsp_defcfunc" {
            count = options.blank_lines_before_defcfunc;
        } else if !label
            && !one_of(
                &directive,
                &[b"modfunc", b"modcfunc", b"modinit", b"modterm"],
            )
        {
            continue;
        }

        let mut start = i;
        while start > 0 {
            let previous = &lines[start - 1];
            if previous.opaque
                || previous.first + 1 != previous.last
                || tokens[previous.first].kind != Kind::Comment
                || source[tokens[previous.first].begin..tokens[previous.first].end]
                    .windows(7)
                    .any(|w| w == b"hspfmt:")
            {
                break;
            }
            start -= 1;
        }
        let content = start;
        if count < 0 && (options.preserve_indent || content == i) {
            continue;
        }
        if count >= 0 {
            while start > 0
                && !lines[start - 1].opaque
                && lines[start - 1].first == lines[start - 1].last
            {
                start -= 1;
            }
        }
        if lines[start].begin < copied {
            continue;
        }
        out.extend_from_slice(&source[copied..lines[start].begin]);
        let at_start = count >= 0 && start == 0;
        if at_start && source.starts_with(b"\xef\xbb\xbf") {
            out.extend_from_slice(b"\xef\xbb\xbf");
        }
        let newline = if start > 0 {
            lines[start - 1].newline
        } else {
            line.newline
        };
        if !at_start {
            for _ in 0..count {
                out.extend_from_slice(newline);
            }
        }
        copied = lines[content].begin;
        if at_start && copied == 0 && source.starts_with(b"\xef\xbb\xbf") {
            copied = 3;
        }
        if !options.preserve_indent {
            let indentation = &source[line.begin..tokens[line.first].begin];
            for j in content..i {
                let comment = &lines[j];
                if copied == 0 && source.starts_with(b"\xef\xbb\xbf") {
                    out.extend_from_slice(b"\xef\xbb\xbf");
                }
                out.extend_from_slice(indentation);
                out.extend_from_slice(&source[tokens[comment.first].begin..comment.end]);
                out.extend_from_slice(comment.newline);
                copied = comment.end + comment.newline.len();
            }
        }
    }
    out.extend_from_slice(&source[copied..]);
    Ok(out)
}

fn declaration_type_end(items: &[Item], mut pos: usize, end: usize, chsp: bool) -> usize {
    let begin = pos;
    if pos == end || items[pos].kind != Kind::Word {
        return begin;
    }
    let type_name = lower(&items[pos].text);
    pos += 1;
    if !chsp {
        return if one_of(
            &type_name,
            &[
                b"int", b"double", b"str", b"var", b"array", b"label", b"local",
            ],
        ) {
            pos
        } else {
            begin
        };
    }
    if one_of(&type_name, &[b"int", b"int64", b"double", b"str", b"label"]) {
        return pos;
    }
    if !one_of(&type_name, &[b"array", b"local"]) || pos == end || items[pos].text != b"[" {
        return begin;
    }
    pos += 1;
    if pos == end
        || items[pos].kind != Kind::Word
        || !one_of(
            &lower(&items[pos].text),
            &[b"int", b"int64", b"double", b"str"],
        )
    {
        return begin;
    }
    pos += 1;
    let mut dimensions = 0;
    while type_name == b"local" && pos < end && items[pos].text == b"[" {
        dimensions += 1;
        if dimensions > 4
            || pos + 2 >= end
            || items[pos + 1].kind != Kind::Number
            || items[pos + 1].text.is_empty()
            || !items[pos + 1].text.iter().all(|&b| b.is_ascii_digit())
            || items[pos + 2].text != b"]"
        {
            return begin;
        }
        pos += 3;
    }
    if pos == end || items[pos].text != b"]" {
        return begin;
    }
    pos + 1
}

fn declaration_commas(raw: &[u8], items: &[Item], options: &Options) -> Vec<u8> {
    if options.comma_spacing == Spacing::Preserve || items.len() < 3 {
        return raw.to_vec();
    }
    let directive = lower(&items[1].text);
    let chsp = one_of(&directive, &[b"chsp_deffunc", b"chsp_defcfunc"]);
    if !chsp && !one_of(&directive, &[b"deffunc", b"defcfunc"]) {
        return raw.to_vec();
    }
    let mut last = items.len();
    if items.last().map(|it| it.kind) == Some(Kind::Comment) {
        last -= 1;
    }
    let mut pos = 2;
    if !chsp && pos < last && lower(&items[pos].text) == b"local" {
        pos += 1;
    }
    if pos == last || items[pos].kind != Kind::Word || lower(&items[pos].text) == b"prep" {
        return raw.to_vec();
    }
    pos += 1;
    let arrow =
        |i: usize| chsp && i + 1 < last && items[i].text == b"-" && items[i + 1].text == b">";
    while pos < last {
        if arrow(pos) {
            break;
        }
        let name = declaration_type_end(items, pos, last, chsp);
        if name == pos || name == last || items[name].kind != Kind::Word {
            return raw.to_vec();
        }
        pos = name + 1;
        if pos == last || arrow(pos) {
            break;
        }
        if items[pos].text != b"," {
            return raw.to_vec();
        }
        pos += 1;
        if pos == last || arrow(pos) {
            return raw.to_vec();
        }
    }
    if chsp {
        if arrow(pos) {
            if pos + 3 != last || items[pos + 2].kind != Kind::Word {
                return raw.to_vec();
            }
            let result = lower(&items[pos + 2].text);
            if directive == b"chsp_defcfunc" {
                if !one_of(&result, &[b"int", b"int64", b"double", b"str"]) {
                    return raw.to_vec();
                }
            } else if result != b"void" {
                return raw.to_vec();
            }
        } else if directive == b"chsp_defcfunc" {
            return raw.to_vec();
        }
    }
    let mut out = Vec::new();
    let mut consumed = 0;
    for (i, item) in items.iter().enumerate() {
        if item.text != b"," {
            if i > 0 && items[i - 1].text == b"," {
                if options.comma_spacing == Spacing::Space {
                    out.push(b' ');
                }
            } else {
                out.extend_from_slice(&item.gap);
            }
        }
        out.extend_from_slice(&item.text);
        consumed += item.gap.len() + item.text.len();
    }
    out.extend_from_slice(&raw[consumed..]);
    out
}

fn close_block(state: &mut State, close: &[u8], line: usize) -> Result<(), Error> {
    if state.blocks.is_empty() || state.blocks.last().unwrap().close != close {
        return Err(Error::new(
            line,
            format!(
                "unmatched block terminator: {}",
                String::from_utf8_lossy(close)
            ),
        ));
    }
    state.blocks.pop();
    Ok(())
}

fn parse_line(items: &[Item], state: &mut State, line: usize) -> Result<(), Error> {
    let mut statement = true;
    for item in items {
        if item.kind == Kind::Comment {
            continue;
        }
        let word = lower(&item.text);
        if item.text == b"{" {
            state.blocks.push(Frame {
                close: b"}".to_vec(),
                case_body: false,
                line,
            });
            statement = true;
        } else if item.text == b"}" {
            close_block(state, b"}", line)?;
            statement = true;
        } else if item.text == b":" {
            statement = true;
        } else {
            if statement {
                if word == b"repeat" || word == b"foreach" {
                    state.blocks.push(Frame {
                        close: b"loop".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"while" {
                    state.blocks.push(Frame {
                        close: b"wend".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"for" {
                    state.blocks.push(Frame {
                        close: b"next".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"do" {
                    state.blocks.push(Frame {
                        close: b"until".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"switch" {
                    state.blocks.push(Frame {
                        close: b"swend".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if one_of(&word, &[b"loop", b"wend", b"next", b"until", b"swend"]) {
                    close_block(state, &word, line)?;
                } else if word == b"case" || word == b"default" {
                    if state.blocks.is_empty() || state.blocks.last().unwrap().close != b"swend" {
                        return Err(Error::new(line, "case/default outside switch"));
                    }
                    state.blocks.last_mut().unwrap().case_body = true;
                }
            }
            statement = false;
        }
    }
    Ok(())
}

fn binary(s: &[u8]) -> bool {
    one_of(
        s,
        &[
            b"=", b"==", b"!", b"!=", b"+", b"-", b"*", b"/", b"\\", b"&", b"|", b"^", b"&&",
            b"||", b"<", b">", b"<=", b">=", b"<<", b">>", b"+=", b"-=", b"*=", b"/=", b"\\=",
            b"&=", b"|=", b"^=", b"and", b"or", b"xor",
        ],
    )
}

fn next_code(items: &[Item], mut pos: usize, end: usize) -> usize {
    while pos < end && items[pos].kind == Kind::Comment {
        pos += 1;
    }
    pos
}

fn variable_end(items: &[Item], begin: usize, end: usize) -> usize {
    if begin == end
        || items[begin].kind != Kind::Word
        || one_of(
            &lower(&items[begin].text),
            &[
                b"if", b"else", b"while", b"until", b"repeat", b"foreach", b"for", b"switch",
                b"case", b"return", b"mes", b"print", b"logmes",
            ],
        )
    {
        return begin;
    }
    let mut pos = next_code(items, begin + 1, end);
    while pos < end {
        if items[pos].text == b"@" {
            let name = next_code(items, pos + 1, end);
            pos = if name < end && items[name].kind == Kind::Word {
                next_code(items, name + 1, end)
            } else {
                name
            };
        } else if items[pos].text == b"." {
            pos = next_code(items, pos + 1, end);
            if pos == end {
                return begin;
            }
            if items[pos].kind == Kind::Word || items[pos].kind == Kind::Number {
                pos = next_code(items, pos + 1, end);
                continue;
            }
            if items[pos].text != b"(" {
                return begin;
            }
        } else if items[pos].text != b"(" {
            break;
        }
        if pos < end && items[pos].text == b"(" {
            let mut depth = 0;
            loop {
                if items[pos].text == b"(" {
                    depth += 1;
                } else if items[pos].text == b")" {
                    depth -= 1;
                }
                pos += 1;
                if pos >= end || depth <= 0 {
                    break;
                }
            }
            if depth != 0 {
                return begin;
            }
            pos = next_code(items, pos, end);
        }
    }
    pos
}

fn operator_spelling(mut items: Vec<Item>, options: &Options) -> Vec<Item> {
    if options.operator_style == OperatorStyle::Preserve
        && options.increment_style == OperatorStyle::Preserve
    {
        return items;
    }
    let mut begin = 0;
    while begin < items.len() {
        let mut end = begin;
        while end < items.len()
            && !(items[end].kind == Kind::Symbol && one_of(&items[end].text, &[b":", b"{", b"}"]))
        {
            end += 1;
        }
        let start = next_code(&items, begin, end);
        let target = variable_end(&items, start, end);
        let variable_statement = target > start
            && target < end
            && (binary(&items[target].text) || one_of(&items[target].text, &[b"++", b"--"]));
        if variable_statement
            && options.increment_style != OperatorStyle::Preserve
            && one_of(&items[target].text, &[b"+", b"-", b"++", b"--"])
            && next_code(&items, target + 1, end) == end
        {
            let plus = items[target].text[0] == b'+';
            items[target].text = if options.increment_style == OperatorStyle::Hsp {
                if plus {
                    b"+".to_vec()
                } else {
                    b"-".to_vec()
                }
            } else if plus {
                b"++".to_vec()
            } else {
                b"--".to_vec()
            };
        }
        if options.operator_style != OperatorStyle::Preserve {
            for i in start..end {
                if items[i].kind != Kind::Symbol || (variable_statement && i == target) {
                    continue;
                }
                let mut left = i;
                while left > start && items[left - 1].kind == Kind::Comment {
                    left -= 1;
                }
                let right = next_code(&items, i + 1, end);
                if left == start || right == end || left - 1 == start {
                    continue;
                }
                let before = &items[left - 1];
                let after = &items[right];
                let operand_before = (before.kind == Kind::Word && !binary(&lower(&before.text)))
                    || before.kind == Kind::Number
                    || before.kind == Kind::String
                    || one_of(&before.text, &[b")", b"]"]);
                let operand_after = (after.kind == Kind::Word && !binary(&lower(&after.text)))
                    || after.kind == Kind::Number
                    || after.kind == Kind::String
                    || one_of(&after.text, &[b"(", b"[", b"-", b"+", b"*"]);
                if !operand_before || !operand_after {
                    continue;
                }
                let text = &mut items[i].text;
                if options.operator_style == OperatorStyle::Hsp {
                    if text == b"&&" {
                        *text = b"&".to_vec();
                    } else if text == b"||" {
                        *text = b"|".to_vec();
                    } else if text == b"!=" {
                        *text = b"!".to_vec();
                    } else if text == b"==" {
                        *text = b"=".to_vec();
                    }
                } else if text == b"&" {
                    *text = b"&&".to_vec();
                } else if text == b"|" {
                    *text = b"||".to_vec();
                } else if text == b"!" {
                    *text = b"!=".to_vec();
                } else if text == b"=" {
                    *text = b"==".to_vec();
                }
            }
        }
        begin = if end == items.len() { end } else { end + 1 };
    }
    items
}

fn wraps_expression(items: &[Item], begin: usize, end: usize) -> bool {
    if end - begin < 2 || items[begin].text != b"(" || items[end - 1].text != b")" {
        return false;
    }
    let mut depth = 0;
    for i in begin..end {
        if items[i].text == b"(" {
            depth += 1;
        } else if items[i].text == b")" {
            depth -= 1;
        }
        if depth <= 0 && i != end - 1 {
            return false;
        }
    }
    depth == 0
}

fn expression_parens(items: &[Item], options: &Options) -> Vec<Item> {
    if options.condition_parens == Parentheses::Preserve
        && options.repeat_parens == Parentheses::Preserve
    {
        return items.to_vec();
    }
    let mut result = Vec::new();
    let mut statement = true;
    let mut i = 0;
    while i < items.len() {
        let word = lower(&items[i].text);
        let condition =
            statement && items[i].kind == Kind::Word && (word == b"if" || word == b"while");
        let repeat = statement && items[i].kind == Kind::Word && word == b"repeat";
        let mode = if condition {
            options.condition_parens
        } else if repeat {
            options.repeat_parens
        } else {
            Parentheses::Preserve
        };
        result.push(items[i].clone());
        i += 1;
        if mode != Parentheses::Preserve {
            let mut end = i;
            let mut stack: Vec<&[u8]> = Vec::new();
            let mut eligible = true;
            while end < items.len() {
                let item = &items[end];
                if item.kind == Kind::Comment || one_of(&item.text, &[b":", b"{", b"}"]) {
                    break;
                }
                if item.text == b"(" || item.text == b"[" {
                    stack.push(&item.text);
                } else if item.text == b")" || item.text == b"]" {
                    let expected = if item.text == b")" {
                        b"(".as_slice()
                    } else {
                        b"[".as_slice()
                    };
                    if stack.is_empty() || stack.last().copied() != Some(expected) {
                        eligible = false;
                        break;
                    }
                    stack.pop();
                } else if item.text == b"," && stack.is_empty() && !repeat {
                    eligible = false;
                }
                end += 1;
            }
            if !stack.is_empty() {
                eligible = false;
            }
            if end < items.len()
                && items[end].kind == Kind::Comment
                && items[end].text.starts_with(b"/*")
            {
                eligible = false;
            }
            if eligible {
                let mut start = i;
                let mut depth = 0;
                for t in i..=end {
                    if t < end && (items[t].text == b"(" || items[t].text == b"[") {
                        depth += 1;
                    }
                    if t < end && (items[t].text == b")" || items[t].text == b"]") {
                        depth -= 1;
                    }
                    if t != end && !(repeat && items[t].text == b"," && depth == 0) {
                        continue;
                    }
                    let mut first = start;
                    let mut last = t;
                    if mode == Parentheses::Remove {
                        while wraps_expression(items, first, last) {
                            first += 1;
                            last -= 1;
                        }
                    }
                    let add = first < last
                        && mode == Parentheses::Add
                        && !wraps_expression(items, first, last);
                    if add {
                        result.push(Item {
                            kind: Kind::Symbol,
                            text: b"(".to_vec(),
                            gap: b" ".to_vec(),
                        });
                    }
                    for p in first..last {
                        let mut item = items[p].clone();
                        if p == first {
                            item.gap = if add { b"".to_vec() } else { b" ".to_vec() };
                        }
                        result.push(item);
                    }
                    if add {
                        result.push(Item {
                            kind: Kind::Symbol,
                            text: b")".to_vec(),
                            gap: b"".to_vec(),
                        });
                    }
                    if t < end {
                        result.push(items[t].clone());
                    }
                    start = t + 1;
                }
                i = end;
            }
        }
        let previous = &items[i - 1];
        if one_of(&previous.text, &[b":", b"{", b"}"]) {
            statement = true;
        } else if previous.kind != Kind::Comment {
            statement = word == b"else";
        }
    }
    result
}

fn ambiguous_label_gaps(items: &[Item]) -> Vec<bool> {
    let mut gaps = vec![false; items.len()];
    let mut begin = 0;
    while begin < items.len() {
        let mut end = begin;
        while end < items.len() && !one_of(&items[end].text, &[b":", b"{", b"}"]) {
            end += 1;
        }
        let start = next_code(items, begin, end);
        if start < end
            && !one_of(
                &lower(&items[start].text),
                &[
                    b"goto", b"gosub", b"onclick", b"onkey", b"onexit", b"onerror", b"oncmd",
                    b"button",
                ],
            )
        {
            let mut pos = start + 1;
            if items[start].kind == Kind::Word && variable_end(items, start, end) != start {
                pos = next_code(items, pos, end);
                while pos < end && items[pos].text == b"@" {
                    pos = next_code(items, pos + 1, end);
                    if pos < end && items[pos].kind == Kind::Word {
                        pos = next_code(items, pos + 1, end);
                    }
                }
                if pos + 1 < end
                    && items[pos].text == b"*"
                    && (items[pos + 1].kind == Kind::Word || items[pos + 1].text == b"@")
                {
                    gaps[pos] = true;
                    gaps[pos + 1] = true;
                }
            }
        }
        begin = if end == items.len() { end } else { end + 1 };
    }
    gaps
}

fn print_items(
    items: &[Item],
    options: &Options,
    ambiguous_spacing: Option<&mut bool>,
) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let preserved_gaps = ambiguous_label_gaps(items);
    let mut previous_prefix = false;
    let mut expect_operand = true;
    let mut statement = true;
    let mut parens = 0;
    let mut previous_command = false;
    let mut found_ambiguity = false;

    for i in 0..items.len() {
        let item = &items[i];
        let mut text = item.text.clone();
        let word = lower(&text);
        let prev = if i > 0 { &items[i - 1].text[..] } else { b"" };
        let next = if i + 1 < items.len() {
            &items[i + 1].text[..]
        } else {
            b""
        };

        let jump = parens == 0 && item.kind == Kind::Word && one_of(&word, &[b"goto", b"gosub"]);
        let command = statement || jump || (previous_command && (text == b"@" || prev == b"@"));
        let prefix = expect_operand && one_of(&text, &[b"-", b"+", b"*"]);
        let postfix = one_of(&text, &[b"+", b"-", b"++", b"--"])
            && i > 0
            && (next.is_empty()
                || next == b":"
                || next == b"}"
                || items[i + 1].kind == Kind::Comment);
        let mut space = i != 0;
        let mut preserve_gap = false;

        if i > 0 {
            let mut apply_spacing = |mode: Spacing| {
                preserve_gap = mode == Spacing::Preserve;
                space = mode != Spacing::Compact;
            };
            if item.kind == Kind::Comment {
                apply_spacing(options.comment_spacing);
            } else if text == b":" || prev == b":" {
                apply_spacing(options.colon_spacing);
            } else if text == b"{" || prev == b"{" || text == b"}" || prev == b"}" {
                space = true;
            } else if text == b"," {
                preserve_gap = options.comma_spacing == Spacing::Preserve;
                space = false;
            } else if text == b")" || text == b"]" {
                space = false;
            } else if prev == b"," {
                apply_spacing(options.comma_spacing);
            } else if prev == b"(" || prev == b"[" || previous_prefix || postfix {
                space = false;
            } else if text == b"(" || text == b"[" {
                if binary(&lower(prev)) {
                    apply_spacing(options.operator_spacing);
                } else {
                    space = !item.gap.is_empty();
                }
            } else if text == b"."
                || prev == b"."
                || text == b"@"
                || (prev == b"@" && !binary(&word))
            {
                space = false;
            } else if binary(&word) || binary(&lower(prev)) {
                apply_spacing(options.operator_spacing);
            }

            if (item.kind == Kind::Word && items[i - 1].kind == Kind::Word)
                || (item.kind == Kind::Number && items[i - 1].kind == Kind::Word)
                || (item.kind == Kind::Word && items[i - 1].kind == Kind::Number)
            {
                space = true;
            }

            if previous_command
                && !item.gap.is_empty()
                && !one_of(
                    &text,
                    &[
                        b"=", b"+=", b"-=", b"*=", b"/=", b"\\=", b"&=", b"|=", b"^=",
                    ],
                )
                && !postfix
                && !one_of(&text, &[b"(", b"[", b".", b"@", b":", b"{", b"}"])
            {
                space = true;
            }

            if !space && !prev.is_empty() && !text.is_empty() {
                let combined = [prev[prev.len() - 1], text[0]];
                if one_of(
                    &combined,
                    &[
                        b"++", b"--", b"/*", b"//", b"==", b"!=", b"<=", b">=", b"<<", b">>",
                        b"&&", b"||", b"+=", b"-=", b"*=", b"/=", b"\\=", b"&=", b"|=", b"^=",
                    ],
                ) {
                    space = true;
                }
            }

            if item.kind == Kind::Word && one_of(&word, &[b"and", b"or", b"xor"]) {
                space = true;
            }
            if one_of(&lower(prev), &[b"and", b"or", b"xor"]) {
                space = true;
            }
        }

        if options.hsp_numeric_prefixes && item.kind == Kind::Number && text.len() > 2 {
            if text.starts_with(b"0x") {
                text = [b"$", &text[2..]].concat();
            } else if text.starts_with(b"0b") {
                text = [b"%", &text[2..]].concat();
            }
        }

        if preserved_gaps[i] {
            let desired: &[u8] = if options.operator_spacing == Spacing::Preserve {
                &item.gap
            } else if options.operator_spacing == Spacing::Space {
                b" "
            } else {
                b""
            };
            if item.gap != desired {
                found_ambiguity = true;
            }
            out.extend_from_slice(&item.gap);
        } else if preserve_gap {
            out.extend_from_slice(&item.gap);
        } else if space {
            out.push(b' ');
        }

        out.extend_from_slice(&text);
        previous_command = command && (item.kind == Kind::Word || text == b"@");
        previous_prefix = prefix;
        if text == b"(" || text == b"[" {
            parens += 1;
            expect_operand = true;
        } else if text == b")" || text == b"]" {
            parens -= 1;
            expect_operand = false;
        } else if one_of(&text, &[b":", b"{", b"}"]) {
            statement = true;
            expect_operand = true;
        } else if text == b"," || binary(&word) {
            expect_operand = true;
            statement = false;
        } else if item.kind != Kind::Comment {
            expect_operand = command && item.kind == Kind::Word && parens == 0;
            statement = false;
        }
    }

    if let Some(ambig) = ambiguous_spacing {
        *ambig = found_ambiguity;
    }

    let printed_tokens = lex(&out, options.encoding)?;
    let mut index = 0;
    for token in &printed_tokens {
        if token.kind == Kind::Space {
            continue;
        }
        if index >= items.len() {
            return Err(Error::new(0, "spacing changed token boundaries"));
        }
        let mut expected = items[index].text.clone();
        if options.hsp_numeric_prefixes && items[index].kind == Kind::Number {
            if expected.starts_with(b"0x") {
                expected = [b"$", &expected[2..]].concat();
            } else if expected.starts_with(b"0b") {
                expected = [b"%", &expected[2..]].concat();
            }
        }
        if token.kind != items[index].kind || &out[token.begin..token.end] != expected {
            return Err(Error::new(0, "spacing changed token boundaries"));
        }
        index += 1;
    }
    if index != items.len() {
        return Err(Error::new(0, "spacing changed token boundaries"));
    }

    Ok(out)
}

fn followed_by_else(source: &[u8], tokens: &[Token], mut next: usize) -> bool {
    while next < tokens.len() {
        let token = &tokens[next];
        if token.kind == Kind::Space
            || token.kind == Kind::Newline
            || token.kind == Kind::Comment
            || token.kind == Kind::Bom
        {
            next += 1;
            continue;
        }
        return token.kind == Kind::Word && lower(&source[token.begin..token.end]) == b"else";
    }
    false
}

fn short_if(items: Vec<Item>, options: &Options, indent_size: usize) -> Result<Vec<Item>, Error> {
    if !options.short_if || items.len() < 5 || lower(&items[0].text) != b"if" {
        return Ok(items);
    }
    for item in &items {
        if item.kind == Kind::Comment {
            return Ok(items);
        }
    }
    if items.last().unwrap().text != b"}" {
        return Ok(items);
    }
    let mut open = 1;
    while open < items.len() && items[open].text != b"{" {
        open += 1;
    }
    if open < 2 || open == items.len() {
        return Ok(items);
    }
    for i in 1..open {
        if one_of(&items[i].text, &[b":", b"}"]) {
            return Ok(items);
        }
    }
    let mut start = open + 1;
    for i in start..items.len() {
        if items[i].text == b"{" {
            return Ok(items);
        }
        if items[i].text == b":" || items[i].text == b"}" {
            if items[i].text == b"}" && i != items.len() - 1 {
                return Ok(items);
            }
            if i == start || items[start].kind != Kind::Word {
                return Ok(items);
            }
            if one_of(
                &lower(&items[start].text),
                &[
                    b"if", b"else", b"repeat", b"foreach", b"loop", b"while", b"wend", b"for",
                    b"next", b"do", b"until", b"switch", b"case", b"default", b"swbreak", b"swend",
                ],
            ) {
                return Ok(items);
            }
            start = i + 1;
        }
    }
    let mut result = items.clone();
    result.pop();
    result[open].text = b":".to_vec();
    if result[1].text == b"(" && result[open - 1].text == b")" {
        let mut nesting = 0;
        let mut wraps = true;
        for i in 1..open {
            if result[i].text == b"(" {
                nesting += 1;
            }
            if result[i].text == b")" {
                nesting -= 1;
            }
            if nesting == 0 && i != open - 1 {
                wraps = false;
            }
        }
        if wraps && nesting == 0 {
            result.remove(open - 1);
            result.remove(1);
        }
    }
    let spelled = operator_spelling(expression_parens(&result, options), options);
    let printed = print_items(&spelled, options, None)?;
    if printed.len() + indent_size > options.line_width {
        return Ok(items);
    }
    Ok(result)
}

fn short_if_lines(source: &[u8], options: &Options) -> Result<Vec<u8>, Error> {
    if !options.short_if {
        return Ok(source.to_vec());
    }
    let tokens = lex(source, options.encoding)?;
    let lines = source_lines(source, &tokens);
    let text = |t: usize| &source[tokens[t].begin..tokens[t].end];
    let mut out = Vec::new();
    let mut copied = 0;
    let mut i = 0;
    while i + 2 < lines.len() {
        let head = &lines[i];
        let body = &lines[i + 1];
        let close = &lines[i + 2];
        if head.opaque
            || body.opaque
            || close.opaque
            || head.multiline
            || body.multiline
            || close.multiline
            || head.directive
            || body.directive
            || close.directive
            || head.first == head.last
            || body.first == body.last
            || close.first + 1 != close.last
            || lower(text(head.first)) != b"if"
            || text(head.last - 1) != b"{"
            || text(close.first) != b"}"
            || followed_by_else(source, &tokens, close.last)
        {
            i += 1;
            continue;
        }
        let mut items = Vec::new();
        for j in i..=i + 2 {
            let mut previous = tokens[lines[j].first].begin;
            for t in lines[j].first..lines[j].last {
                let token = &tokens[t];
                if token.kind == Kind::Space {
                    continue;
                }
                let gap = if t == lines[j].first {
                    if j == i {
                        b"".to_vec()
                    } else {
                        b" ".to_vec()
                    }
                } else {
                    source[previous..token.begin].to_vec()
                };
                items.push(Item {
                    kind: token.kind,
                    text: text(t).to_vec(),
                    gap,
                });
                previous = token.end;
            }
        }
        let prefix = &source[head.begin..tokens[head.first].begin];
        let indent_size = prefix.len()
            - if prefix.starts_with(b"\xef\xbb\xbf") {
                3
            } else {
                0
            };
        let shortened = short_if(items.clone(), options, indent_size)?;
        if shortened.len() == items.len() {
            i += 1;
            continue;
        }
        out.extend_from_slice(&source[copied..head.begin]);
        out.extend_from_slice(prefix);
        let spelled = operator_spelling(expression_parens(&shortened, options), options);
        let printed = print_items(&spelled, options, None)?;
        out.extend_from_slice(&printed);
        out.extend_from_slice(close.newline);
        copied = close.end + close.newline.len();
        i += 3;
    }
    out.extend_from_slice(&source[copied..]);
    Ok(out)
}

pub fn lex(s: &[u8], encoding: Encoding) -> Result<Vec<Token>, Error> {
    let enc = if encoding == Encoding::Auto {
        detect_encoding(s)?
    } else {
        encoding
    };
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let begin = i;
        let mut kind = Kind::Symbol;
        if i == 0 && s.starts_with(b"\xef\xbb\xbf") {
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
                    || (hex && ((b'a'..=b'f').contains(&c) || (b'A'..=b'F').contains(&c)))
                {
                    i += 1;
                } else if !hex && !bin && c == b'.' && i + 1 < s.len() && digit(s[i + 1]) {
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

struct FormatContext<'a> {
    source: &'a [u8],
    options: &'a Options,
    diagnostics: Option<&'a mut Vec<Diagnostic>>,
    original_lines: &'a [&'a [u8]],
    tokens: &'a [Token],

    state: State,
    conditionals: Vec<Conditional>,
    items: Vec<Item>,
    out: Vec<u8>,
    begin: usize,
    protected_line: bool,
    continuation: bool,
    gap: Vec<u8>,
    line_number: usize,
    line_start: usize,
    disabled: bool,
    ignored: bool,
}

impl<'a> FormatContext<'a> {
    fn flush(&mut self, end: usize, newline: &[u8], next_token: usize) -> Result<(), Error> {
        let raw = &self.source[self.begin..end];
        let mut preserve =
            self.protected_line || self.continuation || self.disabled || self.ignored;
        let next_continuation = !self.items.is_empty() && self.items.last().unwrap().text == b"\\";
        if next_continuation {
            preserve = true;
        }
        let was_disabled = self.disabled;
        let was_ignored = self.ignored;
        self.ignored = false;
        if self.items.len() == 1 && self.items[0].kind == Kind::Comment {
            let kind = marker_kind(&self.items[0].text);
            if kind == Marker::Off {
                self.disabled = true;
                preserve = true;
            } else if kind == Marker::On {
                self.disabled = false;
                preserve = true;
            } else if kind == Marker::Ignore {
                self.ignored = true;
                preserve = true;
            }
        }
        self.ignored = self.ignored || (was_ignored && next_continuation);
        let opaque = self.continuation || next_continuation || was_disabled || self.disabled;
        let label =
            self.items.len() >= 2 && self.items[0].text == b"*" && self.items[1].kind == Kind::Word;
        let directive = if self.items.len() > 1 && self.items[0].text == b"#" {
            lower(&self.items[1].text)
        } else {
            Vec::new()
        };
        let continued_chsp = next_continuation
            && !self.continuation
            && !was_disabled
            && !self.disabled
            && one_of(
                &directive,
                &[b"chsp_module", b"chsp_deffunc", b"chsp_defcfunc"],
            );

        if (!opaque || continued_chsp) && !self.items.is_empty() && self.items[0].text == b"#" {
            preserve = true;
            if one_of(&directive, &[b"if", b"ifdef", b"ifndef"]) {
                self.conditionals.push(Conditional {
                    before: self.state.clone(),
                    branch: State::default(),
                    has_branch: false,
                    has_else: false,
                    line: self.line_start,
                });
            } else if directive == b"else" || directive == b"elif" {
                if self.conditionals.is_empty() {
                    return Err(Error::new(self.line_start, "unmatched preprocessor branch"));
                }
                let c = self.conditionals.last_mut().unwrap();
                if c.has_else {
                    return Err(Error::new(self.line_start, "branch after #else"));
                }
                if c.has_branch && !c.branch.same_structure(&self.state) {
                    return Err(Error::new(
                        self.line_start,
                        "conditional branches have different block structure",
                    ));
                }
                if c.has_branch {
                    self.state.function = self.state.function.max(c.branch.function);
                    self.state.label = self.state.label.max(c.branch.label);
                }
                c.branch = self.state.clone();
                c.has_branch = true;
                c.has_else = directive == b"else";
                self.state = c.before.clone();
            } else if directive == b"endif" {
                if self.conditionals.is_empty() {
                    return Err(Error::new(self.line_start, "unmatched #endif"));
                }
                let c = self.conditionals.pop().unwrap();
                if (c.has_branch && !c.branch.same_structure(&self.state))
                    || (!c.has_else && !c.before.same_structure(&self.state))
                {
                    return Err(Error::new(
                        self.line_start,
                        "conditional branches have different block structure",
                    ));
                }
                if c.has_branch {
                    self.state.function = self.state.function.max(c.branch.function);
                    self.state.label = self.state.label.max(c.branch.label);
                }
                if !c.has_else {
                    self.state.function = self.state.function.max(c.before.function);
                    self.state.label = self.state.label.max(c.before.label);
                }
            } else if directive == b"chsp_module" {
                if !self.state.blocks.is_empty() || self.state.chsp_module != 0 {
                    return Err(Error::new(
                        self.line_start,
                        "nested cHSP module or module inside open block",
                    ));
                }
                self.state.chsp_module = self.line_start;
                self.state.function = 0;
                self.state.label = 0;
            } else if one_of(&directive, &[b"chsp_deffunc", b"chsp_defcfunc"]) {
                if self.state.chsp_module == 0 {
                    return Err(Error::new(
                        self.line_start,
                        "cHSP function outside #chsp_module",
                    ));
                }
                if !self.state.blocks.is_empty() || self.state.chsp_function != 0 {
                    return Err(Error::new(
                        self.line_start,
                        "cHSP function inside open block or function",
                    ));
                }
                self.state.chsp_function = self.line_start;
                self.state.function = 1;
                self.state.label = 0;
            } else if directive == b"chsp_end" {
                if self.state.chsp_function == 0 {
                    return Err(Error::new(self.line_start, "unmatched #chsp_end"));
                }
                if !self.state.blocks.is_empty() {
                    return Err(Error::new(self.line_start, "#chsp_end inside open block"));
                }
                self.state.chsp_function = 0;
                self.state.function = 0;
                self.state.label = 0;
            } else if directive == b"chsp_module_end" {
                if self.state.chsp_module == 0 {
                    return Err(Error::new(self.line_start, "unmatched #chsp_module_end"));
                }
                if !self.state.blocks.is_empty() || self.state.chsp_function != 0 {
                    return Err(Error::new(
                        self.line_start,
                        "#chsp_module_end inside open block or function",
                    ));
                }
                self.state.chsp_module = 0;
                self.state.function = 0;
                self.state.label = 0;
            } else if one_of(
                &directive,
                &[
                    b"deffunc",
                    b"defcfunc",
                    b"modfunc",
                    b"modcfunc",
                    b"modinit",
                    b"modterm",
                ],
            ) {
                if self.state.chsp_function != 0 {
                    return Err(Error::new(
                        self.line_start,
                        "HSP function inside cHSP function",
                    ));
                }
                if !self.state.blocks.is_empty() {
                    return Err(Error::new(
                        self.line_start,
                        "function declaration inside open block",
                    ));
                }
                self.state.function = 1;
                self.state.label = 0;
            } else if directive == b"global" || directive == b"module" {
                if self.state.chsp_function != 0 {
                    return Err(Error::new(
                        self.line_start,
                        "HSP module boundary inside cHSP function",
                    ));
                }
                if !self.state.blocks.is_empty() {
                    return Err(Error::new(
                        self.line_start,
                        "module boundary inside open block",
                    ));
                }
                self.state.function = 0;
                self.state.label = 0;
            }
        } else if !preserve {
            let mut line_state = self.state.clone();
            if !self.items.is_empty() {
                let first = lower(&self.items[0].text);
                if one_of(
                    &first,
                    &[b"}", b"loop", b"wend", b"next", b"until", b"swend"],
                ) {
                    close_block(&mut line_state, &first, self.line_start)?;
                } else if (first == b"case" || first == b"default") && !line_state.blocks.is_empty()
                {
                    line_state.blocks.last_mut().unwrap().case_body = false;
                }
            }
            let depth = if label {
                0
            } else {
                line_state.depth(self.options)
            };
            let first_non_space = raw
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .unwrap_or(raw.len());
            let indentation = &raw[..first_non_space];
            let indent_size = if self.options.preserve_indent {
                indentation.len()
            } else if self.options.tabs {
                depth
            } else {
                depth * self.options.indent_width
            };
            let following_else = self.options.short_if
                && !self.items.is_empty()
                && lower(&self.items[0].text) == b"if"
                && followed_by_else(self.source, self.tokens, next_token);
            let printed = if following_else {
                self.items.clone()
            } else {
                short_if(self.items.clone(), self.options, indent_size)?
            };
            let printed = expression_parens(&printed, self.options);
            let printed = operator_spelling(printed, self.options);
            if !self.items.is_empty() {
                if self.options.preserve_indent {
                    self.out.extend_from_slice(indentation);
                } else {
                    let fill_byte = if self.options.tabs { b'\t' } else { b' ' };
                    self.out.resize(self.out.len() + indent_size, fill_byte);
                }
                let mut ambiguous_spacing = false;
                let formatted_line =
                    print_items(&printed, self.options, Some(&mut ambiguous_spacing))?;
                self.out.extend_from_slice(&formatted_line);
                if ambiguous_spacing {
                    if let Some(diag) = self.diagnostics.as_deref_mut() {
                        diag.push(Diagnostic {
                            line: self.line_number,
                            source: self.original_lines[self.line_number - 1].to_vec(),
                        });
                    }
                }
            }
        }
        if !opaque && label && self.options.indent_labels {
            self.state.label = 1;
        }
        if !opaque && (self.items.is_empty() || self.items[0].text != b"#") {
            parse_line(&self.items, &mut self.state, self.line_start)?;
        }
        if preserve {
            if !opaque
                && !self.protected_line
                && !was_ignored
                && !self.items.is_empty()
                && self.items[0].text == b"#"
            {
                let formatted_decl = declaration_commas(raw, &self.items, self.options);
                self.out.extend_from_slice(&formatted_decl);
            } else {
                self.out.extend_from_slice(raw);
            }
        }
        self.out.extend_from_slice(newline);
        self.continuation = next_continuation;
        self.items.clear();
        self.protected_line = false;
        self.gap.clear();
        self.begin = end + newline.len();
        Ok(())
    }
}

pub fn format(
    source: &[u8],
    user_options: &Options,
    diagnostics: Option<&mut Vec<Diagnostic>>,
) -> Result<Vec<u8>, Error> {
    let mut options = user_options.clone();
    if options.encoding == Encoding::Auto {
        options.encoding = detect_encoding(source)?;
    }
    if options.indent_width > 16 {
        return Err(Error::new(0, "indent width must be between 0 and 16"));
    }
    if options.base_indent > 16 {
        return Err(Error::new(0, "base indent must be between 0 and 16"));
    }
    if options.loop_indent > 16 {
        return Err(Error::new(0, "loop indent must be between 0 and 16"));
    }
    for &count in &[
        options.blank_lines_before_module,
        options.blank_lines_before_deffunc,
        options.blank_lines_before_defcfunc,
    ] {
        if !(-1..=16).contains(&count) {
            return Err(Error::new(
                0,
                "blank line count must be between 0 and 16, or -1 to preserve",
            ));
        }
    }

    let mut original_lines = Vec::new();
    if diagnostics.is_some() {
        let mut pos = 0;
        while pos < source.len() {
            let mut end = pos;
            while end < source.len() && source[end] != b'\r' && source[end] != b'\n' {
                end += 1;
            }
            original_lines.push(&source[pos..end]);
            pos = end;
            if pos < source.len() && source[pos] == b'\r' {
                pos += 1;
            }
            if pos < source.len() && source[pos] == b'\n' {
                pos += 1;
            }
        }
    }

    let mut current_source: std::borrow::Cow<'_, [u8]> = std::borrow::Cow::Borrowed(source);
    if options.full_width_spaces == FullWidthSpaces::Normalize {
        current_source = std::borrow::Cow::Owned(normalize_full_width_spaces(
            &current_source,
            options.encoding,
        )?);
    }
    if options.comment_style != CommentStyle::Preserve
        || options.block_comments != BlockComments::Preserve
    {
        current_source = std::borrow::Cow::Owned(rewrite_comments(&current_source, &options)?);
    }

    let tokens = lex(&current_source, options.encoding)?;
    let mut ctx = FormatContext {
        source: &current_source,
        options: &options,
        diagnostics,
        original_lines: &original_lines,
        tokens: &tokens,
        state: State::default(),
        conditionals: Vec::new(),
        items: Vec::new(),
        out: Vec::new(),
        begin: 0,
        protected_line: false,
        continuation: false,
        gap: Vec::new(),
        line_number: 1,
        line_start: 1,
        disabled: false,
        ignored: false,
    };

    for (token_index, token) in tokens.iter().enumerate() {
        let text = &current_source[token.begin..token.end];
        if token.kind == Kind::Bom {
            ctx.out.extend_from_slice(text);
            ctx.begin = token.end;
        } else if token.kind == Kind::Newline {
            ctx.flush(token.begin, text, token_index + 1)?;
        } else if token.kind == Kind::Space {
            ctx.gap.extend_from_slice(text);
        } else {
            if text.contains(&b'\r') || text.contains(&b'\n') {
                ctx.protected_line = true;
            }
            ctx.items.push(Item {
                kind: token.kind,
                text: text.to_vec(),
                gap: ctx.gap.clone(),
            });
            ctx.gap.clear();
        }
        let mut i = 0;
        while i < text.len() {
            if text[i] == b'\r' {
                ctx.line_number += 1;
                if i + 1 < text.len() && text[i + 1] == b'\n' {
                    i += 1;
                }
            } else if text[i] == b'\n' {
                ctx.line_number += 1;
            }
            i += 1;
        }
        if token.kind == Kind::Newline {
            ctx.line_start = ctx.line_number;
        }
    }
    if ctx.begin < current_source.len() {
        ctx.flush(current_source.len(), b"", tokens.len())?;
    }

    if !ctx.conditionals.is_empty() {
        return Err(Error::new(
            ctx.conditionals.last().unwrap().line,
            "unterminated preprocessor conditional",
        ));
    }
    if !ctx.state.blocks.is_empty() {
        let close_str = String::from_utf8_lossy(&ctx.state.blocks.last().unwrap().close);
        return Err(Error::new(
            ctx.state.blocks.last().unwrap().line,
            format!("unterminated block, expected {close_str}"),
        ));
    }
    if ctx.state.chsp_function != 0 {
        return Err(Error::new(
            ctx.state.chsp_function,
            "unterminated cHSP function, expected #chsp_end",
        ));
    }
    if ctx.state.chsp_module != 0 {
        return Err(Error::new(
            ctx.state.chsp_module,
            "unterminated cHSP module, expected #chsp_module_end",
        ));
    }

    let out1 = short_if_lines(&ctx.out, &options)?;
    declaration_layout(&out1, &options)
}
