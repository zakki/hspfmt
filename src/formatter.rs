use crate::document::{Atom, Document};
use crate::encoding::{character_end, resolve_encoding, ResolvedEncoding};
use crate::lexer::{lex_fragment, lex_resolved};
use crate::parser::{close_block, parse_line, Conditional, Item, State};
use crate::util::{lower, one_of};
use crate::*;
use std::borrow::Cow;

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
struct SourceLine {
    begin: usize,
    end: usize,
    first: usize,
    last: usize,
    opaque: bool,
    directive: bool,
    multiline: bool,
}

fn source_lines(source: &Document<'_>) -> Vec<SourceLine> {
    let tokens = &source.atoms;
    let mut lines = Vec::new();
    let mut begin = 0;
    let mut disabled = false;
    let mut continuation = false;
    let mut ignored = false;
    let mut append = |begin: usize, end: usize| {
        let mut first = begin;
        while first < end && matches!(tokens[first].kind, Kind::Space | Kind::Bom) {
            first += 1;
        }
        let mut last = end;
        while last > first && tokens[last - 1].kind == Kind::Space {
            last -= 1;
        }
        let marker = if first + 1 == last && tokens[first].kind == Kind::Comment {
            marker_kind(source.text(first))
        } else {
            Marker::None
        };
        let next = first < last && source.text(last - 1) == b"\\";
        let multiline = tokens[first..last]
            .iter()
            .any(|token| token.text.contains(&b'\r') || token.text.contains(&b'\n'));
        lines.push(SourceLine {
            begin,
            end,
            first,
            last,
            opaque: disabled || ignored || continuation || next || marker != Marker::None,
            directive: first < last && source.text(first) == b"#",
            multiline,
        });
        if marker != Marker::None && marker != Marker::Ignore {
            disabled = marker == Marker::Off;
        }
        ignored = marker == Marker::Ignore || (ignored && next);
        continuation = next;
    };
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == Kind::Newline {
            append(begin, index);
            begin = index + 1;
        }
    }
    if begin < tokens.len() {
        append(begin, tokens.len());
    }
    lines
}

fn append_newline<'a>(out: &mut Document<'a>, source: &Document<'a>, line: &SourceLine) {
    if line.end < source.atoms.len() {
        out.append(&source.atoms[line.end..line.end + 1]);
    }
}

const SPACES: &[u8] = &[b' '; 256];
const TABS: &[u8] = &[b'\t'; 256];

// Indentation shorter than the static buffers borrows them instead of allocating per line.
fn indentation(tabs: bool, size: usize) -> Cow<'static, [u8]> {
    let fill = if tabs { TABS } else { SPACES };
    match fill.get(..size) {
        Some(text) => Cow::Borrowed(text),
        None => Cow::Owned(vec![fill[0]; size]),
    }
}

fn without_location(error: Error) -> Error {
    Error::new(None, error.message())
}

fn extend_atoms(out: &mut Vec<u8>, atoms: &[Atom<'_>]) {
    for atom in atoms {
        out.extend_from_slice(&atom.text);
    }
}

fn normalize_full_width_spaces<'a>(
    source: Document<'a>,
    encoding: ResolvedEncoding,
) -> Result<Document<'a>, Error> {
    let lines = source_lines(&source);
    let full_space: &[u8] = if encoding == ResolvedEncoding::Cp932 {
        b"\x81\x40"
    } else {
        b"\xe3\x80\x80"
    };
    let mut out = Document::default();
    let mut copied = 0;
    let mut disabled = false;
    let mut ignored = false;
    for line in &lines {
        let mut first = line.first;
        while first < line.last {
            if source.atoms[first].kind == Kind::Space {
                first += 1;
                continue;
            }
            if source.atoms[first].kind != Kind::Word {
                break;
            }
            let mut text = source.text(first);
            while text.starts_with(full_space) {
                text = &text[full_space.len()..];
            }
            if !text.is_empty() {
                break;
            }
            first += 1;
        }
        let marker = if first + 1 == line.last && source.atoms[first].kind == Kind::Comment {
            marker_kind(source.text(first))
        } else {
            Marker::None
        };
        let preserve = (disabled || ignored) && marker == Marker::None;
        let next = line.first < line.last && source.text(line.last - 1) == b"\\";
        ignored = marker == Marker::Ignore || (ignored && next);
        if matches!(marker, Marker::Off | Marker::On) {
            disabled = marker == Marker::Off;
        }
        if preserve {
            continue;
        }
        let mut index = line.first;
        while index < line.last {
            let atom = &source.atoms[index];
            if atom.kind != Kind::Word
                || !atom.text.windows(full_space.len()).any(|w| w == full_space)
            {
                index += 1;
                continue;
            }
            out.append(&source.atoms[copied..index]);
            // A normalized word can become a number which consumes neighbouring
            // '.', exponent signs or suffixes. Classify the complete adjacent run.
            let mut end = index + 1;
            while end < line.last && source.atoms[end].kind != Kind::Space {
                end += 1;
            }
            let mut fragment = Document::default();
            for atom in &source.atoms[index..end] {
                if atom.kind != Kind::Word {
                    fragment.append(std::slice::from_ref(atom));
                    continue;
                }
                let mut begin = 0;
                let mut pos = 0;
                while pos < atom.text.len() {
                    let next = character_end(&atom.text, pos, encoding)?;
                    if &atom.text[pos..next] == full_space {
                        fragment.push(Kind::Word, atom.slice(begin, pos));
                        fragment.push(Kind::Space, Cow::Borrowed(b" "));
                        begin = next;
                    }
                    pos = next;
                }
                fragment.push(Kind::Word, atom.slice(begin, pos));
            }
            out.append(&classify_fragment(fragment, encoding)?.atoms);
            index = end;
            copied = end;
        }
    }
    out.append(&source.atoms[copied..]);
    Ok(out)
}

