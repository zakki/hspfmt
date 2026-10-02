#include "hspfmt.h"
#include <iostream>
#include <stdexcept>

namespace {
unsigned count = 0;
void expect(const std::string &input, const std::string &expected, hspfmt::Options options = {}) {
    ++count;
    std::string restored;
    for (const auto &token : hspfmt::lex(input, options.encoding))
        restored.append(input, token.begin, token.end - token.begin);
    if (restored != input) throw std::runtime_error("roundtrip failed");
    const auto actual = hspfmt::format(input, options);
    if (actual != expected)
        throw std::runtime_error("case " + std::to_string(count) + "\nexpected:\n" + expected + "\nactual:\n" + actual);
    if (hspfmt::format(actual, options) != actual) throw std::runtime_error("not idempotent: " + actual);
}
void rejects(const std::string &input) {
    ++count;
    try { hspfmt::format(input); } catch (const std::runtime_error &) { return; }
    throw std::runtime_error("expected rejection: " + input);
}
}

int main() {
    try {
        expect("", "");
        expect("a=1:b=2\n", "a = 1 : b = 2\n");
        expect("if flag:foo=bar:baz=1\n", "if flag : foo = bar : baz = 1\n");
        expect("while x<10\nx+=1\nwend\n", "while x < 10\n    x += 1\nwend\n");
        expect("repeat 2\nwhile flag\nmes a\nwend\nloop\n", "repeat 2\n    while flag\n        mes a\n    wend\nloop\n");
        expect("for i,0,3\ndo\na++\nuntil a=2\nnext\n", "for i, 0, 3\n    do\n        a++\n    until a = 2\nnext\n");
        expect("switch x\ncase 1\na=2\nswbreak\ndefault\na=3\nswend\n",
               "switch x\n    case 1\n        a = 2\n        swbreak\n    default\n        a = 3\nswend\n");
        expect("if flag {\na=1\n} else {\na=2\n}\n", "if flag {\n    a = 1\n} else {\n    a = 2\n}\n");
        expect("#deffunc foo int p\nx=1\nreturn\n#global\nx=2\n", "#deffunc foo int p\n    x = 1\n    return\n#global\nx = 2\n");
        expect("*main\ngoto *main\na=a*2+-1\n", "*main\ngoto *main\na = a * 2 + -1\n");
        expect("mes \"; { : } //\" ; comment  stays\n", "mes \"; { : } //\" ; comment  stays\n");
        expect("s={\"\n  : while \\\"}\n\"}\nx=1\n", "s={\"\n  : while \\\"}\n\"}\nx = 1\n");
        expect("/* block\n while {\n*/\nx=1\n", "/* block\n while {\n*/\nx = 1\n");
        expect("\xef\xbb\xbf" "a=1\r\nb=2", "\xef\xbb\xbf" "a = 1\r\nb = 2");
        expect("#define f(%1) mes %1 : mes %1\nf 1\n", "#define f(%1) mes %1 : mes %1\nf 1\n");
        expect("#if FLAG\nwhile x\n#else\nwhile y\n#endif\na=1\nwend\n",
               "#if FLAG\nwhile x\n#else\nwhile y\n#endif\n    a = 1\nwend\n");
        expect("#if FLAG\na=1\n#else\na=2\n#endif\n", "#if FLAG\na = 1\n#else\na = 2\n#endif\n");
        expect("#ifdef A\n#deffunc f\nreturn\n#endif\n#global\n", "#ifdef A\n#deffunc f\n    return\n#endif\n#global\n");
        expect("foreach a\na(cnt)=1\nloop\n", "foreach a\n    a(cnt) = 1\nloop\n");
        expect("if flag { /* comment\n still comment */\nx=1\n}\n", "if flag { /* comment\n still comment */\n    x = 1\n}\n");
        expect("mes \"; hspfmt: off\"\na=1\n", "mes \"; hspfmt: off\"\na = 1\n");
        expect("a=1.0e-10:b=$ffff_0000L\n", "a = 1.0e-10 : b = $ffff_0000L\n");
        expect("mes a(0),,b@mod\n", "mes a(0),, b@mod\n");
        expect("a = 1 + \\\n  2\n", "a = 1 + \\\n  2\n");
        expect("; hspfmt: off\ncustom_start\nx=1\ncustom_end\n; hspfmt: on\na=2\n",
               "; hspfmt: off\ncustom_start\nx=1\ncustom_end\n; hspfmt: on\na = 2\n");
        hspfmt::Options options;
        options.hsp_numeric_prefixes = true;
        expect("a=0x00Ff:b=0b0010:s=\"0xff\"\n", "a = $00Ff : b = %0010 : s = \"0xff\"\n", options);
        options = {};
        options.short_if = true;
        expect("if (flag) { foo=bar:baz=1 }\n", "if flag : foo = bar : baz = 1\n", options);
        expect("if (flag) { foo=bar } : baz=1\n", "if (flag) { foo = bar } : baz = 1\n", options);
        expect("if a { if b { x=1 } }\n", "if a { if b { x = 1 } }\n", options);
        expect("if a { mes 1 }\n", "if a { mes 1 }\n", options);
        expect("if a { x=1 } ; note\n", "if a { x = 1 } ; note\n", options);
        options.line_width = 10;
        expect("if (flag) { x=1 }\n", "if (flag) { x = 1 }\n", options);
        options = {};
        options.tabs = true;
        expect("repeat\na=1\nloop\n", "repeat\n\ta = 1\nloop\n", options);
        options = {};
        options.binary_spaces = false;
        expect("a = b + 1\nif a = 1 : mes -1\n", "a=b+1\nif a=1 : mes -1\n", options);
        expect("x = a - -1\nx = a + +1\nif (a) and (b) : x=1\n", "x=a- -1\nx=a+ +1\nif (a) and (b) : x=1\n", options);
        options = {};
        options.encoding = hspfmt::Encoding::Cp932;
        expect("mes \"\x95\x5c\"\r\na=1\r\n", "mes \"\x95\x5c\"\r\na = 1\r\n", options);
        rejects("mes \"unterminated");
        rejects("/* unterminated");
        rejects("while x\nx=1\n");
        rejects("wend\n");
        rejects("#if X\nwhile a\n#endif\n");
        rejects("#else\n");
        rejects("mes \"\xc0\x80\"\n");
        rejects("mes \"\xed\xa0\x80\"\n");
        rejects(std::string("a\0b", 3));
        std::cout << count << " formatter cases passed\n";
        return 0;
    } catch (const std::exception &error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
