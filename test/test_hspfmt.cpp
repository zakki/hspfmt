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
void rejects(const std::string &input, hspfmt::Options options = {}) {
    ++count;
    try { hspfmt::format(input, options); } catch (const std::runtime_error &) { return; }
    throw std::runtime_error("expected rejection: " + input);
}
void diagnostic_cases(const std::string &input, const std::string &expected,
                      const std::vector<hspfmt::Diagnostic> &warnings, hspfmt::Options options = {}) {
    expect(input, expected, options);
    std::vector<hspfmt::Diagnostic> actual;
    if (hspfmt::format(input, options, &actual) != expected || actual.size() != warnings.size())
        throw std::runtime_error("unexpected ambiguity diagnostics: " + input);
    for (std::size_t i = 0; i < actual.size(); ++i)
        if (actual[i].line != warnings[i].line || actual[i].source != warnings[i].source)
            throw std::runtime_error("wrong ambiguity source location: " + input);
}
}

int main() {
    try {
        expect("", "");
        expect("a=1:b=2\n", "a = 1 : b = 2\n");
        for (const auto encoding : {hspfmt::Encoding::Utf8, hspfmt::Encoding::Cp932}) {
            hspfmt::Options japanese;
            japanese.encoding = encoding;
            // Kanji, hiragana, U+3000, and a CP932 character whose trail byte is 0x5c.
            const std::string kanji = encoding == hspfmt::Encoding::Utf8
                ? u8"\u6f22\u5b57" : "\x8a\xbf\x8e\x9a";
            const std::string hiragana = encoding == hspfmt::Encoding::Utf8
                ? u8"\u3072\u3089\u304c\u306a" : "\x82\xd0\x82\xe7\x82\xaa\x82\xc8";
            const std::string table = encoding == hspfmt::Encoding::Utf8
                ? u8"\u8868" : "\x95\x5c";
            const std::string full_space = encoding == hspfmt::Encoding::Utf8
                ? u8"\u3000" : "\x81\x40";
            const std::string sum = encoding == hspfmt::Encoding::Utf8
                ? u8"\u52a0\u7b97" : "\x89\xc1\x8e\x5a";
            expect(kanji + "=2\n" + hiragana + "=3\n" + table + "=" + kanji + "+" + hiragana + "\n",
                   kanji + " = 2\n" + hiragana + " = 3\n" + table + " = " + kanji + " + " + hiragana + "\n", japanese);
            expect(kanji + "A1=" + hiragana + "2*3\n",
                   kanji + "A1 = " + hiragana + "2 * 3\n", japanese);
            expect("dim " + kanji + ",2\n" + kanji + "(0)=" + table + "\n" + kanji + ".1=" + hiragana + "\n",
                   "dim " + kanji + ", 2\n" + kanji + "(0) = " + table + "\n" + kanji + ".1 = " + hiragana + "\n", japanese);
            expect("*" + table + "\ngosub *" + table + "\n" + kanji + "=*" + table + "\n",
                   "*" + table + "\ngosub *" + table + "\n" + kanji + " = *" + table + "\n", japanese);
            expect(hiragana + "@" + kanji + " *" + table + "\n",
                   hiragana + "@" + kanji + " *" + table + "\n", japanese);
            // Preserve full-width spaces as identifier bytes, including leading/standalone ones.
            expect(kanji + full_space + hiragana + "=1\n" + full_space + kanji + "=2\n" + full_space + "=3\n",
                   kanji + full_space + hiragana + " = 1\n" + full_space + kanji + " = 2\n" + full_space + " = 3\n", japanese);
            expect("mes" + full_space + kanji + "\n" + full_space + "mes 1\n",
                   "mes" + full_space + kanji + "\n" + full_space + "mes 1\n", japanese);
            expect("mes \"" + table + full_space + hiragana + "\" ; " + kanji + full_space + "\n",
                   "mes \"" + table + full_space + hiragana + "\" ; " + kanji + full_space + "\n", japanese);
            // Keep the ASCII uppercase trail byte in the CP932 function name intact.
            expect("x=" + sum + "(" + kanji + "," + hiragana + ")\n",
                   "x = " + sum + "(" + kanji + ", " + hiragana + ")\n", japanese);
            japanese.operator_style = hspfmt::OperatorStyle::C;
            japanese.increment_style = hspfmt::OperatorStyle::C;
            japanese.binary_spaces = false;
            expect("if " + kanji + "=" + hiragana + " : " + table + "+\n",
                   "if " + kanji + "==" + hiragana + " : " + table + "++\n", japanese);
            hspfmt::Options normalized;
            normalized.encoding = encoding;
            normalized.full_width_spaces = hspfmt::FullWidthSpaces::Normalize;
            expect(full_space + "repeat" + full_space + "2\n" + full_space + kanji + full_space + "=" + full_space + hiragana + "+1\nloop\n",
                   "repeat 2\n    " + kanji + " = " + hiragana + " + 1\nloop\n", normalized);
            expect("mes" + full_space + full_space + kanji + full_space + "\n",
                   "mes " + kanji + "\n", normalized);
            expect("mes" + full_space + "\"" + table + full_space + "\" ; " + full_space + "\n" +
                   "mes" + full_space + "'" + full_space + "' // " + full_space + "\n" +
                   "a" + full_space + "=" + full_space + "1 /* " + full_space + " */\n",
                   "mes \"" + table + full_space + "\" ; " + full_space + "\n" +
                   "mes '" + full_space + "' // " + full_space + "\n" +
                   "a = 1 /* " + full_space + " */\n", normalized);
            expect("mes" + full_space + "{\"" + full_space + "\r\n" + table + full_space + "\"}\r\n" +
                   full_space + "/* " + full_space + "\r\n" + full_space + " */\r\n",
                   "mes {\"" + full_space + "\r\n" + table + full_space + "\"}\r\n" +
                   " /* " + full_space + "\r\n" + full_space + " */\r\n", normalized);
            expect("\xef\xbb\xbf" + full_space + kanji + "=1\r\nmes" + full_space + kanji,
                   "\xef\xbb\xbf" + kanji + " = 1\r\nmes " + kanji, normalized);
            expect("#const" + full_space + kanji + full_space + "1\nmes" + full_space + "1,\\\n" + full_space + "2\n",
                   "#const " + kanji + " 1\nmes 1,\\\n 2\n", normalized);
            expect("; hspfmt: off\nmes" + full_space + kanji + "\n; hspfmt: on\nmes" + full_space + kanji + "\n",
                   "; hspfmt: off\nmes" + full_space + kanji + "\n; hspfmt: on\nmes " + kanji + "\n", normalized);
            expect(full_space + " \t" + full_space + "; hspfmt: off\nmes" + full_space + kanji + "\n" + full_space + "; hspfmt: on\nmes" + full_space + kanji + "\n",
                   "  \t ; hspfmt: off\nmes" + full_space + kanji + "\n ; hspfmt: on\nmes " + kanji + "\n", normalized);
            diagnostic_cases("mes" + full_space + "1\nfoo" + full_space + "*bar\n",
                             "mes 1\nfoo *bar\n", {{2, "foo" + full_space + "*bar"}}, normalized);
            normalized.comment_style = hspfmt::CommentStyle::C;
            normalized.block_comments = hspfmt::BlockComments::Lines;
            expect("/* " + full_space + " */\nmes" + full_space + kanji + " ; " + full_space + "\n",
                   "// " + full_space + " \nmes " + kanji + " // " + full_space + "\n", normalized);
            if (encoding == hspfmt::Encoding::Cp932)
                expect("a=\x83\x81@b\n", "a = \x83\x81@b\n", normalized);
        }
        diagnostic_cases("foo *bar\n", "foo *bar\n", {{1, "foo *bar"}});
        diagnostic_cases("foo*bar\n", "foo*bar\n", {{1, "foo*bar"}});
        diagnostic_cases("foo * bar\n", "foo * bar\n", {});
        diagnostic_cases("foo\t*\tbar\n", "foo\t*\tbar\n", {{1, "foo\t*\tbar"}});
        diagnostic_cases("foo@mod*bar,2*3\n", "foo@mod*bar, 2 * 3\n", {{1, "foo@mod*bar,2*3"}});
        diagnostic_cases("foo /* head */ *bar\n", "foo /* head */ *bar\n", {{1, "foo /* head */ *bar"}});
        diagnostic_cases("if flag:foo *bar:baz*qux\n", "if flag : foo *bar : baz*qux\n",
                         {{1, "if flag:foo *bar:baz*qux"}});
        diagnostic_cases("a=*lb\na=x*y\nx=f(*lb,2*3)\na(0)*b\na.0*b\n",
                         "a = *lb\na = x * y\nx = f(*lb, 2 * 3)\na(0) * b\na.0 * b\n", {});
        diagnostic_cases("goto *lb\ngosub *lb\nonclick gosub *lb\non n goto *lb\n",
                         "goto *lb\ngosub *lb\nonclick gosub *lb\non n goto *lb\n", {});
        diagnostic_cases("/* first\nsecond */\r\nfoo*bar\r\n", "/* first\nsecond */\r\nfoo*bar\r\n",
                         {{3, "foo*bar"}});
        diagnostic_cases("mes {\"\nmultiline\n\"}\nfoo*bar", "mes {\"\nmultiline\n\"}\nfoo*bar",
                         {{4, "foo*bar"}});
        diagnostic_cases("; hspfmt: off\nfoo*bar\n; hspfmt: on\nfoo*bar\n",
                         "; hspfmt: off\nfoo*bar\n; hspfmt: on\nfoo*bar\n", {{4, "foo*bar"}});
        expect("if flag:foo=bar:baz=1\n", "if flag : foo = bar : baz = 1\n");
        expect("while x<10\nx+=1\nwend\n", "while x < 10\n    x += 1\nwend\n");
        expect("repeat 2\nwhile flag\nmes a\nwend\nloop\n", "repeat 2\n    while flag\n        mes a\n    wend\nloop\n");
        expect("for i,0,3\ndo\na++\nuntil a=2\nnext\n", "for i, 0, 3\n    do\n        a++\n    until a = 2\nnext\n");
        expect("switch x\ncase 1\na=2\nswbreak\ndefault\na=3\nswend\n",
               "switch x\n    case 1\n        a = 2\n        swbreak\n    default\n        a = 3\nswend\n");
        expect("if flag {\na=1\n} else {\na=2\n}\n", "if flag {\n    a = 1\n} else {\n    a = 2\n}\n");
        expect("#deffunc foo int p\nx=1\nreturn\n#global\nx=2\n", "#deffunc foo int p\n    x = 1\n    return\n#global\nx = 2\n");
        expect("*main\ngoto *main\na=a*2+-1\n", "*main\ngoto *main\na = a * 2 + -1\n");
        expect("onclick gosub *queue_mouse_click\nonkey goto *key\nonexit gosub *exit\n"
               "onerror goto *error\noncmd gosub *message,100+2\n",
               "onclick gosub *queue_mouse_click\nonkey goto *key\nonexit gosub *exit\n"
               "onerror goto *error\noncmd gosub *message, 100 + 2\n");
        expect("on (n*2) GOSUB *first,*second\nif flag:onclick goto *click\n"
               "onclick *click\nbutton gosub \"go\",*click\na=b*c\n",
               "on (n * 2) GOSUB *first, *second\nif flag : onclick goto *click\n"
               "onclick *click\nbutton gosub \"go\", *click\na = b * c\n");
        for (const auto style : {hspfmt::OperatorStyle::Preserve, hspfmt::OperatorStyle::Hsp,
                                 hspfmt::OperatorStyle::C}) {
            for (const bool compact : {false, true}) {
                hspfmt::Options label_options;
                label_options.operator_style = style;
                label_options.binary_spaces = !compact;
                expect("a = *lb1\nlabels(0)=*lb1:labels.1=*lb2\na@mod=*lb1@mod\n"
                       "gosub a\nonclick gosub labels(0)\nproduct=x*y\n",
                       compact ? "a=*lb1\nlabels(0)=*lb1 : labels.1=*lb2\na@mod=*lb1@mod\n"
                                 "gosub a\nonclick gosub labels(0)\nproduct=x*y\n"
                               : "a = *lb1\nlabels(0) = *lb1 : labels.1 = *lb2\na@mod = *lb1@mod\n"
                                 "gosub a\nonclick gosub labels(0)\nproduct = x * y\n",
                       label_options);
                expect("custom *lb1,2*3,*lb2\ncustom 2*3,*lb1\n"
                       "x=custom_value(*lb1,custom_value(*lb2,2*3,*lb1),*lb2)+1\n",
                       compact ? "custom *lb1, 2*3, *lb2\ncustom 2*3, *lb1\n"
                                 "x=custom_value(*lb1, custom_value(*lb2, 2*3, *lb1), *lb2)+1\n"
                               : "custom *lb1, 2 * 3, *lb2\ncustom 2 * 3, *lb1\n"
                                 "x = custom_value(*lb1, custom_value(*lb2, 2 * 3, *lb1), *lb2) + 1\n",
                       label_options);
                expect("custom@mod *lb1,2*3,*lb2\nx=custom_value@mod(*lb1,2*3,*lb2)\n"
                       "product=x@mod*y@mod\n",
                       compact ? "custom@mod *lb1, 2*3, *lb2\nx=custom_value@mod(*lb1, 2*3, *lb2)\n"
                                 "product=x@mod*y@mod\n"
                               : "custom@mod *lb1, 2 * 3, *lb2\nx = custom_value@mod(*lb1, 2 * 3, *lb2)\n"
                                 "product = x@mod * y@mod\n",
                       label_options);
            }
        }
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
        expect("a=1d:b=2D:c=1.25d:d=2.5D\n", "a = 1d : b = 2D : c = 1.25d : d = 2.5D\n");
        expect("a=%_0010:b=%__10_01:c=%_1010L\n", "a = %_0010 : b = %__10_01 : c = %_1010L\n");
        expect("mes a(0),,b@mod\n", "mes a(0),, b@mod\n");
        expect("a = 1 + \\\n  2\n", "a = 1 + \\\n  2\n");
        expect("; hspfmt: off\ncustom_start\nx=1\ncustom_end\n; hspfmt: on\na=2\n",
               "; hspfmt: off\ncustom_start\nx=1\ncustom_end\n; hspfmt: on\na = 2\n");
        hspfmt::Options options;
        options.hsp_numeric_prefixes = true;
        expect("a=0x00Ff:b=0b0010:s=\"0xff\"\n", "a = $00Ff : b = %0010 : s = \"0xff\"\n", options);
        expect("a=0b_0010:b=0b__10_01\n", "a = %_0010 : b = %__10_01\n", options);
        options = {};
        options.short_if = true;
        expect("if (flag) { foo=bar:baz=1 }\n", "if flag : foo = bar : baz = 1\n", options);
        expect("if (flag) { foo=bar } : baz=1\n", "if (flag) { foo = bar } : baz = 1\n", options);
        expect("if a { if b { x=1 } }\n", "if a { if b { x = 1 } }\n", options);
        expect("if a { mes 1 }\n", "if a { mes 1 }\n", options);
        expect("if a { x=1 } ; note\n", "if a { x = 1 } ; note\n", options);
        expect("if a { x=1 }\nelse { x=2 }\n", "if a { x = 1 }\nelse { x = 2 }\n", options);
        expect("if a { x=1 }\r\n  ELSE { x=2 }",
               "if a { x = 1 }\r\nELSE { x = 2 }", options);
        expect("if a { x=1 }\n/* note */ else { x=2 }\n",
               "if a { x = 1 }\n/* note */ else { x = 2 }\n", options);
        expect("if a {\nif b { x=1 }\nelse { x=2 }\n}\n",
               "if a {\n    if b { x = 1 }\n    else { x = 2 }\n}\n", options);
        expect("if a { x=1 }\n; note\ny=2\n", "if a : x = 1\n; note\ny = 2\n", options);
        options.line_width = 10;
        expect("if (flag) { x=1 }\n", "if (flag) { x = 1 }\n", options);
        options = {};
        options.tabs = true;
        expect("repeat\na=1\nloop\n", "repeat\n\ta = 1\nloop\n", options);
        options = {};
        options.binary_spaces = false;
        expect("a=1d:b=2D:c=%_0010\n", "a=1d : b=2D : c=%_0010\n", options);
        diagnostic_cases("foo*bar\n", "foo*bar\n", {}, options);
        diagnostic_cases("foo *bar\n", "foo *bar\n", {{1, "foo *bar"}}, options);
        diagnostic_cases("foo * bar\n", "foo * bar\n", {{1, "foo * bar"}}, options);
        diagnostic_cases("foo\t*\tbar\rfoo@mod *bar", "foo\t*\tbar\rfoo@mod *bar",
                         {{1, "foo\t*\tbar"}, {2, "foo@mod *bar"}}, options);
        expect("onclick gosub *click\non n+1 goto *first,*second\na=b*c\n",
               "onclick gosub *click\non n+1 goto *first, *second\na=b*c\n", options);
        expect("a = b + 1\nif a = 1 : mes -1\n", "a=b+1\nif a=1 : mes -1\n", options);
        expect("x = a - -1\nx = a + +1\nif (a) and (b) : x=1\n", "x=a- -1\nx=a+ +1\nif (a) and (b) : x=1\n", options);
        options = {};
        options.encoding = hspfmt::Encoding::Cp932;
        diagnostic_cases("\xef\xbb\xbf" "foo*bar\r\nfoo *\x95\x5c\r\n",
                         "\xef\xbb\xbf" "foo*bar\r\nfoo *\x95\x5c\r\n",
                         {{1, "\xef\xbb\xbf" "foo*bar"}, {2, "foo *\x95\x5c"}}, options);
        expect("mes \"\x95\x5c\"\r\na=1\r\n", "mes \"\x95\x5c\"\r\na = 1\r\n", options);
        options = {};
        options.indent_labels = true;
        expect("*main\na=1\ngoto *next\nreturn\na=2\n*next\nrepeat 2\na++\nloop\n",
               "*main\n    a = 1\n    goto *next\n    return\n    a = 2\n*next\n    repeat 2\n        a++\n    loop\n", options);
        expect("*main\nx=1\n#deffunc f\n*local\nx=2\n#global\nx=3\n",
               "*main\n    x = 1\n#deffunc f\n*local\n    x = 2\n#global\nx = 3\n", options);
        expect("#if A\n*one\n#else\n*two\n#endif\nx=1\n", "#if A\n*one\n#else\n*two\n#endif\n    x = 1\n", options);
        expect("*main /* docs\nmore */\nx=1\n", "*main /* docs\nmore */\n    x = 1\n", options);
        options.tabs = true;
        expect("*main\nrepeat\nx=1\nloop\n", "*main\n\trepeat\n\t\tx = 1\n\tloop\n", options);
        options = {};
        options.comment_style = hspfmt::CommentStyle::Semicolon;
        diagnostic_cases("// head\nfoo*bar // tail\n", "; head\nfoo*bar ; tail\n",
                         {{2, "foo*bar // tail"}}, options);
        expect("// hello\nx=1 // tail\nmes \"// stays\"\n", "; hello\nx = 1 ; tail\nmes \"// stays\"\n", options);
        expect("#define f x // stays\nx = 1 + \\\n  2 // stays\n", "#define f x // stays\nx = 1 + \\\n  2 // stays\n", options);
        expect("// hspfmt: off\nx=1\n", "// hspfmt: off\nx = 1\n", options);
        options.comment_style = hspfmt::CommentStyle::C;
        expect("; hello\nx=1 ; tail\n; hspfmt: off\nx=2 ; kept\n; hspfmt: on\nx=3\n",
               "// hello\nx = 1 // tail\n; hspfmt: off\nx=2 ; kept\n; hspfmt: on\nx = 3\n", options);
        options.block_comments = hspfmt::BlockComments::Lines;
        diagnostic_cases("/* head\n tail */\nfoo*bar // tail\n", "// head\n// tail \nfoo*bar // tail\n",
                         {{3, "foo*bar // tail"}}, options);
        expect("/* hello\n world\n*/\nx=1\n", "// hello\n// world\n//\nx = 1\n", options);
        expect("x=1 /* inline */ : x=2\n", "x = 1 /* inline */ : x = 2\n", options);
        expect("/* head */ x=1\n", "/* head */ x = 1\n", options);
        expect("if x { /* multiline\n kept */\nx=1\n}\n", "if x { /* multiline\n kept */\n    x = 1\n}\n", options);
        expect("\xef\xbb\xbf" "/* head\r\n tail*/", "\xef\xbb\xbf" "// head\r\n// tail", options);
        expect("/* hspfmt: off */\nx=1\n", "/* hspfmt: off */\nx = 1\n", options);
        options.comment_style = hspfmt::CommentStyle::Semicolon;
        expect("repeat\n/* a\nb*/\nloop\n", "repeat\n    ; a\n    ;b\nloop\n", options);
        options = {};
        options.block_comments = hspfmt::BlockComments::Block;
        expect("; a\n// b\nx=1\n", "/* a\n b*/\nx = 1\n", options);
        expect("; a\r\n; b", "/* a\r\n b*/", options);
        expect("; a */ b\n; c /* d\n; safe\n", "; a */ b\n; c /* d\n/* safe*/\n", options);
        expect("x=1 ; tail\n; hspfmt: off\n; kept\n; hspfmt: on\n", "x = 1 ; tail\n; hspfmt: off\n; kept\n; hspfmt: on\n", options);
        options = {};
        options.condition_parens = hspfmt::Parentheses::Add;
        expect("if a+b>0 : x=1\nwhile flag\nx=2\nwend\n", "if (a + b > 0) : x = 1\nwhile (flag)\n    x = 2\nwend\n", options);
        expect("if (a) and (b) { if x : y=1 }\n", "if ((a) and (b)) { if (x) : y = 1 }\n", options);
        expect("if f(1,2) : x=1 ; tail\n", "if (f(1, 2)) : x = 1 ; tail\n", options);
        expect("if a {\nx=1\n} else if b {\nx=2\n}\n", "if (a) {\n    x = 1\n} else if (b) {\n    x = 2\n}\n", options);
        expect("while\nx=1\nwend\n", "while\n    x = 1\nwend\n", options);
        expect("if flag /* inline */ : x=1\n", "if flag /* inline */ : x = 1\n", options);
        expect("#if X\nx=1\n#endif\n", "#if X\nx = 1\n#endif\n", options);
        options.short_if = true;
        expect("if (a) { x=1 }\n", "if (a) : x = 1\n", options);
        options.line_width = 16;
        expect("if flag { x=1 }\n", "if (flag) { x = 1 }\n", options);
        options = {};
        options.condition_parens = hspfmt::Parentheses::Remove;
        expect("if ((flag)) : x=1\nwhile ((a)+(b))\nx=2\nwend\n", "if flag : x = 1\nwhile (a) +(b)\n    x = 2\nwend\n", options);
        expect("if (-a) : x=1\nif (a) & (b) : y=1\n", "if -a : x = 1\nif (a) & (b) : y = 1\n", options);
        options = {};
        options.repeat_parens = hspfmt::Parentheses::Add;
        expect("repeat n+1, f(1,2)\nx=1\nloop\nrepeat ,2\nloop\nrepeat\nloop\n",
               "repeat (n + 1), (f(1, 2))\n    x = 1\nloop\nrepeat , (2)\nloop\nrepeat\nloop\n", options);
        options.repeat_parens = hspfmt::Parentheses::Remove;
        expect("repeat ((n+1)), (f(1,2))\nloop\n", "repeat n + 1, f(1, 2)\nloop\n", options);
        options = {};
        options.blank_lines_before_module = 2;
        options.blank_lines_before_deffunc = 1;
        options.blank_lines_before_defcfunc = 0;
        diagnostic_cases("x=1\n#module m\nfoo*bar\n#global\n",
                         "x = 1\n\n\n#module m\nfoo*bar\n#global\n", {{3, "foo*bar"}}, options);
        expect("\n\n; module docs\n#module m\n\n\n; function docs\n#deffunc f\nx=1\n\n#defcfunc g\nreturn 1\n#global\n#module n\n",
               "; module docs\n#module m\n\n; function docs\n#deffunc f\n    x = 1\n#defcfunc g\n    return 1\n#global\n\n\n#module n\n", options);
        expect("\xef\xbb\xbf" "\r\n\r\n#module m\r\nx=1\r\n#deffunc f",
               "\xef\xbb\xbf" "#module m\r\nx = 1\r\n\r\n#deffunc f", options);
        expect("\xef\xbb\xbf" "; docs\n#module m\n", "\xef\xbb\xbf" "; docs\n#module m\n", options);
        expect("x=1\n/* docs\nmore */\n#deffunc f\nreturn\n", "x = 1\n\n/* docs\nmore */\n#deffunc f\n    return\n", options);
        expect("s={\"\n#module text\n\"}\n; hspfmt: off\n#module kept\n; hspfmt: on\n#module m\n",
               "s={\"\n#module text\n\"}\n; hspfmt: off\n#module kept\n; hspfmt: on\n\n\n#module m\n", options);
        options.comment_style = hspfmt::CommentStyle::Semicolon;
        options.block_comments = hspfmt::BlockComments::Lines;
        expect("x=1\n/* docs\nmore */\n#deffunc f\nreturn\n", "x = 1\n\n; docs\n;more \n#deffunc f\n    return\n", options);
        options = {};
        options.encoding = hspfmt::Encoding::Cp932;
        options.comment_style = hspfmt::CommentStyle::C;
        expect("; \x95\x5c\r\n", "// \x95\x5c\r\n", options);
        options = {};
        options.operator_style = hspfmt::OperatorStyle::C;
        expect("a=b\nx=a=b\nif a=b & a!c | b=c : x=1\n",
               "a = b\nx = a == b\nif a == b && a != c || b == c : x = 1\n", options);
        expect("mes (a)=b\nwhile (a)=b\na-=1\nwend\n", "mes (a) == b\nwhile (a) == b\n    a -= 1\nwend\n", options);
        expect("a(0)=b=c\na@mod=1\na.0=2\na.(i+1)=3\n",
               "a(0) = b == c\na@mod = 1\na.0 = 2\na.(i + 1) = 3\n", options);
        expect("a (i=j)=b!c\n", "a (i == j) = b != c\n", options);
        expect("a /* lhs */ = b /* rhs */ = c\n", "a /* lhs */ = b /* rhs */ == c\n", options);
        expect("a&=b:c|=d\na & b=c\n", "a &= b : c |= d\na & b == c\n", options);
        expect("foo (a)=b\n", "foo (a) = b\n", options);
        expect("if a=b { x=1 } else if c!d { x=2 }\n", "if a == b { x = 1 } else if c != d { x = 2 }\n", options);
        expect("mes \"a=b & c!d\" ; a=b\n#define f(%1) %1=1\nf 1\n",
               "mes \"a=b & c!d\" ; a=b\n#define f(%1) %1=1\nf 1\n", options);
        expect("; hspfmt: off\na=b=c\n; hspfmt: on\nx=1 + \\\n a=b\n",
               "; hspfmt: off\na=b=c\n; hspfmt: on\nx=1 + \\\n a=b\n", options);
        expect("mes !a\nx=a!\n", "mes ! a\nx = a !\n", options);
        options.binary_spaces = false;
        expect("x=a=b & c! -d\n", "x=a==b&&c!=-d\n", options);
        options = {};
        options.operator_style = hspfmt::OperatorStyle::Hsp;
        expect("a=b\nx=a==b\nif a==b && a!=c || b==c : x=1\n",
               "a = b\nx = a = b\nif a = b & a ! c | b = c : x = 1\n", options);
        expect("x==b\na(0)==b==c\n", "x == b\na(0) == b = c\n", options);
        expect("x=a<=b && a>=b\na+=1\n", "x = a <= b & a >= b\na += 1\n", options);
        options.binary_spaces = false;
        expect("x = a == b && c != -d\n", "x=a=b&c!-d\n", options);
        options = {};
        options.increment_style = hspfmt::OperatorStyle::C;
        expect("a+:b-\na(0)+\na@mod-\na.0+\na.(i+1)-\n",
               "a++ : b--\na(0)++\na@mod--\na.0++\na.(i + 1)--\n", options);
        expect("if flag : a+\nif flag { a- }\n", "if flag : a++\nif flag { a-- }\n", options);
        expect("a /* lhs */ + /* tail */\n", "a /* lhs */++ /* tail */\n", options);
        expect("a+2\na+=2\nx=a+b\nmes a+\nx=a++\n++a\n", "a +2\na += 2\nx = a + b\nmes a+\nx = a++\n++ a\n", options);
        expect("a+ /* middle */ +b\n", "a+ /* middle */ +b\n", options);
        options = {};
        options.increment_style = hspfmt::OperatorStyle::Hsp;
        expect("a++:b--\na(0)++\na@mod--\na.0++\n", "a+ : b-\na(0)+\na@mod-\na.0+\n", options);
        expect("x=a++\nmes a--\n++a\na+=2\n", "x = a++\nmes a--\n++ a\na += 2\n", options);
        options = {};
        options.operator_style = hspfmt::OperatorStyle::C;
        options.increment_style = hspfmt::OperatorStyle::C;
        options.short_if = true;
        options.condition_parens = hspfmt::Parentheses::Add;
        expect("if a=b { x=c!d }\nif a=b : a+\n", "if (a == b) : x = c != d\nif (a == b) : a++\n", options);
        options.line_width = 23;
        expect("if a=b { x=c!d }\n", "if (a == b) { x = c != d }\n", options);
        options = {};
        options.encoding = hspfmt::Encoding::Cp932;
        options.operator_style = hspfmt::OperatorStyle::C;
        options.increment_style = hspfmt::OperatorStyle::C;
        expect("\xef\xbb\xbf" "a=b=c\r\na+", "\xef\xbb\xbf" "a = b == c\r\na++", options);
        expect("mes \"\x95\x5c\" ; ==\r\na+\r\n", "mes \"\x95\x5c\" ; ==\r\na++\r\n", options);
        options = {};
        options.blank_lines_before_module = 17;
        rejects("#module m\n", options);
        options.blank_lines_before_module = -2;
        rejects("#module m\n", options);
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