// Reclassify only runs containing normalized words. Tokens wholly contained
// in an original slice keep borrowing it; only newly merged tokens own bytes.
fn classify_fragment<'a>(
    fragment: Document<'a>,
    encoding: ResolvedEncoding,
) -> Result<Document<'a>, Error> {
    let bytes = fragment.bytes();
    let mut out = Document::default();
    let mut atom_index = 0;
    let mut atom_begin = 0;
    // Positions inside a fragment are not input positions; report the message only.
    let tokens = lex_fragment(&bytes, encoding).map_err(without_location)?;
    for token in tokens {
        while atom_begin + fragment.atoms[atom_index].text.len() <= token.begin {
            atom_begin += fragment.atoms[atom_index].text.len();
            atom_index += 1;
        }
        let atom = &fragment.atoms[atom_index];
        let text = if token.end <= atom_begin + atom.text.len() {
            atom.slice(token.begin - atom_begin, token.end - atom_begin)
        } else {
            Cow::Owned(bytes[token.begin..token.end].to_vec())
        };
        out.push(token.kind, text);
    }
    Ok(out)
}

fn rewrite_comments<'a>(source: Document<'a>, options: &Options) -> Document<'a> {
    let lines = source_lines(&source);
    let tokens = &source.atoms;
    let text = |i: usize| source.text(i);
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
        &comment[if comment.starts_with(b";") { 1 } else { 2 }..]
    };
    let prefix: &[u8] = if options.comment_style == CommentStyle::C {
        b"//"
    } else {
        b";"
    };
    let mut out = Document::default();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if options.block_comments == BlockComments::Block
            && line_comment(line)
            && !body(line).windows(2).any(|w| w == b"*/" || w == b"/*")
        {
            out.append(&tokens[line.begin..line.first]);
            let mut comment = b"/*".to_vec();
            comment.extend_from_slice(body(line));
            while i + 1 < lines.len()
                && line_comment(&lines[i + 1])
                && !body(&lines[i + 1])
                    .windows(2)
                    .any(|w| w == b"*/" || w == b"/*")
            {
                if lines[i].end < tokens.len() {
                    comment.extend_from_slice(text(lines[i].end));
                }
                i += 1;
                comment.extend_from_slice(body(&lines[i]));
            }
            comment.extend_from_slice(b"*/");
            out.push(Kind::Comment, Cow::Owned(comment));
            out.append(&tokens[lines[i].first + 1..lines[i].end]);
            append_newline(&mut out, &source, &lines[i]);
        } else if options.block_comments == BlockComments::Lines
            && standalone(line)
            && text(line.first).starts_with(b"/*")
        {
            let atom = &tokens[line.first];
            let content = &atom.text[2..atom.text.len() - 2];
            let mut indent_begin = line.begin;
            if indent_begin < line.first && tokens[indent_begin].kind == Kind::Bom {
                out.append(&tokens[indent_begin..indent_begin + 1]);
                indent_begin += 1;
            }
            let mut pos = 0;
            loop {
                out.append(&tokens[indent_begin..line.first]);
                let end = content[pos..]
                    .iter()
                    .position(|&b| b == b'\r' || b == b'\n')
                    .map_or(content.len(), |n| pos + n);
                let comment = [prefix, &content[pos..end]].concat();
                out.push(Kind::Comment, Cow::Owned(comment));
                if end == content.len() {
                    break;
                }
                let next = end
                    + if content[end] == b'\r' && content.get(end + 1) == Some(&b'\n') {
                        2
                    } else {
                        1
                    };
                out.push(Kind::Newline, atom.slice(2 + end, 2 + next));
                pos = next;
                // Preserve the legacy trailing empty line comment as well.
                if pos == content.len() {
                    out.append(&tokens[indent_begin..line.first]);
                    out.push(Kind::Comment, Cow::Owned(prefix.to_vec()));
                    break;
                }
            }
            out.append(&tokens[line.first + 1..line.end]);
            append_newline(&mut out, &source, line);
        } else {
            for (index, atom) in tokens.iter().enumerate().take(line.end).skip(line.begin) {
                if !line.opaque
                    && !line.directive
                    && !line.multiline
                    && options.comment_style != CommentStyle::Preserve
                    && index >= line.first
                    && index < line.last
                    && atom.kind == Kind::Comment
                    && !atom.text.starts_with(b"/*")
                    && !atom.text.windows(7).any(|w| w == b"hspfmt:")
                {
                    let begin = if atom.text.starts_with(b";") { 1 } else { 2 };
                    out.push(
                        Kind::Comment,
                        Cow::Owned([prefix, &atom.text[begin..]].concat()),
                    );
                } else {
                    out.append(std::slice::from_ref(atom));
                }
            }
            append_newline(&mut out, &source, line);
        }
        i += 1;
    }
    out
}

