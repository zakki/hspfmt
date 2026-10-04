use crate::Encoding;

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
