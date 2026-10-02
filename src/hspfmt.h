#pragma once

#include <cstddef>
#include <string>
#include <string_view>
#include <vector>

namespace hspfmt {

enum class Encoding { Utf8, Cp932 };
enum class Kind { Space, Newline, Word, Number, String, Comment, Symbol, Bom };
enum class CommentStyle { Preserve, Semicolon, C };
enum class BlockComments { Preserve, Lines, Block };
enum class Parentheses { Preserve, Add, Remove };
enum class OperatorStyle { Preserve, Hsp, C };

// Every byte belongs to exactly one token, including trivia and the BOM.
struct Token {
    Kind kind;
    std::size_t begin;
    std::size_t end;
};

struct Options {
    Encoding encoding = Encoding::Utf8;
    unsigned indent_width = 4;
    bool tabs = false;
    bool binary_spaces = true;
    bool hsp_numeric_prefixes = false;
    bool short_if = false;
    unsigned line_width = 100;
    bool indent_labels = false;
    CommentStyle comment_style = CommentStyle::Preserve;
    BlockComments block_comments = BlockComments::Preserve;
    Parentheses condition_parens = Parentheses::Preserve;
    Parentheses repeat_parens = Parentheses::Preserve;
    // -1 preserves existing blank lines; otherwise use exactly this count.
    int blank_lines_before_module = -1;
    int blank_lines_before_deffunc = -1;
    int blank_lines_before_defcfunc = -1;
    OperatorStyle operator_style = OperatorStyle::Preserve;
    OperatorStyle increment_style = OperatorStyle::Preserve;
};

// Invalid/unterminated input throws std::runtime_error. No source is modified.
std::vector<Token> lex(std::string_view source, Encoding encoding);
std::string format(std::string_view source, const Options &options = {});

} // namespace hspfmt
