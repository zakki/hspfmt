#include "hspfmt.h"

#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <stdexcept>
#include <random>
#ifdef _WIN32
#define NOMINMAX
#include <windows.h>
#include <fcntl.h>
#include <io.h>
#endif

namespace {
namespace fs = std::filesystem;

void replace_file(const fs::path &path, const std::string &output) {
    // Stage on the same filesystem, in an exclusively created directory.
    // Only replace the original after the entire output has been closed successfully.
    std::random_device random;
    fs::path directory;
    for (unsigned attempt = 0; attempt < 100; ++attempt) {
        auto candidate = path.parent_path() / (".hspfmt-" + std::to_string(random()));
        if (fs::create_directory(candidate)) { directory = candidate; break; }
    }
    if (directory.empty()) throw std::runtime_error("cannot create temporary directory");
    const auto temporary = directory / "output";
    struct Cleanup {
        fs::path file, directory;
        ~Cleanup() {
            std::error_code ignored;
            fs::remove(file, ignored);
            fs::remove(directory, ignored);
        }
    } cleanup{temporary, directory};
    std::ofstream file(temporary, std::ios::binary);
    if (!file) throw std::runtime_error("cannot create temporary output");
    file.write(output.data(), static_cast<std::streamsize>(output.size()));
    file.close();
    if (!file) throw std::runtime_error("temporary output write failed");
    fs::permissions(temporary, fs::status(path).permissions());
#ifdef _WIN32
    if (!ReplaceFileW(path.c_str(), temporary.c_str(), nullptr, 0, nullptr, nullptr))
        throw std::runtime_error("cannot replace input file (Windows error " + std::to_string(GetLastError()) + ")");
#else
    fs::rename(temporary, path);
#endif
}

unsigned number(const std::string &s) {
    if (s.empty() || s.find_first_not_of("0123456789") != std::string::npos)
        throw std::runtime_error("invalid numeric option: " + s);
    const auto value = std::stoul(s);
    if (value > 10000) throw std::runtime_error("numeric option too large");
    return static_cast<unsigned>(value);
}

hspfmt::Parentheses parentheses(const std::string &s) {
    if (s == "preserve") return hspfmt::Parentheses::Preserve;
    if (s == "add") return hspfmt::Parentheses::Add;
    if (s == "remove") return hspfmt::Parentheses::Remove;
    throw std::runtime_error("invalid parentheses mode: " + s);
}

int blank_lines(const std::string &s) {
    if (s == "preserve") return -1;
    const auto value = number(s);
    if (value > 16) throw std::runtime_error("blank line count must be between 0 and 16");
    return static_cast<int>(value);
}

hspfmt::OperatorStyle operator_style(const std::string &s) {
    if (s == "preserve") return hspfmt::OperatorStyle::Preserve;
    if (s == "hsp") return hspfmt::OperatorStyle::Hsp;
    if (s == "c") return hspfmt::OperatorStyle::C;
    throw std::runtime_error("invalid operator style: " + s);
}

bool parse_formatting_option(const std::string &arg, hspfmt::Options &options) {
    if (arg == "--tabs") options.tabs = true;
    else if (arg == "--compact-operators") options.binary_spaces = false;
    else if (arg == "--hsp-prefixes") options.hsp_numeric_prefixes = true;
    else if (arg == "--short-if") options.short_if = true;
    else if (arg.rfind("--operator-style=", 0) == 0) options.operator_style = operator_style(arg.substr(17));
    else if (arg.rfind("--increment-style=", 0) == 0) options.increment_style = operator_style(arg.substr(18));
    else if (arg == "--indent-labels") options.indent_labels = true;
    else if (arg == "--no-indent-labels") options.indent_labels = false;
    else if (arg.rfind("--comment-style=", 0) == 0) {
        const auto value = arg.substr(16);
        if (value == "preserve") options.comment_style = hspfmt::CommentStyle::Preserve;
        else if (value == "semicolon") options.comment_style = hspfmt::CommentStyle::Semicolon;
        else if (value == "c") options.comment_style = hspfmt::CommentStyle::C;
        else throw std::runtime_error("invalid comment style: " + value);
    }
    else if (arg.rfind("--block-comments=", 0) == 0) {
        const auto value = arg.substr(17);
        if (value == "preserve") options.block_comments = hspfmt::BlockComments::Preserve;
        else if (value == "lines") options.block_comments = hspfmt::BlockComments::Lines;
        else if (value == "block") options.block_comments = hspfmt::BlockComments::Block;
        else throw std::runtime_error("invalid block comment mode: " + value);
    }
    else if (arg.rfind("--condition-parens=", 0) == 0) options.condition_parens = parentheses(arg.substr(19));
    else if (arg.rfind("--repeat-parens=", 0) == 0) options.repeat_parens = parentheses(arg.substr(16));
    else if (arg.rfind("--blank-lines-before-module=", 0) == 0) options.blank_lines_before_module = blank_lines(arg.substr(28));
    else if (arg.rfind("--blank-lines-before-deffunc=", 0) == 0) options.blank_lines_before_deffunc = blank_lines(arg.substr(29));
    else if (arg.rfind("--blank-lines-before-defcfunc=", 0) == 0) options.blank_lines_before_defcfunc = blank_lines(arg.substr(30));
    else if (arg == "--encoding=cp932") options.encoding = hspfmt::Encoding::Cp932;
    else if (arg == "--encoding=utf8") options.encoding = hspfmt::Encoding::Utf8;
    else if (arg.rfind("--indent=", 0) == 0) options.indent_width = number(arg.substr(9));
    else if (arg.rfind("--line-width=", 0) == 0) options.line_width = number(arg.substr(13));
    else return false;
    return true;
}

void load_config(const fs::path &path, hspfmt::Options &options) {
    std::ifstream file(path);
    if (!file) throw std::runtime_error("cannot open config file: " + path.string());
    std::string line;
    std::size_t line_num = 0;
    while (std::getline(file, line)) {
        ++line_num;
        const auto start = line.find_first_not_of(" \t\r\n");
        if (start == std::string::npos) continue;
        const auto end = line.find_last_not_of(" \t\r\n");
        std::string opt = line.substr(start, end - start + 1);
        if (opt.empty() || opt[0] == '#' || opt[0] == ';') continue;
        if (opt.rfind("--", 0) != 0) opt = "--" + opt;
        if (!parse_formatting_option(opt, options))
            throw std::runtime_error(path.string() + ":" + std::to_string(line_num) +
                                     ": invalid or unsupported config option: " + opt);
    }
    if (file.bad() || (file.fail() && !file.eof()))
        throw std::runtime_error("config read failed: " + path.string());
}
}