// The final pass writes bytes directly instead of building another Document.
fn declaration_layout(source: &Document<'_>, options: &Options) -> Vec<u8> {
    let lines = source_lines(source);
    let tokens = &source.atoms;
    let mut out = Vec::with_capacity(source.byte_len());
    let mut copied = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.opaque || line.multiline || line.first + 1 >= line.last {
            continue;
        }
        let first = &tokens[line.first];
        let label = first.kind == Kind::Symbol
            && source.text(line.first) == b"*"
            && tokens[line.first + 1].kind == Kind::Word;
        if !line.directive && !label {
            continue;
        }
        let directive = if line.directive {
            lower(source.text(line.first + 1))
        } else {
            Vec::new()
        };
        let count = match directive.as_slice() {
            b"module" | b"chsp_module" => options.blank_lines_before_module,
            b"deffunc" | b"chsp_deffunc" => options.blank_lines_before_deffunc,
            b"defcfunc" | b"chsp_defcfunc" => options.blank_lines_before_defcfunc,
            b"modfunc" | b"modcfunc" | b"modinit" | b"modterm" => -1,
            _ if label => -1,
            _ => continue,
        };
        let mut start = i;
        while start > 0 {
            let previous = &lines[start - 1];
            if previous.opaque
                || previous.first + 1 != previous.last
                || tokens[previous.first].kind != Kind::Comment
                || source
                    .text(previous.first)
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
        extend_atoms(&mut out, &tokens[copied..lines[start].begin]);
        let at_start = count >= 0 && start == 0;
        let bom = tokens.first().map(|t| t.kind) == Some(Kind::Bom);
        if at_start && bom {
            extend_atoms(&mut out, &tokens[..1]);
        }
        let newline_index = if start > 0 {
            lines[start - 1].end
        } else {
            line.end
        };
        if !at_start && newline_index < tokens.len() {
            for _ in 0..count {
                extend_atoms(&mut out, &tokens[newline_index..newline_index + 1]);
            }
        }
        copied = lines[content].begin;
        if at_start && copied == 0 && bom {
            copied = 1;
        }
        if !options.preserve_indent {
            for comment in &lines[content..i] {
                if copied == 0 && bom {
                    extend_atoms(&mut out, &tokens[..1]);
                }
                extend_atoms(&mut out, &tokens[line.begin..line.first]);
                extend_atoms(&mut out, &tokens[comment.first..comment.end]);
                if comment.end < tokens.len() {
                    extend_atoms(&mut out, &tokens[comment.end..comment.end + 1]);
                }
                copied = (comment.end + 1).min(tokens.len());
            }
        }
    }
    extend_atoms(&mut out, &tokens[copied..]);
    out
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
    if !one_of(&type_name, &[b"array", b"local"])
        || pos == end
        || items[pos].text != b"[".as_slice()
    {
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
    while type_name == b"local" && pos < end && items[pos].text == b"[".as_slice() {
        dimensions += 1;
        if dimensions > 4
            || pos + 2 >= end
            || items[pos + 1].kind != Kind::Number
            || items[pos + 1].text.is_empty()
            || !items[pos + 1].text.iter().all(|&b| b.is_ascii_digit())
            || items[pos + 2].text != b"]".as_slice()
        {
            return begin;
        }
        pos += 3;
    }
    if pos == end || items[pos].text != b"]".as_slice() {
        return begin;
    }
    pos + 1
}

fn declaration_commas<'a>(items: &[Item<'a>], options: &Options) -> Option<Vec<Item<'a>>> {
    if options.comma_spacing == Spacing::Preserve || items.len() < 3 {
        return None;
    }
    let directive = lower(&items[1].text);
    let chsp = one_of(&directive, &[b"chsp_deffunc", b"chsp_defcfunc"]);
    if !chsp && !one_of(&directive, &[b"deffunc", b"defcfunc"]) {
        return None;
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
        return None;
    }
    pos += 1;
    let arrow = |i: usize| {
        chsp && i + 1 < last
            && items[i].text == b"-".as_slice()
            && items[i + 1].text == b">".as_slice()
    };
    while pos < last {
        if arrow(pos) {
            break;
        }
        let name = declaration_type_end(items, pos, last, chsp);
        if name == pos || name == last || items[name].kind != Kind::Word {
            return None;
        }
        pos = name + 1;
        if pos == last || arrow(pos) {
            break;
        }
        if items[pos].text != b",".as_slice() {
            return None;
        }
        pos += 1;
        if pos == last || arrow(pos) {
            return None;
        }
    }
    if chsp {
        if arrow(pos) {
            if pos + 3 != last || items[pos + 2].kind != Kind::Word {
                return None;
            }
            let result = lower(&items[pos + 2].text);
            if directive == b"chsp_defcfunc" {
                if !one_of(&result, &[b"int", b"int64", b"double", b"str"]) {
                    return None;
                }
            } else if result != b"void" {
                return None;
            }
        } else if directive == b"chsp_defcfunc" {
            return None;
        }
    }
    let mut result = items.to_vec();
    for i in 0..result.len() {
        if result[i].text == b",".as_slice() {
            result[i].gap = Cow::Borrowed(b"");
        } else if i > 0 && result[i - 1].text == b",".as_slice() {
            result[i].gap = Cow::Borrowed(if options.comma_spacing == Spacing::Space {
                b" "
            } else {
                b""
            });
        }
    }
    Some(result)
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
        if items[pos].text == b"@".as_slice() {
            let name = next_code(items, pos + 1, end);
            pos = if name < end && items[name].kind == Kind::Word {
                next_code(items, name + 1, end)
            } else {
                name
            };
        } else if items[pos].text == b".".as_slice() {
            pos = next_code(items, pos + 1, end);
            if pos == end {
                return begin;
            }
            if items[pos].kind == Kind::Word || items[pos].kind == Kind::Number {
                pos = next_code(items, pos + 1, end);
                continue;
            }
            if items[pos].text != b"(".as_slice() {
                return begin;
            }
        } else if items[pos].text != b"(".as_slice() {
            break;
        }
        if pos < end && items[pos].text == b"(".as_slice() {
            let mut depth = 0;
            loop {
                if items[pos].text == b"(".as_slice() {
                    depth += 1;
                } else if items[pos].text == b")".as_slice() {
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

fn operator_spelling<'a>(mut items: Vec<Item<'a>>, options: &Options) -> Vec<Item<'a>> {
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
                    Cow::Borrowed(b"+")
                } else {
                    Cow::Borrowed(b"-")
                }
            } else if plus {
                Cow::Borrowed(b"++")
            } else {
                Cow::Borrowed(b"--")
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
                    if text.as_ref() == b"&&".as_slice() {
                        *text = Cow::Borrowed(b"&");
                    } else if text.as_ref() == b"||".as_slice() {
                        *text = Cow::Borrowed(b"|");
                    } else if text.as_ref() == b"!=".as_slice() {
                        *text = Cow::Borrowed(b"!");
                    } else if text.as_ref() == b"==".as_slice() {
                        *text = Cow::Borrowed(b"=");
                    }
                } else if text.as_ref() == b"&".as_slice() {
                    *text = Cow::Borrowed(b"&&");
                } else if text.as_ref() == b"|".as_slice() {
                    *text = Cow::Borrowed(b"||");
                } else if text.as_ref() == b"!".as_slice() {
                    *text = Cow::Borrowed(b"!=");
                } else if text.as_ref() == b"=".as_slice() {
                    *text = Cow::Borrowed(b"==");
                }
            }
        }
        begin = if end == items.len() { end } else { end + 1 };
    }
    items
}

