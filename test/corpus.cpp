#include "hspfmt.h"
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <stdexcept>

// Integration check against user-selected trees. Never writes corpus files.
int main(int argc, char **argv) {
    if (argc < 2) {
        std::cerr << "Usage: hspfmt_corpus directory [directory ...]\n";
        return 2;
    }
    unsigned files = 0, rejected = 0, failed = 0;
    for (int arg = 1; arg < argc; ++arg) {
        for (const auto &entry : std::filesystem::recursive_directory_iterator(argv[arg])) {
            const auto ext = entry.path().extension();
            if (!entry.is_regular_file() || (ext != ".as" && ext != ".hsp" && ext != ".chsp")) continue;
            std::ifstream file(entry.path(), std::ios::binary);
            const std::string input{std::istreambuf_iterator<char>(file), {}};
            hspfmt::Options options;
            std::vector<hspfmt::Token> original;
            try { original = hspfmt::lex(input, options.encoding); }
            catch (const std::runtime_error &) {
                options.encoding = hspfmt::Encoding::Cp932;
                try { original = hspfmt::lex(input, options.encoding); }
                catch (const std::runtime_error &e) {
                    ++rejected;
                    std::cerr << entry.path() << ": " << e.what() << '\n';
                    continue;
                }
            }
            ++files;
            std::string restored;
            for (const auto &token : original) restored.append(input, token.begin, token.end - token.begin);
            if (restored != input) { ++failed; std::cerr << entry.path() << ": roundtrip failed\n"; }
            std::string formatted;
            try { formatted = hspfmt::format(input, options); }
            catch (const std::runtime_error &e) {
                ++rejected;
                std::cerr << entry.path() << ": " << e.what() << '\n';
                continue;
            }
            try {
                if (hspfmt::format(formatted, options) != formatted) throw std::runtime_error("not idempotent");
                const auto after = hspfmt::lex(formatted, options.encoding);
                std::vector<std::pair<hspfmt::Kind, std::string>> before_tokens, after_tokens;
                for (const auto &t : original)
                    if (t.kind != hspfmt::Kind::Space) before_tokens.emplace_back(t.kind, input.substr(t.begin, t.end - t.begin));
                for (const auto &t : after)
                    if (t.kind != hspfmt::Kind::Space) after_tokens.emplace_back(t.kind, formatted.substr(t.begin, t.end - t.begin));
                if (before_tokens != after_tokens) throw std::runtime_error("non-whitespace tokens changed");
            } catch (const std::runtime_error &e) {
                ++failed;
                std::cerr << entry.path() << ": " << e.what() << '\n';
            }
        }
    }
    std::cout << files << " tokenized, " << rejected << " rejected, " << failed << " invariant failures\n";
    return failed ? 1 : 0;
}
