use crate::*;
use std::path::{Path, PathBuf};

fn number(s: &str) -> Result<usize, String> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid numeric option: {}", s));
    }
    let significant = s.find(|c| c != '0');
    if let Some(pos) = significant {
        if s.len() - pos > 5 {
            return Err(format!("numeric option too large: {}", s));
        }
    }
    match s.parse::<usize>() {
        Ok(v) if v <= 10000 => Ok(v),
        _ => Err(format!("numeric option too large: {}", s)),
    }
}

fn parentheses(s: &str) -> Result<Parentheses, String> {
    match s {
        "preserve" => Ok(Parentheses::Preserve),
        "add" => Ok(Parentheses::Add),
        "remove" => Ok(Parentheses::Remove),
        _ => Err(format!("invalid parentheses mode: {}", s)),
    }
}

fn blank_lines(s: &str) -> Result<i32, String> {
    if s == "preserve" {
        return Ok(-1);
    }
    let value = number(s)?;
    if value > 16 {
        return Err("blank line count must be between 0 and 16".to_string());
    }
    Ok(value as i32)
}

fn operator_style(s: &str) -> Result<OperatorStyle, String> {
    match s {
        "preserve" => Ok(OperatorStyle::Preserve),
        "hsp" => Ok(OperatorStyle::Hsp),
        "c" => Ok(OperatorStyle::C),
        _ => Err(format!("invalid operator style: {}", s)),
    }
}

fn spacing(s: &str) -> Result<Spacing, String> {
    match s {
        "preserve" => Ok(Spacing::Preserve),
        "space" => Ok(Spacing::Space),
        "compact" => Ok(Spacing::Compact),
        _ => Err(format!("invalid spacing mode: {}", s)),
    }
}

