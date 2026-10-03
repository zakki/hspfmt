#include "hspfmt.h"

#include <algorithm>
#include <stdexcept>
#include <utility>

namespace hspfmt {
namespace {

bool digit(char c) { return c >= '0' && c <= '9'; }
bool alpha(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_';
}
std::string lower(std::string_view s) {
    std::string out(s);
    for (char &c : out) if (c >= 'A' && c <= 'Z') c += 'a' - 'A';
    return out;
}
bool one_of(std::string_view s, std::initializer_list<std::string_view> values) {
    return std::find(values.begin(), values.end(), s) != values.end();
}

// Validate boundaries so a CP932 trail byte (notably 0x5c) is never syntax.
std::size_t character_end(std::string_view s, std::size_t i, Encoding encoding) {
    const auto c = static_cast<unsigned char>(s[i]);
    if (c < 128) return i + 1;
    if (encoding == Encoding::Cp932) {
        if (c >= 0xa1 && c <= 0xdf) return i + 1;
        if ((c >= 0x81 && c <= 0x9f) || (c >= 0xe0 && c <= 0xfc)) {
            if (i + 1 < s.size()) {
                const auto next = static_cast<unsigned char>(s[i + 1]);
                if (next >= 0x40 && next <= 0xfc && next != 0x7f) return i + 2;
            }
        }
        throw std::runtime_error("invalid CP932 byte sequence at byte " + std::to_string(i));
    }
    unsigned count = c >= 0xc2 && c <= 0xdf ? 2 :
                     c >= 0xe0 && c <= 0xef ? 3 :
                     c >= 0xf0 && c <= 0xf4 ? 4 : 0;
    if (!count || i + count > s.size())
        throw std::runtime_error("invalid UTF-8; use --encoding=cp932 for CP932 input");
    for (unsigned j = 1; j < count; ++j) {
        const auto b = static_cast<unsigned char>(s[i + j]);
        if (b < 0x80 || b > 0xbf) throw std::runtime_error("invalid UTF-8 continuation");
    }
    const auto second = static_cast<unsigned char>(s[i + 1]);
    if ((c == 0xe0 && second < 0xa0) || (c == 0xed && second >= 0xa0) ||
        (c == 0xf0 && second < 0x90) || (c == 0xf4 && second >= 0x90))
        throw std::runtime_error("invalid UTF-8 code point");
    return i + count;
}

struct Item {
    Kind kind;
    std::string text;
    std::string gap;
};

struct Frame {
    std::string close;
    bool case_body = false;
    bool operator==(const Frame &other) const {
        return close == other.close && case_body == other.case_body;
    }
};
struct State {
    std::vector<Frame> blocks;
    unsigned function = 0;
    unsigned label = 0;
    unsigned depth() const {
        unsigned n = std::max(function, label);
        for (const auto &frame : blocks) n += frame.case_body ? 2 : 1;
        return n;
    }
};
struct Conditional {
    State before;
    State branch;
    bool has_branch = false;
    bool has_else = false;
};

// A multiline token stays in one logical line. Never split strings or comments
// into apparent directives/code while applying layout rules.
struct SourceLine {
    std::size_t begin, end, first, last;
    std::string_view newline;
    bool opaque, directive, multiline;
};

std::vector<SourceLine> source_lines(std::string_view source, const std::vector<Token> &tokens) {
    std::vector<SourceLine> lines;
    std::size_t begin = 0, first = 0;
    bool disabled = false, continuation = false;
    auto append = [&](std::size_t end, std::size_t last, std::string_view newline) {
        auto start = first;
        while (start < last && (tokens[start].kind == Kind::Space || tokens[start].kind == Kind::Bom)) ++start;
        while (last > start && tokens[last - 1].kind == Kind::Space) --last;
        const auto text = [&](std::size_t i) { return source.substr(tokens[i].begin, tokens[i].end - tokens[i].begin); };
        const bool marker = start + 1 == last && tokens[start].kind == Kind::Comment &&
            one_of(text(start), {"; hspfmt: off", "; hspfmt: on"});
        const bool next = start < last && text(last - 1) == "\\";
        bool multiline = false;
        for (auto i = start; i < last; ++i)
            if (text(i).find_first_of("\r\n") != std::string_view::npos) multiline = true;
        lines.push_back({begin, end, start, last, newline, disabled || continuation || next || marker,
                         start < last && text(start) == "#", multiline});
        if (marker) disabled = text(start) == "; hspfmt: off";
        continuation = next;
    };
    for (std::size_t i = 0; i < tokens.size(); ++i) {
        if (tokens[i].kind != Kind::Newline) continue;
        append(tokens[i].begin, i, source.substr(tokens[i].begin, tokens[i].end - tokens[i].begin));
        begin = tokens[i].end;
        first = i + 1;
    }
    if (begin < source.size()) append(source.size(), tokens.size(), "");
    return lines;
}

std::string normalize_full_width_spaces(std::string_view source, Encoding encoding) {
    const auto tokens = lex(source, encoding);
    const auto lines = source_lines(source, tokens);
    const std::string_view full_space = encoding == Encoding::Utf8 ? "\xe3\x80\x80" : "\x81\x40";
    std::string out;
    std::size_t copied = 0;
    bool disabled = false;
    for (const auto &line : lines) {
        auto first = line.first;
        // Recognize standalone markers even when their indentation uses U+3000.
        while (first < line.last) {
            if (tokens[first].kind == Kind::Space) { ++first; continue; }
            if (tokens[first].kind != Kind::Word) break;
            const auto &token = tokens[first];
            auto text = source.substr(token.begin, token.end - token.begin);
            while (text.substr(0, full_space.size()) == full_space) text.remove_prefix(full_space.size());
            if (!text.empty()) break;
            ++first;
        }
        bool marker = false;
        bool next_disabled = disabled;
        if (first + 1 == line.last && tokens[first].kind == Kind::Comment) {
            const auto &token = tokens[first];
            const auto text = source.substr(token.begin, token.end - token.begin);
            marker = one_of(text, {"; hspfmt: off", "; hspfmt: on"});
            if (marker) next_disabled = text == "; hspfmt: off";
        }
        const bool preserve = disabled && !marker;
        disabled = next_disabled;
        if (preserve) continue;
        for (auto t = line.first; t < line.last; ++t) {
            const auto &token = tokens[t];
            // U+3000 outside strings/comments belongs to a Word in the lossless lexer.
            if (token.kind != Kind::Word) continue;
            out.append(source.substr(copied, token.begin - copied));
            for (auto pos = token.begin; pos < token.end;) {
                const auto end = character_end(source, pos, encoding);
                const auto character = source.substr(pos, end - pos);
                if (character == full_space) out += ' ';
                else out.append(character);
                pos = end;
            }
            copied = token.end;
        }
    }
    out.append(source.substr(copied));
    return out;
}

std::string rewrite_comments(std::string_view source, const Options &options) {
    if (options.comment_style == CommentStyle::Preserve && options.block_comments == BlockComments::Preserve)
        return std::string(source);
    const auto tokens = lex(source, options.encoding);
    const auto lines = source_lines(source, tokens);
    const auto text = [&](std::size_t i) { return source.substr(tokens[i].begin, tokens[i].end - tokens[i].begin); };
    const auto standalone = [&](const SourceLine &line) {
        return !line.opaque && !line.directive && line.first + 1 == line.last &&
            tokens[line.first].kind == Kind::Comment && text(line.first).find("hspfmt:") == std::string_view::npos;
    };
    const auto line_comment = [&](const SourceLine &line) {
        return standalone(line) && text(line.first).substr(0, 2) != "/*";
    };
    const auto body = [&](const SourceLine &line) {
        const auto comment = text(line.first);
        return comment.substr(comment[0] == ';' ? 1 : 2);
    };
    const std::string prefix = options.comment_style == CommentStyle::C ? "//" : ";";
    std::string out;
    for (std::size_t i = 0; i < lines.size(); ++i) {
        const auto &line = lines[i];
        if (options.block_comments == BlockComments::Block && line_comment(line) &&
            body(line).find("*/") == std::string_view::npos && body(line).find("/*") == std::string_view::npos) {
            out.append(source.substr(line.begin, tokens[line.first].begin - line.begin));
            out += "/*";
            out.append(body(line));
            while (i + 1 < lines.size() && line_comment(lines[i + 1]) &&
                   body(lines[i + 1]).find("*/") == std::string_view::npos &&
                   body(lines[i + 1]).find("/*") == std::string_view::npos) {
                out.append(lines[i].newline);
                out.append(body(lines[++i]));
            }
            out += "*/";
            out.append(source.substr(tokens[lines[i].first].end, lines[i].end - tokens[lines[i].first].end));
            out.append(lines[i].newline);
        } else if (options.block_comments == BlockComments::Lines && standalone(line) &&
                   text(line.first).substr(0, 2) == "/*") {
            const auto comment = text(line.first);
            auto content = comment.substr(2, comment.size() - 4);
            auto indent = source.substr(line.begin, tokens[line.first].begin - line.begin);
            if (indent.substr(0, 3) == "\xef\xbb\xbf") { out += "\xef\xbb\xbf"; indent.remove_prefix(3); }
            out.append(indent);
            out += prefix;
            for (std::size_t pos = 0; pos < content.size();) {
                const auto end = content.find_first_of("\r\n", pos);
                if (end == std::string_view::npos) { out.append(content.substr(pos)); break; }
                out.append(content.substr(pos, end - pos));
                auto next = end + 1;
                if (content[end] == '\r' && next < content.size() && content[next] == '\n') ++next;
                out.append(content.substr(end, next - end));
                out.append(indent);
                out += prefix;
                pos = next;
            }
            out.append(source.substr(tokens[line.first].end, line.end - tokens[line.first].end));
            out.append(line.newline);
        } else {
            std::size_t pos = line.begin;
            if (!line.opaque && !line.directive && !line.multiline && options.comment_style != CommentStyle::Preserve) {
                for (auto t = line.first; t < line.last; ++t) {
                    if (tokens[t].kind != Kind::Comment || text(t).substr(0, 2) == "/*" ||
                        text(t).find("hspfmt:") != std::string_view::npos) continue;
                    out.append(source.substr(pos, tokens[t].begin - pos));
                    out += prefix;
                    out.append(text(t).substr(text(t)[0] == ';' ? 1 : 2));
                    pos = tokens[t].end;
                }
            }
            out.append(source.substr(pos, line.end - pos));
            out.append(line.newline);
        }
    }
    return out;
}

std::string declaration_spacing(std::string_view source, const Options &options) {
    if (options.blank_lines_before_module < 0 && options.blank_lines_before_deffunc < 0 &&
        options.blank_lines_before_defcfunc < 0) return std::string(source);
    const auto tokens = lex(source, options.encoding);
    const auto lines = source_lines(source, tokens);
    std::string out;
    std::size_t copied = 0;
    for (std::size_t i = 0; i < lines.size(); ++i) {
        const auto &line = lines[i];
        if (line.opaque || line.multiline || !line.directive || line.first + 1 >= line.last) continue;
        const auto &token = tokens[line.first + 1];
        const auto directive = lower(source.substr(token.begin, token.end - token.begin));
        int count = -1;
        if (directive == "module") count = options.blank_lines_before_module;
        else if (directive == "deffunc") count = options.blank_lines_before_deffunc;
        else if (directive == "defcfunc") count = options.blank_lines_before_defcfunc;
        if (count < 0) continue;
        std::size_t start = i;
        while (start > 0) {
            const auto &previous = lines[start - 1];
            if (previous.opaque || previous.first + 1 != previous.last ||
                tokens[previous.first].kind != Kind::Comment ||
                source.substr(tokens[previous.first].begin, tokens[previous.first].end - tokens[previous.first].begin).find("hspfmt:") != std::string_view::npos) break;
            --start;
        }
        const auto content = start;
        while (start > 0 && !lines[start - 1].opaque && lines[start - 1].first == lines[start - 1].last) --start;
        if (lines[start].begin < copied) continue;
        out.append(source.substr(copied, lines[start].begin - copied));
        // No leading blank lines, including for a file starting with a BOM.
        const bool at_start = start == 0;
        if (at_start && source.substr(0, 3) == "\xef\xbb\xbf") out += "\xef\xbb\xbf";
        const auto newline = start > 0 ? lines[start - 1].newline : line.newline;
        if (!at_start) for (int n = 0; n < count; ++n) out.append(newline);
        copied = lines[content].begin;
        if (at_start && copied == 0 && source.substr(0, 3) == "\xef\xbb\xbf") copied = 3;
    }
    out.append(source.substr(copied));
    return out;
}
void close_block(State &state, std::string_view close) {
    if (state.blocks.empty() || state.blocks.back().close != close)
        throw std::runtime_error("unmatched block terminator: " + std::string(close));
    state.blocks.pop_back();
}

// Surface grammar: statement boundaries, braces, and standard macro blocks.
// Includes and macros are deliberately not expanded.
void parse_line(const std::vector<Item> &items, State &state) {
    bool statement = true;
    for (const auto &item : items) {
        if (item.kind == Kind::Comment) continue;
        const std::string word = lower(item.text);
        if (item.text == "{") {
            state.blocks.push_back({"}", false});
            statement = true;
        } else if (item.text == "}") {
            close_block(state, "}");
            statement = true;
        } else if (item.text == ":") {
            statement = true;
        } else {
            if (statement) {
                if (word == "repeat" || word == "foreach") state.blocks.push_back({"loop", false});
                else if (word == "while") state.blocks.push_back({"wend", false});
                else if (word == "for") state.blocks.push_back({"next", false});
                else if (word == "do") state.blocks.push_back({"until", false});
                else if (word == "switch") state.blocks.push_back({"swend", false});
                else if (one_of(word, {"loop", "wend", "next", "until", "swend"}))
                    close_block(state, word);
                else if (word == "case" || word == "default") {
                    if (state.blocks.empty() || state.blocks.back().close != "swend")
                        throw std::runtime_error("case/default outside switch");
                    state.blocks.back().case_body = true;
                }
            }
            statement = false;
        }
    }
}

bool binary(std::string_view s) {
    return one_of(s, {"=", "==", "!", "!=", "+", "-", "*", "/", "\\", "&", "|", "^",
                      "&&", "||", "<", ">", "<=", ">=", "<<", ">>", "+=", "-=", "*=",
                      "/=", "\\=", "&=", "|=", "^=", "and", "or", "xor"});
}

std::size_t next_code(const std::vector<Item> &items, std::size_t pos, std::size_t end) {
    while (pos < end && items[pos].kind == Kind::Comment) ++pos;
    return pos;
}

// Recognize only a variable-shaped statement head. This protects assignment
// operators and permits increments on array elements and qualified variables.
// A parenthesized argument to an unknown command is ambiguous and is preserved.
std::size_t variable_end(const std::vector<Item> &items, std::size_t begin, std::size_t end) {
    if (begin == end || items[begin].kind != Kind::Word ||
        one_of(lower(items[begin].text), {"if", "else", "while", "until", "repeat", "foreach", "for",
                                        "switch", "case", "return", "mes", "print", "logmes"})) return begin;
    auto pos = next_code(items, begin + 1, end);
    while (pos < end) {
        if (items[pos].text == "@") {
            const auto name = next_code(items, pos + 1, end);
            if (name == end || items[name].kind != Kind::Word) return begin;
            pos = next_code(items, name + 1, end);
        } else if (items[pos].text == ".") {
            pos = next_code(items, pos + 1, end);
            if (pos == end) return begin;
            if (items[pos].kind == Kind::Word || items[pos].kind == Kind::Number) {
                pos = next_code(items, pos + 1, end);
                continue;
            }
            if (items[pos].text != "(") return begin;
        } else if (items[pos].text != "(") break;
        if (pos < end && items[pos].text == "(") {
            int depth = 0;
            do {
                if (items[pos].text == "(") ++depth;
                else if (items[pos].text == ")") --depth;
                ++pos;
            } while (pos < end && depth > 0);
            if (depth != 0) return begin;
            pos = next_code(items, pos, end);
        }
    }
    return pos;
}

std::vector<Item> operator_spelling(std::vector<Item> items, const Options &options) {
    if (options.operator_style == OperatorStyle::Preserve && options.increment_style == OperatorStyle::Preserve)
        return items;
    for (std::size_t begin = 0; begin < items.size();) {
        auto end = begin;
        while (end < items.size() && !(items[end].kind == Kind::Symbol && one_of(items[end].text, {":", "{", "}"}))) ++end;
        const auto start = next_code(items, begin, end);
        const auto target = variable_end(items, start, end);
        // The first operator after a variable is a statement operator: x = ...,
        // x += ..., or the legacy x & ... form. Expressions inside the variable's
        // subscripts and to the right of this operator remain eligible.
        const bool variable_statement = target > start && target < end &&
            (binary(items[target].text) || one_of(items[target].text, {"++", "--"}));
        if (variable_statement && options.increment_style != OperatorStyle::Preserve &&
            one_of(items[target].text, {"+", "-", "++", "--"}) && next_code(items, target + 1, end) == end) {
            const bool plus = items[target].text[0] == '+';
            items[target].text = options.increment_style == OperatorStyle::Hsp ? (plus ? "+" : "-") : (plus ? "++" : "--");
        }
        if (options.operator_style != OperatorStyle::Preserve) {
            for (auto i = start; i < end; ++i) {
                if (items[i].kind != Kind::Symbol || (variable_statement && i == target)) continue;
                auto left = i;
                while (left > start && items[left - 1].kind == Kind::Comment) --left;
                const auto right = next_code(items, i + 1, end);
                if (left == start || right == end || left - 1 == start) continue;
                const auto &before = items[left - 1];
                const auto &after = items[right];
                const bool operand_before = (before.kind == Kind::Word && !binary(lower(before.text))) ||
                    before.kind == Kind::Number || before.kind == Kind::String || one_of(before.text, {")", "]"});
                const bool operand_after = (after.kind == Kind::Word && !binary(lower(after.text))) ||
                    after.kind == Kind::Number || after.kind == Kind::String || one_of(after.text, {"(", "[", "-", "+", "*"});
                if (!operand_before || !operand_after) continue;
                auto &text = items[i].text;
                if (options.operator_style == OperatorStyle::Hsp) {
                    if (text == "&&") text = "&";
                    else if (text == "||") text = "|";
                    else if (text == "!=") text = "!";
                    else if (text == "==") text = "=";
                } else {
                    if (text == "&") text = "&&";
                    else if (text == "|") text = "||";
                    else if (text == "!") text = "!=";
                    else if (text == "=") text = "==";
                }
            }
        }
        begin = end == items.size() ? end : end + 1;
    }
    return items;
}

bool wraps_expression(const std::vector<Item> &items, std::size_t begin, std::size_t end) {
    if (end - begin < 2 || items[begin].text != "(" || items[end - 1].text != ")") return false;
    int depth = 0;
    for (auto i = begin; i < end; ++i) {
        if (items[i].text == "(") ++depth;
        else if (items[i].text == ")") --depth;
        if (depth <= 0 && i != end - 1) return false;
    }
    return depth == 0;
}

std::vector<Item> expression_parens(const std::vector<Item> &items, const Options &options) {
    if (options.condition_parens == Parentheses::Preserve && options.repeat_parens == Parentheses::Preserve)
        return items;
    std::vector<Item> result;
    bool statement = true;
    for (std::size_t i = 0; i < items.size();) {
        const auto word = lower(items[i].text);
        const bool condition = statement && items[i].kind == Kind::Word && (word == "if" || word == "while");
        const bool repeat = statement && items[i].kind == Kind::Word && word == "repeat";
        const auto mode = condition ? options.condition_parens : repeat ? options.repeat_parens : Parentheses::Preserve;
        result.push_back(items[i++]);
        if (mode != Parentheses::Preserve) {
            // Only balanced, uninterrupted expressions are eligible. A top-level
            // comma separates repeat arguments, never their enclosing pair.
            auto end = i;
            std::vector<std::string> stack;
            bool eligible = true;
            for (; end < items.size(); ++end) {
                const auto &item = items[end];
                if (item.kind == Kind::Comment || one_of(item.text, {":", "{", "}"})) break;
                if (item.text == "(" || item.text == "[") stack.push_back(item.text);
                else if (item.text == ")" || item.text == "]") {
                    if (stack.empty() || stack.back() != (item.text == ")" ? "(" : "[")) { eligible = false; break; }
                    stack.pop_back();
                } else if (item.text == "," && stack.empty() && !repeat) eligible = false;
            }
            if (!stack.empty()) eligible = false;
            if (end < items.size() && items[end].kind == Kind::Comment && items[end].text.substr(0, 2) == "/*")
                eligible = false;
            if (eligible) {
                auto start = i;
                int depth = 0;
                for (auto t = i; t <= end; ++t) {
                    if (t < end && (items[t].text == "(" || items[t].text == "[")) ++depth;
                    if (t < end && (items[t].text == ")" || items[t].text == "]")) --depth;
                    if (t != end && !(repeat && items[t].text == "," && depth == 0)) continue;
                    auto first = start, last = t;
                    if (mode == Parentheses::Remove)
                        while (wraps_expression(items, first, last)) { ++first; --last; }
                    const bool add = first < last && mode == Parentheses::Add && !wraps_expression(items, first, last);
                    if (add) result.push_back({Kind::Symbol, "(", " "});
                    for (auto p = first; p < last; ++p) {
                        auto item = items[p];
                        if (p == first) item.gap = add ? "" : " ";
                        result.push_back(std::move(item));
                    }
                    if (add) result.push_back({Kind::Symbol, ")", ""});
                    if (t < end) result.push_back(items[t]);
                    start = t + 1;
                }
                i = end;
            }
        }
        const auto &previous = items[i - 1];
        if (one_of(previous.text, {":", "{", "}"})) statement = true;
        else if (previous.kind != Kind::Comment) statement = word == "else";
    }
    return result;
}

// At a bare statement head, name *value can be a command taking a label or
// a variable's legacy multiplication assignment. Do not guess the name's role.
std::vector<bool> ambiguous_label_gaps(const std::vector<Item> &items) {
    std::vector<bool> gaps(items.size(), false);
    for (std::size_t begin = 0; begin < items.size();) {
        auto end = begin;
        while (end < items.size() && !one_of(items[end].text, {":", "{", "}"})) ++end;
        const auto start = next_code(items, begin, end);
        if (start < end && !one_of(lower(items[start].text),
                {"goto", "gosub", "onclick", "onkey", "onexit", "onerror", "oncmd", "button"})) {
            auto pos = start + 1;
            if (items[start].kind == Kind::Word && variable_end(items, start, end) != start) {
                pos = next_code(items, pos, end);
                while (pos + 1 < end && items[pos].text == "@" && items[pos + 1].kind == Kind::Word)
                    pos = next_code(items, pos + 2, end);
                if (pos + 1 < end && items[pos].text == "*" &&
                    (items[pos + 1].kind == Kind::Word || items[pos + 1].text == "@"))
                    gaps[pos] = gaps[pos + 1] = true;
            }
        }
        begin = end == items.size() ? end : end + 1;
    }
    return gaps;
}

std::string print_items(const std::vector<Item> &items, const Options &options,
                        bool *ambiguous_spacing = nullptr) {
    std::string out;
    const auto preserved_gaps = ambiguous_label_gaps(items);
    bool previous_prefix = false;
    bool expect_operand = true;
    bool statement = true;
    int parens = 0;
    bool previous_command = false;
    for (std::size_t i = 0; i < items.size(); ++i) {
        const auto &item = items[i];
        std::string text = item.text;
        const std::string word = lower(text);
        const std::string prev = i ? items[i - 1].text : "";
        const std::string next = i + 1 < items.size() ? items[i + 1].text : "";
        // goto/gosub also introduce label operands inside event and on commands.
        const bool jump = parens == 0 && item.kind == Kind::Word && one_of(word, {"goto", "gosub"});
        // A module-qualified command name continues across its @module suffix.
        const bool command = statement || jump || (previous_command && (text == "@" || prev == "@"));
        const bool prefix = expect_operand && one_of(text, {"-", "+", "*"});
        const bool postfix = one_of(text, {"+", "-", "++", "--"}) && i > 0 &&
            (next.empty() || next == ":" || next == "}" || items[i + 1].kind == Kind::Comment);
        bool space = i != 0;
        if (i) {
            if (item.kind == Kind::Comment) space = true;
            else if (text == ":" || prev == ":" || text == "{" || prev == "{" || text == "}" || prev == "}") space = true;
            else if (text == "," || text == ")" || text == "]") space = false;
            else if (prev == ",") space = true;
            else if (prev == "(" || prev == "[" || previous_prefix || postfix) space = false;
            else if (text == "(" || text == "[") space = !item.gap.empty();
            else if (text == "." || prev == "." || text == "@" || prev == "@") space = false;
            else if (binary(word) || binary(lower(prev))) space = options.binary_spaces;
            // Do not merge word operators with operands when compact spacing is requested.
            if ((item.kind == Kind::Word && items[i - 1].kind == Kind::Word) ||
                (item.kind == Kind::Number && items[i - 1].kind == Kind::Word) ||
                (item.kind == Kind::Word && items[i - 1].kind == Kind::Number)) space = true;
            // First command argument must remain separated, even for unary signs/labels.
            if (previous_command && !item.gap.empty() && text != "=" &&
                !postfix && !one_of(text, {"(", "[", ".", "@"})) space = true;
            // Compact spacing must not create ++, --, /*, //, <=, etc.
            if (!space && !prev.empty() && !text.empty() &&
                one_of(prev.substr(prev.size() - 1) + text.substr(0, 1),
                       {"++", "--", "/*", "//", "==", "!=", "<=", ">=", "<<", ">>",
                        "&&", "||", "+=", "-=", "*=", "/=", "\\=", "&=", "|=", "^="})) space = true;
            if (item.kind == Kind::Word && one_of(word, {"and", "or", "xor"})) space = true;
            if (one_of(lower(prev), {"and", "or", "xor"})) space = true;
        }
        if (options.hsp_numeric_prefixes && item.kind == Kind::Number && text.size() > 2) {
            if (text.substr(0, 2) == "0x") text.replace(0, 2, "$");
            else if (text.substr(0, 2) == "0b") text.replace(0, 2, "%");
        }
        if (preserved_gaps[i]) {
            const std::string desired = options.binary_spaces ? " " : "";
            if (ambiguous_spacing && item.gap != desired) *ambiguous_spacing = true;
            out += item.gap;
        } else if (space) out += ' ';
        out += text;
        previous_command = command && (item.kind == Kind::Word || text == "@");
        previous_prefix = prefix;
        if (text == "(" || text == "[") { ++parens; expect_operand = true; }
        else if (text == ")" || text == "]") { --parens; expect_operand = false; }
        else if (one_of(text, {":", "{", "}"})) { statement = true; expect_operand = true; }
        else if (text == "," || binary(word)) { expect_operand = true; statement = false; }
        else if (item.kind != Kind::Comment) {
            // At statement start a word can be a command; its first argument is an operand.
            expect_operand = command && item.kind == Kind::Word && parens == 0;
            statement = false;
        }
    }
    // A spacing choice must never merge/split tokens. This also guards syntax
    // outside the small surface grammar (for example legacy dot subscripts).
    const auto printed_tokens = lex(out, options.encoding);
    std::size_t index = 0;
    for (const auto &token : printed_tokens) {
        if (token.kind == Kind::Space) continue;
        if (index >= items.size()) throw std::runtime_error("spacing changed token boundaries");
        std::string expected = items[index].text;
        if (options.hsp_numeric_prefixes && items[index].kind == Kind::Number) {
            if (expected.substr(0, 2) == "0x") expected.replace(0, 2, "$");
            if (expected.substr(0, 2) == "0b") expected.replace(0, 2, "%");
        }
        if (token.kind != items[index].kind || out.substr(token.begin, token.end - token.begin) != expected)
            throw std::runtime_error("spacing changed token boundaries");
        ++index;
    }
    if (index != items.size()) throw std::runtime_error("spacing changed token boundaries");
    return out;
}

// Only a complete, isolated one-line if with assignment statements is eligible.
// No nested control flow, trailing statements, comments, or directive rewriting.
std::vector<Item> short_if(std::vector<Item> items, const Options &options, unsigned depth) {
    if (!options.short_if || items.size() < 7 || lower(items[0].text) != "if") return items;
    for (const auto &item : items) if (item.kind == Kind::Comment) return items;
    if (items.back().text != "}") return items;
    std::size_t open = 1;
    while (open < items.size() && items[open].text != "{") ++open;
    if (open < 2 || open == items.size()) return items;
    for (std::size_t i = 1; i < open; ++i)
        if (one_of(items[i].text, {":", "}"})) return items;
    std::size_t start = open + 1;
    for (std::size_t i = start; i < items.size(); ++i) {
        if (items[i].text == "{") return items;
        if (items[i].text == ":" || items[i].text == "}") {
            if (items[i].text == "}" && i != items.size() - 1) return items;
            if (i < start + 3 || items[start].kind != Kind::Word || items[start + 1].text != "=") return items;
            start = i + 1;
        }
    }
    auto result = items;
    result.pop_back();
    result[open].text = ":";
    if (result[1].text == "(" && result[open - 1].text == ")") {
        int nesting = 0;
        bool wraps = true;
        for (std::size_t i = 1; i < open; ++i) {
            if (result[i].text == "(") ++nesting;
            if (result[i].text == ")") --nesting;
            if (nesting == 0 && i != open - 1) wraps = false;
        }
        if (wraps && nesting == 0) {
            result.erase(result.begin() + open - 1);
            result.erase(result.begin() + 1);
        }
    }
    if (print_items(operator_spelling(expression_parens(result, options), options), options).size() + depth * options.indent_width > options.line_width) return items;
    return result;
}

} // namespace

std::vector<Token> lex(std::string_view s, Encoding encoding) {
    std::vector<Token> tokens;
    std::size_t i = 0;
    while (i < s.size()) {
        const std::size_t begin = i;
        Kind kind = Kind::Symbol;
        if (i == 0 && s.substr(0, 3) == "\xef\xbb\xbf") { kind = Kind::Bom; i += 3; }
        else if (s[i] == ' ' || s[i] == '\t') {
            kind = Kind::Space;
            while (i < s.size() && (s[i] == ' ' || s[i] == '\t')) ++i;
        } else if (s[i] == '\r' || s[i] == '\n') {
            kind = Kind::Newline;
            if (s[i++] == '\r' && i < s.size() && s[i] == '\n') ++i;
        } else if (s[i] == ';' || s.substr(i, 2) == "//") {
            kind = Kind::Comment;
            while (i < s.size() && s[i] != '\r' && s[i] != '\n') i = character_end(s, i, encoding);
        } else if (s.substr(i, 2) == "/*") {
            kind = Kind::Comment;
            i += 2;
            while (i < s.size() && s.substr(i, 2) != "*/") i = character_end(s, i, encoding);
            if (i == s.size()) throw std::runtime_error("unterminated block comment");
            i += 2;
        } else if (s[i] == '"' || s[i] == '\'' || s.substr(i, 2) == "{\"") {
            kind = Kind::String;
            const bool multiline = s[i] == '{';
            const char quote = multiline ? '"' : s[i];
            i += multiline ? 2 : 1;
            bool closed = false;
            while (i < s.size()) {
                if (s[i] == quote && (!multiline || s.substr(i, 2) == "\"}")) {
                    i += multiline ? 2 : 1;
                    closed = true;
                    break;
                }
                if (s[i] == '\\') {
                    ++i;
                    if (i < s.size()) i = character_end(s, i, encoding);
                } else {
                    if (!multiline && (s[i] == '\r' || s[i] == '\n'))
                        throw std::runtime_error("newline in quoted string");
                    i = character_end(s, i, encoding);
                }
            }
            if (!closed) throw std::runtime_error("unterminated string");
        } else if (digit(s[i]) || s[i] == '$' || (s[i] == '%' && i + 1 < s.size() &&
                   (digit(s[i + 1]) || s[i + 1] == '_'))) {
            kind = Kind::Number;
            const bool hex = s[i] == '$' || s.substr(i, 2) == "0x";
            const bool bin = s[i] == '%' || s.substr(i, 2) == "0b";
            i += (s.substr(i, 2) == "0x" || s.substr(i, 2) == "0b") ? 2 : 1;
            while (i < s.size()) {
                const char c = s[i];
                if (digit(c) || c == '_' || (hex && ((c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')))) ++i;
                else if (!hex && !bin && c == '.' && i + 1 < s.size() && digit(s[i + 1])) ++i;
                else if (!hex && !bin && (c == 'e' || c == 'E')) {
                    ++i;
                    if (i < s.size() && (s[i] == '+' || s[i] == '-')) ++i;
                } else break;
            }
            if (i < s.size() && (s[i] == 'l' || s[i] == 'L' || s[i] == 'f' || s[i] == 'F' ||
                                (!hex && !bin && (s[i] == 'd' || s[i] == 'D')))) ++i;
        } else if (alpha(s[i]) || static_cast<unsigned char>(s[i]) >= 128) {
            kind = Kind::Word;
            do { i = character_end(s, i, encoding); }
            while (i < s.size() && (alpha(s[i]) || digit(s[i]) || static_cast<unsigned char>(s[i]) >= 128));
        } else {
            if (s[i] == '\0') throw std::runtime_error("NUL byte in source");
            ++i;
            if (i < s.size() && one_of(s.substr(begin, 2), {"==", "!=", "<=", ">=", "<<", ">>", "&&", "||",
                  "++", "--", "+=", "-=", "*=", "/=", "\\=", "&=", "|=", "^="})) ++i;
        }
        tokens.push_back({kind, begin, i});
    }
    return tokens;
}

std::string format(std::string_view source, const Options &options,
                   std::vector<Diagnostic> *diagnostics) {
    if (options.indent_width > 16) throw std::runtime_error("indent width must be between 0 and 16");
    for (const auto count : {options.blank_lines_before_module, options.blank_lines_before_deffunc,
                             options.blank_lines_before_defcfunc})
        if (count < -1 || count > 16) throw std::runtime_error("blank line count must be between 0 and 16, or -1 to preserve");
    std::vector<std::string_view> original_lines;
    if (diagnostics) {
        for (std::size_t pos = 0; pos < source.size();) {
            auto end = source.find_first_of("\r\n", pos);
            if (end == std::string_view::npos) end = source.size();
            original_lines.push_back(source.substr(pos, end - pos));
            pos = end;
            if (pos < source.size() && source[pos++] == '\r' && pos < source.size() && source[pos] == '\n') ++pos;
        }
    }
    std::string normalized;
    if (options.full_width_spaces == FullWidthSpaces::Normalize) {
        normalized = normalize_full_width_spaces(source, options.encoding);
        source = normalized;
    }
    std::string rewritten;
    if (options.comment_style != CommentStyle::Preserve || options.block_comments != BlockComments::Preserve) {
        rewritten = rewrite_comments(source, options);
        source = rewritten;
    }
    const auto tokens = lex(source, options.encoding);
    // An else on the following line must keep its matching if in brace form.
    const auto followed_by_else = [&](std::size_t next) {
        for (; next < tokens.size(); ++next) {
            const auto &token = tokens[next];
            if (token.kind == Kind::Space || token.kind == Kind::Newline ||
                token.kind == Kind::Comment || token.kind == Kind::Bom) continue;
            return token.kind == Kind::Word &&
                lower(source.substr(token.begin, token.end - token.begin)) == "else";
        }
        return false;
    };
    State state;
    std::vector<Conditional> conditionals;
    std::vector<Item> items;
    std::string out;
    std::size_t begin = 0;
    bool protected_line = false;
    bool continuation = false;
    std::string gap;
    std::size_t line_number = 1;
    bool disabled = false;
    auto flush = [&](std::size_t end, std::string_view newline, std::size_t next_token) {
        const std::string_view raw = source.substr(begin, end - begin);
        bool preserve = protected_line || continuation || disabled;
        const bool next_continuation = !items.empty() && items.back().text == "\\";
        if (next_continuation) preserve = true;
        const bool was_disabled = disabled;
        if (items.size() == 1 && items[0].kind == Kind::Comment) {
            if (items[0].text == "; hspfmt: off") { disabled = true; preserve = true; }
            if (items[0].text == "; hspfmt: on") { disabled = false; preserve = true; }
        }
        const bool opaque = continuation || next_continuation || was_disabled || disabled;
        const bool label = items.size() >= 2 && items[0].text == "*" && items[1].kind == Kind::Word;
        if (!opaque && !items.empty() && items[0].text == "#") {
            preserve = true;
            const std::string directive = items.size() > 1 ? lower(items[1].text) : "";
            if (one_of(directive, {"if", "ifdef", "ifndef"})) {
                conditionals.push_back({state, {}, false, false});
            } else if (directive == "else" || directive == "elif") {
                if (conditionals.empty()) throw std::runtime_error("unmatched preprocessor branch");
                auto &c = conditionals.back();
                if (c.has_else) throw std::runtime_error("branch after #else");
                if (c.has_branch && c.branch.blocks != state.blocks) throw std::runtime_error("conditional branches have different block structure");
                if (c.has_branch) state.function = std::max(state.function, c.branch.function);
                if (c.has_branch) state.label = std::max(state.label, c.branch.label);
                c.branch = state;
                c.has_branch = true;
                c.has_else = directive == "else";
                state = c.before;
            } else if (directive == "endif") {
                if (conditionals.empty()) throw std::runtime_error("unmatched #endif");
                const auto c = conditionals.back();
                conditionals.pop_back();
                if ((c.has_branch && c.branch.blocks != state.blocks) || (!c.has_else && c.before.blocks != state.blocks))
                    throw std::runtime_error("conditional branches have different block structure");
                if (c.has_branch) state.function = std::max(state.function, c.branch.function);
                if (!c.has_else) state.function = std::max(state.function, c.before.function);
                if (c.has_branch) state.label = std::max(state.label, c.branch.label);
                if (!c.has_else) state.label = std::max(state.label, c.before.label);
            } else if (one_of(directive, {"deffunc", "defcfunc", "modfunc", "modcfunc", "modinit", "modterm"})) {
                if (!state.blocks.empty()) throw std::runtime_error("function declaration inside open block");
                state.function = 1;
                state.label = 0;
            } else if (directive == "global" || directive == "module") {
                if (!state.blocks.empty()) throw std::runtime_error("module boundary inside open block");
                state.function = 0;
                state.label = 0;
            }
        } else if (!preserve) {
            State line_state = state;
            if (!items.empty()) {
                const auto first = lower(items[0].text);
                if (one_of(first, {"}", "loop", "wend", "next", "until", "swend"})) close_block(line_state, first);
                else if ((first == "case" || first == "default") && !line_state.blocks.empty())
                    line_state.blocks.back().case_body = false;
            }
            const unsigned depth = label ? 0 : line_state.depth();
            const bool following_else = options.short_if && !items.empty() &&
                lower(items[0].text) == "if" && followed_by_else(next_token);
            auto printed = following_else ? items : short_if(items, options, depth);
            printed = expression_parens(printed, options);
            printed = operator_spelling(std::move(printed), options);
            if (!items.empty()) {
                out.append(options.tabs ? depth : depth * options.indent_width, options.tabs ? '\t' : ' ');
                bool ambiguous_spacing = false;
                out += print_items(printed, options, &ambiguous_spacing);
                if (ambiguous_spacing && diagnostics)
                    diagnostics->push_back({line_number, std::string(original_lines.at(line_number - 1))});
            }
        }
        if (!opaque && label && options.indent_labels) state.label = 1;
        if (!opaque && (items.empty() || items[0].text != "#")) parse_line(items, state);
        if (preserve) out.append(raw);
        out.append(newline);
        continuation = next_continuation;
        items.clear();
        protected_line = false;
        gap.clear();
        begin = end + newline.size();
    };
    for (std::size_t token_index = 0; token_index < tokens.size(); ++token_index) {
        const auto &token = tokens[token_index];
        const auto text = source.substr(token.begin, token.end - token.begin);
        if (token.kind == Kind::Bom) { out.append(text); begin = token.end; }
        else if (token.kind == Kind::Newline) flush(token.begin, text, token_index + 1);
        else if (token.kind == Kind::Space) gap.append(text);
        else {
            // Multiline tokens are copied together with their entire surrounding line.
            if (text.find_first_of("\r\n") != std::string_view::npos) protected_line = true;
            items.push_back({token.kind, std::string(text), gap});
            gap.clear();
        }
        for (std::size_t i = 0; i < text.size(); ++i) {
            if (text[i] == '\r') {
                ++line_number;
                if (i + 1 < text.size() && text[i + 1] == '\n') ++i;
            } else if (text[i] == '\n') ++line_number;
        }
    }
    if (begin < source.size()) flush(source.size(), "", tokens.size());
    if (!conditionals.empty()) throw std::runtime_error("unterminated preprocessor conditional");
    if (!state.blocks.empty()) throw std::runtime_error("unterminated block, expected " + state.blocks.back().close);
    return declaration_spacing(out, options);
}

} // namespace hspfmt