fn wraps_expression(items: &[Item], begin: usize, end: usize) -> bool {
    if end - begin < 2
        || items[begin].text != b"(".as_slice()
        || items[end - 1].text != b")".as_slice()
    {
        return false;
    }
    let mut depth = 0;
    for (i, item) in items.iter().enumerate().take(end).skip(begin) {
        if item.text == b"(".as_slice() {
            depth += 1;
        } else if item.text == b")".as_slice() {
            depth -= 1;
        }
        if depth <= 0 && i != end - 1 {
            return false;
        }
    }
    depth == 0
}

fn expression_parens<'a>(items: &[Item<'a>], options: &Options) -> Vec<Item<'a>> {
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
                if item.text == b"(".as_slice() || item.text == b"[".as_slice() {
                    stack.push(&item.text);
                } else if item.text == b")".as_slice() || item.text == b"]".as_slice() {
                    let expected = if item.text == b")".as_slice() {
                        b"(".as_slice()
                    } else {
                        b"[".as_slice()
                    };
                    if stack.is_empty() || stack.last().copied() != Some(expected) {
                        eligible = false;
                        break;
                    }
                    stack.pop();
                } else if item.text == b",".as_slice() && stack.is_empty() && !repeat {
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
                    if t < end
                        && (items[t].text == b"(".as_slice() || items[t].text == b"[".as_slice())
                    {
                        depth += 1;
                    }
                    if t < end
                        && (items[t].text == b")".as_slice() || items[t].text == b"]".as_slice())
                    {
                        depth -= 1;
                    }
                    if t != end && !(repeat && items[t].text == b",".as_slice() && depth == 0) {
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
                            text: Cow::Borrowed(b"("),
                            gap: Cow::Borrowed(b" ".as_slice()),
                        });
                    }
                    for (p, original) in items.iter().enumerate().take(last).skip(first) {
                        let mut item = original.clone();
                        if p == first {
                            item.gap = if add {
                                Cow::Borrowed(b"".as_slice())
                            } else {
                                Cow::Borrowed(b" ".as_slice())
                            };
                        }
                        result.push(item);
                    }
                    if add {
                        result.push(Item {
                            kind: Kind::Symbol,
                            text: Cow::Borrowed(b")"),
                            gap: Cow::Borrowed(b"".as_slice()),
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
                while pos < end && items[pos].text == b"@".as_slice() {
                    pos = next_code(items, pos + 1, end);
                    if pos < end && items[pos].kind == Kind::Word {
                        pos = next_code(items, pos + 1, end);
                    }
                }
                if pos + 1 < end
                    && items[pos].text == b"*".as_slice()
                    && (items[pos + 1].kind == Kind::Word || items[pos + 1].text == b"@".as_slice())
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

fn print_items<'a>(
    items: &[Item<'a>],
    options: &Options,
    encoding: ResolvedEncoding,
    ambiguous_spacing: Option<&mut bool>,
) -> Result<Document<'a>, Error> {
    let mut out = Document::default();
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
        let command =
            statement || jump || (previous_command && (text == b"@".as_slice() || prev == b"@"));
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
            } else if text.as_ref() == b":".as_slice() || prev == b":" {
                apply_spacing(options.colon_spacing);
            } else if text.as_ref() == b"{".as_slice()
                || prev == b"{"
                || text == b"}".as_slice()
                || prev == b"}"
            {
                space = true;
            } else if text.as_ref() == b",".as_slice() {
                preserve_gap = options.comma_spacing == Spacing::Preserve;
                space = false;
            } else if text.as_ref() == b")".as_slice() || text == b"]".as_slice() {
                space = false;
            } else if prev == b"," {
                apply_spacing(options.comma_spacing);
            } else if prev == b"(" || prev == b"[" || previous_prefix || postfix {
                space = false;
            } else if text.as_ref() == b"(".as_slice() || text == b"[".as_slice() {
                if binary(&lower(prev)) {
                    apply_spacing(options.operator_spacing);
                } else {
                    space = !item.gap.is_empty();
                }
            } else if text.as_ref() == b".".as_slice()
                || prev == b"."
                || text == b"@".as_slice()
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
                text = [b"$", &text[2..]].concat().into();
            } else if text.starts_with(b"0b") {
                text = [b"%", &text[2..]].concat().into();
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
            out.push(Kind::Space, item.gap.clone());
        } else if preserve_gap {
            out.push(Kind::Space, item.gap.clone());
        } else if space {
            out.push(Kind::Space, Cow::Borrowed(b" "));
        }

        out.push(item.kind, text.clone());
        previous_command = command && (item.kind == Kind::Word || text == b"@".as_slice());
        previous_prefix = prefix;
        if text.as_ref() == b"(".as_slice() || text == b"[".as_slice() {
            parens += 1;
            expect_operand = true;
        } else if text.as_ref() == b")".as_slice() || text == b"]".as_slice() {
            parens -= 1;
            expect_operand = false;
        } else if one_of(&text, &[b":", b"{", b"}"]) {
            statement = true;
            expect_operand = true;
        } else if text.as_ref() == b",".as_slice() || binary(&word) {
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

    let bytes = out.bytes();
    let printed_tokens = lex_resolved(&bytes, encoding).map_err(without_location)?;
    let mut index = 0;
    for token in &printed_tokens {
        if token.kind == Kind::Space {
            continue;
        }
        if index >= items.len() {
            return Err(Error::new(None, "spacing changed token boundaries"));
        }
        let mut expected = items[index].text.clone();
        if options.hsp_numeric_prefixes && items[index].kind == Kind::Number {
            if expected.starts_with(b"0x") {
                expected = [b"$", &expected[2..]].concat().into();
            } else if expected.starts_with(b"0b") {
                expected = [b"%", &expected[2..]].concat().into();
            }
        }
        if token.kind != items[index].kind || bytes[token.begin..token.end] != *expected {
            return Err(Error::new(None, "spacing changed token boundaries"));
        }
        index += 1;
    }
    if index != items.len() {
        return Err(Error::new(None, "spacing changed token boundaries"));
    }

    Ok(out)
}

fn followed_by_else(source: &Document<'_>, mut next: usize) -> bool {
    while next < source.atoms.len() {
        let token = &source.atoms[next];
        if matches!(
            token.kind,
            Kind::Space | Kind::Newline | Kind::Comment | Kind::Bom
        ) {
            next += 1;
            continue;
        }
        return token.kind == Kind::Word && lower(&token.text) == b"else";
    }
    false
}

fn short_if<'a>(
    items: Vec<Item<'a>>,
    options: &Options,
    encoding: ResolvedEncoding,
    indent_size: usize,
) -> Result<Vec<Item<'a>>, Error> {
    if !options.short_if || items.len() < 5 || lower(&items[0].text) != b"if" {
        return Ok(items);
    }
    for item in &items {
        if item.kind == Kind::Comment {
            return Ok(items);
        }
    }
    if items.last().unwrap().text != b"}".as_slice() {
        return Ok(items);
    }
    let mut open = 1;
    while open < items.len() && items[open].text != b"{".as_slice() {
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
    for i in (open + 1)..items.len() {
        if items[i].text == b"{".as_slice() {
            return Ok(items);
        }
        if items[i].text == b":".as_slice() || items[i].text == b"}".as_slice() {
            if items[i].text == b"}".as_slice() && i != items.len() - 1 {
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
    result[open].text = Cow::Borrowed(b":");
    if result[1].text == b"(".as_slice() && result[open - 1].text == b")".as_slice() {
        let mut nesting = 0;
        let mut wraps = true;
        for (i, item) in result.iter().enumerate().take(open).skip(1) {
            if item.text == b"(".as_slice() {
                nesting += 1;
            }
            if item.text == b")".as_slice() {
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
    let printed = print_items(&spelled, options, encoding, None)?;
    if printed.byte_len() + indent_size > options.line_width {
        return Ok(items);
    }
    Ok(result)
}

fn short_if_lines<'a>(
    source: Document<'a>,
    options: &Options,
    encoding: ResolvedEncoding,
) -> Result<Document<'a>, Error> {
    if !options.short_if {
        return Ok(source);
    }
    let lines = source_lines(&source);
    let text = |index: usize| source.text(index);
    let mut out = Document::default();
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
            || followed_by_else(&source, close.last)
        {
            i += 1;
            continue;
        }
        let mut items = Vec::new();
        for (j, line) in lines.iter().enumerate().skip(i).take(3) {
            let mut line_items = source.items(line.first, line.last);
            if let Some(first) = line_items.first_mut() {
                first.gap = Cow::Borrowed(if j == i { b"" } else { b" " });
            }
            items.extend(line_items);
        }
        let prefix = &source.atoms[head.begin..head.first];
        let indent_size = prefix
            .iter()
            .filter(|t| t.kind != Kind::Bom)
            .map(|t| t.text.len())
            .sum();
        let shortened = short_if(items.clone(), options, encoding, indent_size)?;
        if shortened.len() == items.len() {
            i += 1;
            continue;
        }
        out.append(&source.atoms[copied..head.begin]);
        out.append(prefix);
        let spelled = operator_spelling(expression_parens(&shortened, options), options);
        out.append(&print_items(&spelled, options, encoding, None)?.atoms);
        append_newline(&mut out, &source, close);
        copied = (close.end + 1).min(source.atoms.len());
        i += 3;
    }
    out.append(&source.atoms[copied..]);
    Ok(out)
}

// `'s` borrows pass inputs; `'a` is the original source shared by output atoms.
struct FormatContext<'s, 'a> {
    source: &'s Document<'a>,
    options: &'s Options,
    encoding: ResolvedEncoding,
    diagnostics: Option<&'s mut Vec<Diagnostic>>,
    original_lines: &'s [(&'s [u8], usize)],

    state: State,
    conditionals: Vec<Conditional>,
    items: Vec<Item<'a>>,
    out: Document<'a>,
    begin: usize,
    protected_line: bool,
    continuation: bool,
    line_number: usize,
    line_start: usize,
    disabled: bool,
    ignored: bool,
}

impl<'a> FormatContext<'_, 'a> {
    fn flush(
        &mut self,
        end: usize,
        newline: Option<&Atom<'a>>,
        next_token: usize,
    ) -> Result<(), Error> {
        let mut preserve =
            self.protected_line || self.continuation || self.disabled || self.ignored;
        let next_continuation =
            !self.items.is_empty() && self.items.last().unwrap().text == b"\\".as_slice();
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
        let label = self.items.len() >= 2
            && self.items[0].text == b"*".as_slice()
            && self.items[1].kind == Kind::Word;
        let directive = if self.items.len() > 1 && self.items[0].text == b"#".as_slice() {
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

        if (!opaque || continued_chsp)
            && !self.items.is_empty()
            && self.items[0].text == b"#".as_slice()
        {
            preserve = true;
            if one_of(&directive, &[b"if", b"ifdef", b"ifndef"]) {
                self.conditionals.push(Conditional {
                    before: self.state.clone(),
                    branch: None,
                    has_else: false,
                    line: self.line_start,
                });
            } else if directive == b"else" || directive == b"elif" {
                if self.conditionals.is_empty() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "unmatched preprocessor branch",
                    ));
                }
                let c = self.conditionals.last_mut().unwrap();
                if c.has_else {
                    return Err(Error::new(Some(self.line_start), "branch after #else"));
                }
                if c.branch
                    .as_ref()
                    .is_some_and(|branch| !branch.same_structure(&self.state))
                {
                    return Err(Error::new(
                        Some(self.line_start),
                        "conditional branches have different block structure",
                    ));
                }
                if let Some(branch) = &c.branch {
                    self.state.function |= branch.function;
                    self.state.label |= branch.label;
                }
                c.branch = Some(self.state.clone());
                c.has_else = directive == b"else";
                self.state = c.before.clone();
            } else if directive == b"endif" {
                if self.conditionals.is_empty() {
                    return Err(Error::new(Some(self.line_start), "unmatched #endif"));
                }
                let c = self.conditionals.pop().unwrap();
                if (c
                    .branch
                    .as_ref()
                    .is_some_and(|branch| !branch.same_structure(&self.state)))
                    || (!c.has_else && !c.before.same_structure(&self.state))
                {
                    return Err(Error::new(
                        Some(self.line_start),
                        "conditional branches have different block structure",
                    ));
                }
                if let Some(branch) = &c.branch {
                    self.state.function |= branch.function;
                    self.state.label |= branch.label;
                }
                if !c.has_else {
                    self.state.function |= c.before.function;
                    self.state.label |= c.before.label;
                }
            } else if directive == b"chsp_module" {
                if !self.state.blocks.is_empty() || self.state.chsp_module.is_some() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "nested cHSP module or module inside open block",
                    ));
                }
                self.state.chsp_module = Some(self.line_start);
                self.state.function = false;
                self.state.label = false;
            } else if one_of(&directive, &[b"chsp_deffunc", b"chsp_defcfunc"]) {
                if self.state.chsp_module.is_none() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "cHSP function outside #chsp_module",
                    ));
                }
                if !self.state.blocks.is_empty() || self.state.chsp_function.is_some() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "cHSP function inside open block or function",
                    ));
                }
                self.state.chsp_function = Some(self.line_start);
                self.state.function = true;
                self.state.label = false;
            } else if directive == b"chsp_end" {
                if self.state.chsp_function.is_none() {
                    return Err(Error::new(Some(self.line_start), "unmatched #chsp_end"));
                }
                if !self.state.blocks.is_empty() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "#chsp_end inside open block",
                    ));
                }
                self.state.chsp_function = None;
                self.state.function = false;
                self.state.label = false;
            } else if directive == b"chsp_module_end" {
                if self.state.chsp_module.is_none() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "unmatched #chsp_module_end",
                    ));
                }
                if !self.state.blocks.is_empty() || self.state.chsp_function.is_some() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "#chsp_module_end inside open block or function",
                    ));
                }
                self.state.chsp_module = None;
                self.state.function = false;
                self.state.label = false;
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
                if self.state.chsp_function.is_some() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "HSP function inside cHSP function",
                    ));
                }
                if !self.state.blocks.is_empty() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "function declaration inside open block",
                    ));
                }
                self.state.function = true;
                self.state.label = false;
            } else if directive == b"global" || directive == b"module" {
                if self.state.chsp_function.is_some() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "HSP module boundary inside cHSP function",
                    ));
                }
                if !self.state.blocks.is_empty() {
                    return Err(Error::new(
                        Some(self.line_start),
                        "module boundary inside open block",
                    ));
                }
                self.state.function = false;
                self.state.label = false;
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
            let indent_size = if self.options.preserve_indent {
                self.items.first().map_or(0, |item| item.gap.len())
            } else if self.options.tabs {
                depth
            } else {
                depth * self.options.indent_width
            };
            let following_else = self.options.short_if
                && !self.items.is_empty()
                && lower(&self.items[0].text) == b"if"
                && followed_by_else(self.source, next_token);
            let printed = if following_else {
                self.items.clone()
            } else {
                short_if(self.items.clone(), self.options, self.encoding, indent_size)?
            };
            let printed = expression_parens(&printed, self.options);
            let printed = operator_spelling(printed, self.options);
            if !self.items.is_empty() {
                if self.options.preserve_indent {
                    let prefix_end = self.begin
                        + self.source.atoms[self.begin..end]
                            .iter()
                            .take_while(|t| t.kind == Kind::Space)
                            .count();
                    self.out.append(&self.source.atoms[self.begin..prefix_end]);
                } else {
                    self.out
                        .push(Kind::Space, indentation(self.options.tabs, indent_size));
                }
                let mut ambiguous_spacing = false;
                let formatted_line = print_items(
                    &printed,
                    self.options,
                    self.encoding,
                    Some(&mut ambiguous_spacing),
                )?;
                self.out.append(&formatted_line.atoms);
                if ambiguous_spacing {
                    if let (Some(diag), Some(&(source, byte_offset))) = (
                        self.diagnostics.as_deref_mut(),
                        self.original_lines.get(self.line_number - 1),
                    ) {
                        diag.push(Diagnostic {
                            line: self.line_number,
                            source: source.to_vec(),
                            byte_offset,
                            kind: DiagnosticKind::AmbiguousLabelOrMultiplication,
                        });
                    }
                }
            }
        }
        if !opaque && label && self.options.indent_labels {
            self.state.label = true;
        }
        if !opaque && (self.items.is_empty() || self.items[0].text != b"#".as_slice()) {
            parse_line(&self.items, &mut self.state, self.line_start)?;
        }
        if preserve {
            if !opaque
                && !self.protected_line
                && !was_ignored
                && !self.items.is_empty()
                && self.items[0].text == b"#".as_slice()
            {
                if let Some(formatted_decl) = declaration_commas(&self.items, self.options) {
                    self.out.append_items(&formatted_decl);
                    let trailing = self.source.atoms[self.begin..end]
                        .iter()
                        .rposition(|t| t.kind != Kind::Space)
                        .map_or(self.begin, |n| self.begin + n + 1);
                    self.out.append(&self.source.atoms[trailing..end]);
                } else {
                    self.out.append(&self.source.atoms[self.begin..end]);
                }
            } else {
                self.out.append(&self.source.atoms[self.begin..end]);
            }
        }
        if let Some(newline) = newline {
            self.out.append(std::slice::from_ref(newline));
        }
        self.continuation = next_continuation;
        self.items.clear();
        self.protected_line = false;
        self.begin = end + usize::from(newline.is_some());
        Ok(())
    }
}