/// Apply one CLI-style formatting option; return false for unrecognized options.
pub fn parse_formatting_option(arg: &str, options: &mut Options) -> Result<bool, String> {
    if arg == "--tabs" {
        options.tabs = true;
    } else if arg == "--compact-operators" {
        options.operator_spacing = Spacing::Compact;
    } else if let Some(v) = arg.strip_prefix("--operator-spacing=") {
        options.operator_spacing = spacing(v)?;
    } else if let Some(v) = arg.strip_prefix("--comma-spacing=") {
        options.comma_spacing = spacing(v)?;
    } else if let Some(v) = arg.strip_prefix("--colon-spacing=") {
        options.colon_spacing = spacing(v)?;
    } else if let Some(v) = arg.strip_prefix("--comment-spacing=") {
        options.comment_spacing = spacing(v)?;
    } else if arg == "--hsp-prefixes" {
        options.hsp_numeric_prefixes = true;
    } else if arg == "--short-if" {
        options.short_if = true;
    } else if let Some(value) = arg.strip_prefix("--full-width-spaces=") {
        match value {
            "preserve" => options.full_width_spaces = FullWidthSpaces::Preserve,
            "normalize" => options.full_width_spaces = FullWidthSpaces::Normalize,
            _ => return Err(format!("invalid full-width space mode: {}", value)),
        }
    } else if let Some(v) = arg.strip_prefix("--operator-style=") {
        options.operator_style = operator_style(v)?;
    } else if let Some(v) = arg.strip_prefix("--increment-style=") {
        options.increment_style = operator_style(v)?;
    } else if arg == "--indent-labels" {
        options.indent_labels = true;
    } else if arg == "--no-indent-labels" {
        options.indent_labels = false;
    } else if let Some(value) = arg.strip_prefix("--comment-style=") {
        match value {
            "preserve" => options.comment_style = CommentStyle::Preserve,
            "semicolon" => options.comment_style = CommentStyle::Semicolon,
            "c" => options.comment_style = CommentStyle::C,
            _ => return Err(format!("invalid comment style: {}", value)),
        }
    } else if let Some(value) = arg.strip_prefix("--block-comments=") {
        match value {
            "preserve" => options.block_comments = BlockComments::Preserve,
            "lines" => options.block_comments = BlockComments::Lines,
            "block" => options.block_comments = BlockComments::Block,
            _ => return Err(format!("invalid block comment mode: {}", value)),
        }
    } else if let Some(v) = arg.strip_prefix("--condition-parens=") {
        options.condition_parens = parentheses(v)?;
    } else if let Some(v) = arg.strip_prefix("--repeat-parens=") {
        options.repeat_parens = parentheses(v)?;
    } else if let Some(v) = arg.strip_prefix("--blank-lines-before-module=") {
        options.blank_lines_before_module = blank_lines(v)?;
    } else if let Some(v) = arg.strip_prefix("--blank-lines-before-deffunc=") {
        options.blank_lines_before_deffunc = blank_lines(v)?;
    } else if let Some(v) = arg.strip_prefix("--blank-lines-before-defcfunc=") {
        options.blank_lines_before_defcfunc = blank_lines(v)?;
    } else if arg == "--encoding=auto" {
        options.encoding = Encoding::Auto;
    } else if arg == "--encoding=cp932" {
        options.encoding = Encoding::Cp932;
    } else if arg == "--encoding=utf8" {
        options.encoding = Encoding::Utf8;
    } else if let Some(v) = arg.strip_prefix("--encoding=") {
        return Err(format!("invalid encoding: {}", v));
    } else if let Some(v) = arg.strip_prefix("--base-indent=") {
        options.base_indent = number(v)?;
    } else if let Some(v) = arg.strip_prefix("--loop-indent=") {
        options.loop_indent = number(v)?;
    } else if let Some(value) = arg.strip_prefix("--indent=") {
        options.preserve_indent = value == "preserve";
        if !options.preserve_indent {
            options.indent_width = number(value)?;
        }
    } else if let Some(v) = arg.strip_prefix("--line-width=") {
        options.line_width = number(v)?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

/// Apply ASCII config lines to options, reporting positions in the config bytes.
/// On error, `options` is left unchanged.
pub fn parse_config(content: &[u8], options: &mut Options) -> Result<(), Error> {
    let mut parsed = options.clone();
    let mut offset = 0;
    for (line_idx, line) in content.split(|&b| b == b'\n').enumerate() {
        let line_offset = offset;
        offset += line.len() + 1;
        // These ASCII separators cannot occur inside a CP932 multibyte character.
        let is_space = |b: &u8| matches!(b, b' ' | b'\t' | b'\r' | b'\n');
        let start = line.iter().position(|b| !is_space(b)).unwrap_or(line.len());
        let end = line
            .iter()
            .rposition(|b| !is_space(b))
            .map_or(start, |i| i + 1);
        let trimmed = &line[start..end];
        if trimmed.is_empty() || trimmed.starts_with(b"#") || trimmed.starts_with(b";") {
            continue;
        }
        if !trimmed.is_ascii() {
            return Err(
                Error::new(Some(line_idx + 1), "config options must be ASCII")
                    .with_offset(line_offset + start),
            );
        }
        // ASCII is valid UTF-8.
        let trimmed = std::str::from_utf8(trimmed).unwrap();
        let opt = if trimmed.starts_with("--") {
            trimmed.to_string()
        } else {
            format!("--{}", trimmed)
        };
        match parse_formatting_option(&opt, &mut parsed) {
            Ok(true) => {}
            _ => {
                return Err(Error::new(
                    Some(line_idx + 1),
                    format!("invalid or unsupported config option: {}", opt),
                )
                .with_offset(line_offset + start));
            }
        }
    }
    *options = parsed;
    Ok(())
}

/// Select an explicit config or the working directory's `.hspfmt`.
/// Existence is supplied by the caller, keeping library code free of filesystem I/O.
pub fn find_config(
    directory: &Path,
    explicit: Option<&Path>,
    disabled: bool,
    exists: impl FnOnce(&Path) -> bool,
) -> Option<PathBuf> {
    if disabled {
        return None;
    }
    if let Some(path) = explicit {
        return Some(path.to_path_buf());
    }
    let path = directory.join(".hspfmt");
    exists(&path).then_some(path)
}
