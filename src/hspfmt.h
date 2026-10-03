#pragma once

#include <cstddef>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace hspfmt {

enum class Encoding { Auto, Utf8, Cp932 };
enum class Kind { Space, Newline, Word, Number, String, Comment, Symbol, Bom };
enum class CommentStyle { Preserve, Semicolon, C };
enum class BlockComments { Preserve, Lines, Block };
enum class Parentheses { Preserve, Add, Remove };
enum class OperatorStyle { Preserve, Hsp, C };
enum class FullWidthSpaces { Preserve, Normalize };
enum class Spacing { Preserve, Space, Compact };

// Every byte belongs to exactly one token, including trivia and the BOM.
struct Token {
    Kind kind;
    std::size_t begin;
    std::size_t end;
};

// An ambiguous label/multiplication gap was preserved instead of normalized.
struct Diagnostic {
    std::size_t line; // 1-based physical line in the input.
    std::string source; // Original line without its newline.
};

struct Options {
    Encoding encoding = Encoding::Auto;
    FullWidthSpaces full_width_spaces = FullWidthSpaces::Preserve;
    unsigned indent_width = 4;
    unsigned base_indent = 0;
    unsigned loop_indent = 1;
    bool preserve_indent = false;
    bool tabs = false;
    Spacing operator_spacing = Spacing::Space;
    Spacing comma_spacing = Spacing::Space;
    Spacing colon_spacing = Spacing::Space;
    Spacing comment_spacing = Spacing::Space;
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

// Input and syntax errors. line() is the 1-based physical input line, or 0
// when unknown; what() includes it as a "line N: " prefix.
class Error : public std::runtime_error {
public:
    Error(const std::string &message, std::size_t line)
        : std::runtime_error(line ? "line " + std::to_string(line) + ": " + message : message),
          message_(message), line_(line) {}
    const std::string &message() const { return message_; }
    std::size_t line() const { return line_; }
private:
    std::string message_;
    std::size_t line_;
};

// Detect whether source is UTF-8 or CP932 (returns Encoding::Utf8 or Encoding::Cp932).
Encoding detect_encoding(std::string_view source);
// Invalid/unterminated input throws Error. No source is modified.
std::vector<Token> lex(std::string_view source, Encoding encoding = Encoding::Auto);
// Diagnostics are appended when requested; the library does not write to stderr.
std::string format(std::string_view source, const Options &options = {},
                   std::vector<Diagnostic> *diagnostics = nullptr);

} // namespace hspfmt