fn format_impl(
    source: &[u8],
    user_options: &Options,
    diagnostics: Option<&mut Vec<Diagnostic>>,
) -> Result<Vec<u8>, Error> {
    let encoding = resolve_encoding(source, user_options.encoding)?;
    let options = user_options;
    if options.indent_width > 16 {
        return Err(Error::new(None, "indent width must be between 0 and 16"));
    }
    if options.base_indent > 16 {
        return Err(Error::new(None, "base indent must be between 0 and 16"));
    }
    if options.loop_indent > 16 {
        return Err(Error::new(None, "loop indent must be between 0 and 16"));
    }
    for &count in &[
        options.blank_lines_before_module,
        options.blank_lines_before_deffunc,
        options.blank_lines_before_defcfunc,
    ] {
        if !(-1..=16).contains(&count) {
            return Err(Error::new(
                None,
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
            original_lines.push((&source[pos..end], pos));
            pos = end;
            if pos < source.len() && source[pos] == b'\r' {
                pos += 1;
            }
            if pos < source.len() && source[pos] == b'\n' {
                pos += 1;
            }
        }
    }

    let mut document = Document::parse(source, encoding)?;
    if options.full_width_spaces == FullWidthSpaces::Normalize {
        document = normalize_full_width_spaces(document, encoding)?;
    }
    if options.comment_style != CommentStyle::Preserve
        || options.block_comments != BlockComments::Preserve
    {
        document = rewrite_comments(document, options);
    }
    let mut ctx = FormatContext {
        source: &document,
        options,
        encoding,
        diagnostics,
        original_lines: &original_lines,
        state: State::default(),
        conditionals: Vec::new(),
        items: Vec::new(),
        out: Document::default(),
        begin: 0,
        protected_line: false,
        continuation: false,
        line_number: 1,
        line_start: 1,
        disabled: false,
        ignored: false,
    };
    for (index, atom) in document.atoms.iter().enumerate() {
        if atom.kind == Kind::Bom {
            ctx.out.append(std::slice::from_ref(atom));
            ctx.begin = index + 1;
        } else if atom.kind == Kind::Newline {
            ctx.items = document.items(ctx.begin, index);
            ctx.flush(index, Some(atom), index + 1)?;
        } else if atom.text.contains(&b'\r') || atom.text.contains(&b'\n') {
            ctx.protected_line = true;
        }
        let mut pos = 0;
        while pos < atom.text.len() {
            if atom.text[pos] == b'\r' {
                ctx.line_number += 1;
                if atom.text.get(pos + 1) == Some(&b'\n') {
                    pos += 1;
                }
            } else if atom.text[pos] == b'\n' {
                ctx.line_number += 1;
            }
            pos += 1;
        }
        if atom.kind == Kind::Newline {
            ctx.line_start = ctx.line_number;
        }
    }
    if ctx.begin < document.atoms.len() {
        ctx.items = document.items(ctx.begin, document.atoms.len());
        ctx.flush(document.atoms.len(), None, document.atoms.len())?;
    }

    if !ctx.conditionals.is_empty() {
        return Err(Error::new(
            Some(ctx.conditionals.last().unwrap().line),
            "unterminated preprocessor conditional",
        ));
    }
    if !ctx.state.blocks.is_empty() {
        let close_str = String::from_utf8_lossy(&ctx.state.blocks.last().unwrap().close);
        return Err(Error::new(
            Some(ctx.state.blocks.last().unwrap().line),
            format!("unterminated block, expected {close_str}"),
        ));
    }
    if ctx.state.chsp_function.is_some() {
        return Err(Error::new(
            ctx.state.chsp_function,
            "unterminated cHSP function, expected #chsp_end",
        ));
    }
    if ctx.state.chsp_module.is_some() {
        return Err(Error::new(
            ctx.state.chsp_module,
            "unterminated cHSP module, expected #chsp_module_end",
        ));
    }

    // Release the pre-pass document before the post passes build their copies.
    let out = ctx.out;
    drop(document);
    let out = short_if_lines(out, options, encoding)?;
    Ok(declaration_layout(&out, options))
}

/// Format lossless source bytes. Diagnostics are appended to the supplied vector.
/// No filesystem access, encoding conversion, or process output is performed.
pub fn format(
    source: &[u8],
    options: &Options,
    diagnostics: Option<&mut Vec<Diagnostic>>,
) -> Result<Vec<u8>, Error> {
    format_impl(source, options, diagnostics).map_err(|error| {
        if error.byte_offset().is_some() {
            return error;
        }
        let Some(line) = error.line() else {
            return error;
        };
        let mut current_line = 1;
        let mut offset = 0;
        while offset < source.len() && current_line < line {
            if source[offset] == b'\r' {
                current_line += 1;
                if source.get(offset + 1) == Some(&b'\n') {
                    offset += 1;
                }
            } else if source[offset] == b'\n' {
                current_line += 1;
            }
            offset += 1;
        }
        error.with_offset(offset)
    })
}

/// Format known UTF-8 text without ambiguous encoding detection.
/// This entry point overrides `options.encoding` with `Encoding::Utf8`.
pub fn format_utf8(
    source: &str,
    options: &Options,
    diagnostics: Option<&mut Vec<Diagnostic>>,
) -> Result<String, Error> {
    let mut options = options.clone();
    options.encoding = Encoding::Utf8;
    let bytes = format(source.as_bytes(), &options, diagnostics)?;
    String::from_utf8(bytes).map_err(|_| Error::new(None, "formatter produced invalid UTF-8"))
}