int main(int argc, char **argv) {
    std::string input_name;
    try {
        hspfmt::Options options;
        bool check = false;
        bool write = false;
        bool roundtrip = false;
        bool positional = false;
        std::string stdin_filepath;
        std::string config_path;
        bool no_config = false;
        std::vector<std::string> filenames;

        for (int i = 1; i < argc; ++i) {
            const std::string arg = argv[i];
            if (arg == "--") break;
            if (arg == "--help") {
                std::cout << "Usage: hspfmt [options] [file...|-]\n"
                    "Writes formatted source to stdout unless --write is specified.\n"
                    "  --write, -w          Replace the input file(s) in place (no stdout)\n"
                    "  --check              Exit 1 if formatting differs, 0 if unchanged\n"
                    "  --roundtrip          Reconstruct source from lossless tokens\n"
                    "  --config=FILE        Load configuration file (default: .hspfmt)\n"
                    "  --no-config          Disable configuration file loading\n"
                    "  --stdin-filepath=PATH File path used in diagnostic messages for stdin\n"
                    "  --indent=N --tabs    Indentation (default: 4 spaces)\n"
                    "  --compact-operators Remove optional binary-operator spaces\n"
                    "  --hsp-prefixes       Convert 0x/0b to $/% (preserve digits)\n"
                    "  --operator-style=preserve|hsp|c (binary operator spelling)\n"
                    "  --increment-style=preserve|hsp|c (increment/decrement statements)\n"
                    "  --short-if           Collapse isolated single-line assignment if blocks\n"
                    "  --line-width=N       Short-if byte width limit (default: 100)\n"
                    "  --indent-labels / --no-indent-labels (default: no indentation)\n"
                    "  --comment-style=preserve|semicolon|c (line comment markers)\n"
                    "  --block-comments=preserve|lines|block (standalone comments)\n"
                    "  --condition-parens=preserve|add|remove (if/while)\n"
                    "  --repeat-parens=preserve|add|remove (each repeat argument)\n"
                    "  --blank-lines-before-module=N|preserve (0..16)\n"
                    "  --blank-lines-before-deffunc=N|preserve (0..16)\n"
                    "  --blank-lines-before-defcfunc=N|preserve (0..16)\n"
                    "  --encoding=utf8|cp932 (default: utf8; bytes are preserved)\n"
                    "Exit 2 indicates an input, syntax, option, or output error.\n";
                return 0;
            } else if (arg == "--no-config") {
                no_config = true;
            } else if (arg.rfind("--config=", 0) == 0) {
                config_path = arg.substr(9);
                if (config_path.empty())
                    throw std::runtime_error("--config requires a non-empty file path");
            }
        }

        if (!no_config) {
            if (!config_path.empty()) {
                load_config(config_path, options);
            } else if (fs::exists(".hspfmt")) {
                load_config(".hspfmt", options);
            }
        }

        for (int i = 1; i < argc; ++i) {
            const std::string arg = argv[i];
            if (!positional && arg == "--") positional = true;
            else if (!positional && (arg == "--no-config" || arg.rfind("--config=", 0) == 0)) {
            }
            else if (!positional && arg == "--check") check = true;
            else if (!positional && (arg == "--write" || arg == "-w")) write = true;
            else if (!positional && arg == "--roundtrip") roundtrip = true;
            else if (!positional && arg.rfind("--stdin-filepath=", 0) == 0) stdin_filepath = arg.substr(17);
            else if (!positional && parse_formatting_option(arg, options)) {
            } else if (!positional && arg.size() > 1 && arg[0] == '-') {
                throw std::runtime_error("unknown option: " + arg);
            } else {
                filenames.push_back(arg);
            }
        }

        if (filenames.empty()) filenames.push_back("-");
        const bool is_stdin = filenames.size() == 1 && filenames[0] == "-";

        if (!stdin_filepath.empty() && !is_stdin)
            throw std::runtime_error("--stdin-filepath can only be used with stdin");

        if (filenames.size() > 1) {
            if (!write && !check)
                throw std::runtime_error("multiple files are only supported with --write or --check");
            if (roundtrip)
                throw std::runtime_error("--roundtrip cannot be combined with multiple files");
            for (const auto &f : filenames) {
                if (f == "-") throw std::runtime_error("cannot combine stdin with multiple files");
            }
        }

        if (write && is_stdin)
            throw std::runtime_error("--write requires an input file, not stdin");
        if (write && (check || roundtrip))
            throw std::runtime_error("--write cannot be combined with --check or --roundtrip");

#ifdef _WIN32
        _setmode(_fileno(stdin), _O_BINARY);
        _setmode(_fileno(stdout), _O_BINARY);
#endif

        bool has_diff = false;
        for (const auto &filename : filenames) {
            input_name = filename == "-"
                ? (!stdin_filepath.empty() ? stdin_filepath : "<stdin>")
                : filename;
            if (write && (!fs::is_regular_file(fs::symlink_status(filename)) || fs::hard_link_count(filename) != 1))
                throw std::runtime_error("--write requires a regular file without symbolic or hard links");

            std::ifstream file;
            std::istream *input = &std::cin;
            if (filename != "-") {
                file.open(filename, std::ios::binary);
                if (!file) throw std::runtime_error("cannot open input file");
                input = &file;
            }
            const std::string source{std::istreambuf_iterator<char>(*input), {}};
            if (input->bad()) throw std::runtime_error("input read failed");
            if (file.is_open()) file.close();
            std::string output;
            if (roundtrip) {
                for (const auto &token : hspfmt::lex(source, options.encoding))
                    output.append(source, token.begin, token.end - token.begin);
            } else {
                std::vector<hspfmt::Diagnostic> diagnostics;
                output = hspfmt::format(source, options, &diagnostics);
                for (const auto &diagnostic : diagnostics) {
                    std::cerr << input_name
                              << ':' << diagnostic.line
                              << ": warning: ambiguous label or multiplication; preserving whitespace\n"
                              << diagnostic.source << '\n';
                }
            }
            if (check) {
                if (output != source) has_diff = true;
            } else if (write) {
                if (output != source) replace_file(filename, output);
            } else {
                std::cout.write(output.data(), static_cast<std::streamsize>(output.size()));
                std::cout.flush();
                if (!std::cout) throw std::runtime_error("output write failed");
            }
        }
        if (check) return has_diff ? 1 : 0;
        return 0;
    } catch (const std::exception &error) {
        std::cerr << "hspfmt: ";
        if (!input_name.empty()) std::cerr << input_name << ": ";
        std::cerr << error.what() << '\n';
        return 2;
    }
}
