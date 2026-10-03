#include "hspfmt.h"
#include <iostream>
#include <stdexcept>
#include <utility>

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
void rejects_at(const std::string &input, std::size_t line, hspfmt::Options options = {}) {
    ++count;
    try { hspfmt::format(input, options); } catch (const hspfmt::Error &error) {
        if (error.line() == line) return;
        throw std::runtime_error("expected rejection at line " + std::to_string(line) + ": " + error.what());
    }
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
            expect("#deffunc " + sum + " str " + kanji + ",int " + hiragana + "\r\nreturn\r\n; " + table +
                   "\r\n#defcfunc next int " + kanji + ",str " + hiragana,
                   "#deffunc " + sum + " str " + kanji + ", int " + hiragana + "\r\n    return\r\n; " + table +
                   "\r\n#defcfunc next int " + kanji + ", str " + hiragana, japanese);
            expect("#deffunc f\r\nreturn\r\n; " + table + "\r\n*" + kanji,
                   "#deffunc f\r\n    return\r\n; " + table + "\r\n*" + kanji, japanese);
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
            japanese.operator_spacing = hspfmt::Spacing::Compact;
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
            expect("; hspfmt: ignore\n" + kanji + full_space + "=  1\n" + hiragana + "=2\n",
                   "; hspfmt: ignore\n" + kanji + full_space + "=  1\n" + hiragana + " = 2\n", normalized);
            expect(full_space + "; hspfmt: ignore\nmes" + full_space + kanji + "\nmes" + full_space + hiragana + "\n",
                   " ; hspfmt: ignore\nmes" + full_space + kanji + "\nmes " + hiragana + "\n", normalized);
            expect("; hspfmt: ignore\nmes" + full_space + "1,\\\n" + full_space + "2,\\\n" + full_space + "3\nmes" + full_space + kanji + "\n",
                   "; hspfmt: ignore\nmes" + full_space + "1,\\\n" + full_space + "2,\\\n" + full_space + "3\nmes " + kanji + "\n", normalized);
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
        // A bare global qualifier (name@) is a name like name@module.
        diagnostic_cases("foo@ *bar\nfoo@*bar\n", "foo@ *bar\nfoo@*bar\n", {{1, "foo@ *bar"}, {2, "foo@*bar"}});
        expect("x@=1\nx@ += 1\nx@=a*b\nmes x@, y@\n", "x@ = 1\nx@ += 1\nx@ = a * b\nmes x@, y@\n");
        {
            hspfmt::Options styled;
            styled.operator_style = hspfmt::OperatorStyle::C;
            styled.increment_style = hspfmt::OperatorStyle::C;
            expect("x@=a=b\nx@+\nx@(1)=a=b\n", "x@ = a == b\nx@++\nx@(1) = a == b\n", styled);
            styled.operator_spacing = hspfmt::Spacing::Compact;
            expect("x@ = 1\nx@ += 1\n", "x@=1\nx@+=1\n", styled);
        }
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
        expect("a=x+(y-1)\na=(x)+(y)\na=-(y-1)\na=f(x)+(g(y))\na=(x)*(y)\n",
               "a = x + (y - 1)\na = (x) + (y)\na = -(y - 1)\na = f(x) + (g(y))\na = (x) * (y)\n");
        expect("a=x+[y]\na=f (x)\na=b (y)\na=+(y)\n",
               "a = x + [y]\na = f (x)\na = b (y)\na = +(y)\n");
        expect("#deffunc f str p_path,int p_access ,\tlocal p_tmp ; keep , comma\nreturn\n"
               "#defcfunc local g double p_x,\tint p_y\nreturn p_x+(p_y-1)\n",
               "#deffunc f str p_path, int p_access, local p_tmp ; keep , comma\n    return\n"
               "#defcfunc local g double p_x, int p_y\n    return p_x + (p_y - 1)\n");
        expect("\t#DeFfUnC  f  VAR p_a ,ARRAY p_b,LABEL p_c  // keep\r\nreturn\r\n",
               "\t#DeFfUnC  f  VAR p_a, ARRAY p_b, LABEL p_c  // keep\r\n    return\r\n");
        for (const std::string declaration : {
                 "#define f(%1,%2) %1,%2", "#module m p_a,p_b", "#modfunc f int p_a,int p_b",
                 "#deffunc prep f int p_a,int p_b", "#deffunc f PARAMS",
                 "#deffunc f custom p_a,int p_b", "#deffunc f int p_a,,int p_b",
                 "#deffunc f int p_a,", "#deffunc f int p_a,2",
                 "#deffunc f int p_a /* middle */,int p_b", "#deffunc f int p_a@mod,int p_b",
                 "#deffunc f onexit"}) {
            expect(declaration + "\n", declaration + "\n");
        }
        expect("#deffunc f int p_a,\\\n int p_b\nreturn\n",
               "#deffunc f int p_a,\\\n int p_b\nreturn\n");
        expect("; hspfmt: ignore\n#deffunc f int p_a,int p_b\nreturn\n",
               "; hspfmt: ignore\n#deffunc f int p_a,int p_b\n    return\n");
        expect("; hspfmt: off\n#deffunc f int p_a,int p_b\n; hspfmt: on\n",
               "; hspfmt: off\n#deffunc f int p_a,int p_b\n; hspfmt: on\n");
        // cHSP scopes explicitly close functions and reset the following HSP code.
        expect("#chsp_module \"native\" target=c\n"
               "#chsp_defcfunc sum array[int] a,int n,local[int] v -> int\n"
               "v=0\nrepeat n\nif a(cnt)>0 {\nv+=a(cnt)\n}\nloop\nreturn v\n"
               "#chsp_end\n; module comment\n#chsp_module_end\nx=sum(a,2)\n",
               "#chsp_module \"native\" target=c\n"
               "#chsp_defcfunc sum array[int] a, int n, local[int] v -> int\n"
               "    v = 0\n    repeat n\n        if a(cnt) > 0 {\n            v += a(cnt)\n"
               "        }\n    loop\n    return v\n"
               "#chsp_end\n; module comment\n#chsp_module_end\nx = sum(a, 2)\n");
        {
            const std::string start = "#chsp_module\n";
            const std::string finish = "\n#chsp_end\n#chsp_module_end\n";
            for (const auto &signature : {
                     std::pair<std::string, std::string>{
                         "#chsp_deffunc f int64 x,label cb,local[int64[2][3][4][5]] v -> void",
                         "#chsp_deffunc f int64 x, label cb, local[int64[2][3][4][5]] v -> void"},
                     {"#CHSP_DEFCFUNC f str s,local[str] v -> str ; keep  comment  ",
                      "#CHSP_DEFCFUNC f str s, local[str] v -> str ; keep  comment  "},
                     {"#chsp_defcfunc f array[double] a,local[double[8]] v -> double",
                      "#chsp_defcfunc f array[double] a, local[double[8]] v -> double"},
                     {"#chsp_defcfunc f -> int64", "#chsp_defcfunc f -> int64"},
                     {"#chsp_deffunc f", "#chsp_deffunc f"}}) {
                expect(start + signature.first + finish, start + signature.second + finish);
            }
            const std::string signature = "#chsp_defcfunc f int x ,\tlocal[int[2][3]] v -> int";
            hspfmt::Options spacing;
            spacing.comma_spacing = hspfmt::Spacing::Compact;
            expect(start + signature + finish,
                   start + "#chsp_defcfunc f int x,local[int[2][3]] v -> int" + finish, spacing);
            spacing.comma_spacing = hspfmt::Spacing::Preserve;
            expect(start + signature + finish, start + signature + finish, spacing);
            // Macro-dependent or unfamiliar signatures remain opaque.
            for (const std::string declaration : {
                     "#chsp_defcfunc f PARAMS -> int", "#chsp_defcfunc f var a,int b -> int",
                     "#chsp_defcfunc f local[int[N]] a,int b -> int",
                     "#chsp_defcfunc f local[int[2][3][4][5][6]] a,int b -> int",
                     "#chsp_defcfunc f int a,int b -> custom", "#chsp_defcfunc f int a,int b",
                     "#chsp_deffunc f int a,", "#chsp_deffunc f int a /* keep */,int b",
                     "#chsp_deffunc f int a,int b -> int"}) {
                expect(start + declaration + finish, start + declaration + finish);
            }
            expect("#chsp_module \\\n target=plugin\n#chsp_defcfunc f \\\n int a,\\\n int b -> int\n"
                   "return a+b\n#chsp_end\n#chsp_module_end\nx=1\n",
                   "#chsp_module \\\n target=plugin\n#chsp_defcfunc f \\\n int a,\\\n int b -> int\n"
                   "    return a + b\n#chsp_end\n#chsp_module_end\nx = 1\n");
            expect(start + "; hspfmt: ignore\n  #chsp_deffunc f int a,int b\nreturn\n" + finish,
                   start + "; hspfmt: ignore\n  #chsp_deffunc f int a,int b\n    return\n" + finish);
            const std::string disabled = "; hspfmt: off\n" + start + signature + finish + "; hspfmt: on\n";
            expect(disabled + "x=1\n", disabled + "x = 1\n");
            // C text, its indentation, and HSP-looking tokens must stay intact.
            const std::string embedded = "#chsp_c {\"\r\n  // hspfmt: off\r\n"
                "static int inc(int x) { return x+1; }\r\n  /* repeat : #chsp_end */\r\n\"}\r\n"
                "#chsp_cdecl inc\r\n#chsp_clink \"dl\"\r\n";
            spacing = {};
            spacing.operator_style = hspfmt::OperatorStyle::Hsp;
            spacing.comment_style = hspfmt::CommentStyle::Semicolon;
            spacing.block_comments = hspfmt::BlockComments::Lines;
            expect("\xef\xbb\xbf#chsp_module \"native\"\r\n" + embedded +
                   "#chsp_defcfunc f int x -> int\r\nreturn inc(x+1)\r\n#chsp_end\r\n#chsp_module_end",
                   "\xef\xbb\xbf#chsp_module \"native\"\r\n" + embedded +
                   "#chsp_defcfunc f int x -> int\r\n    return inc(x + 1)\r\n#chsp_end\r\n#chsp_module_end", spacing);
            spacing = {};
            spacing.tabs = true;
            spacing.base_indent = 2;
            expect(start + "#chsp_deffunc f\nrepeat 2\nx=1\nloop\n#chsp_end\nx=2\n#chsp_module_end\n",
                   start + "#chsp_deffunc f\n\t\trepeat 2\n\t\t\tx = 1\n\t\tloop\n#chsp_end\n\t\tx = 2\n#chsp_module_end\n", spacing);
            spacing.preserve_indent = true;
            expect(start + "#chsp_deffunc f\n  return\n#chsp_end\n#chsp_module_end\n",
                   start + "#chsp_deffunc f\n  return\n#chsp_end\n#chsp_module_end\n", spacing);
            spacing = {};
            spacing.blank_lines_before_module = 2;
            spacing.blank_lines_before_deffunc = 1;
            spacing.blank_lines_before_defcfunc = 1;
            expect("x=1\n  ; module docs\n#chsp_module\n  ; function docs\n#chsp_deffunc f\nreturn\n"
                   "#chsp_end\n  ; result docs\n#chsp_defcfunc g -> int\nreturn 1\n#chsp_end\n#chsp_module_end\n",
                   "x = 1\n\n\n; module docs\n#chsp_module\n\n; function docs\n#chsp_deffunc f\n    return\n"
                   "#chsp_end\n\n; result docs\n#chsp_defcfunc g -> int\n    return 1\n#chsp_end\n#chsp_module_end\n", spacing);
            // Ordinary HSP functions can coexist with native declarations.
            expect(start + "#deffunc helper\nreturn\n#chsp_deffunc f\nreturn\n" + finish + "x=1\n",
                   start + "#deffunc helper\n    return\n#chsp_deffunc f\n    return\n" + finish + "x = 1\n");
        }
        expect("#chsp_module\n#if FLAG\n#chsp_deffunc f int a\n#else\n#chsp_deffunc f double a\n#endif\n"
               "return\n#chsp_end\n#chsp_module_end\n",
               "#chsp_module\n#if FLAG\n#chsp_deffunc f int a\n#else\n#chsp_deffunc f double a\n#endif\n"
               "    return\n#chsp_end\n#chsp_module_end\n");
        rejects_at("#chsp_end\n", 1);
        rejects_at("#chsp_module_end\n", 1);
        rejects_at("#chsp_deffunc f\n", 1);
        rejects_at("#chsp_module\n#chsp_module\n", 2);
        rejects_at("#chsp_module\n#chsp_deffunc f\n#chsp_defcfunc g -> int\n", 3);
        rejects_at("#chsp_module\n#chsp_deffunc f\n#chsp_module_end\n", 3);
        rejects_at("#chsp_module\n#chsp_deffunc f\nrepeat 2\n#chsp_end\n", 4);
        rejects_at("#chsp_module\n#chsp_deffunc f\n#deffunc g\n", 3);
        rejects_at("#chsp_module\n#chsp_deffunc f\n#global\n", 3);
        rejects_at("#chsp_module\n#if FLAG\n#chsp_deffunc f\n#endif\n", 4);
        rejects_at("#if FLAG\n#chsp_module\n#else\nx=1\n#endif\n", 5);
        rejects_at("\n#chsp_module\n", 2);
        rejects_at("#chsp_module\n\n#chsp_defcfunc f -> int\nreturn 1\n", 3);
        expect("#deffunc f\nreturn\n; docs for g\n// second line\n#defcfunc g\nreturn 1\n",
               "#deffunc f\n    return\n; docs for g\n// second line\n#defcfunc g\n    return 1\n");
        expect("#deffunc f\nreturn\n  /* docs\n    keep interior */\n#deffunc g\nreturn\n",
               "#deffunc f\n    return\n/* docs\n    keep interior */\n#deffunc g\n    return\n");
        expect("#deffunc f\nreturn\n; body comment\n\n#deffunc g\nreturn\n",
               "#deffunc f\n    return\n    ; body comment\n\n#deffunc g\n    return\n");
        expect("#deffunc f\nreturn\n; hspfmt: ignore\n  ; keep docs\n#deffunc g\nreturn\n",
               "#deffunc f\n    return\n; hspfmt: ignore\n  ; keep docs\n#deffunc g\n    return\n");
        expect("\xef\xbb\xbf" "  ; docs\r\n#defcfunc f int p_a,int p_b",
               "\xef\xbb\xbf" "; docs\r\n#defcfunc f int p_a, int p_b");
        {
            hspfmt::Options docs;
            docs.tabs = true;
            docs.base_indent = 2;
            expect("#deffunc f\nreturn\n; docs\n\t#modfunc g\nreturn\n",
                   "#deffunc f\n\t\treturn\n\t; docs\n\t#modfunc g\n\t\treturn\n", docs);
            docs.preserve_indent = true;
            expect("#deffunc f\n\treturn\n\t  ; docs\n#deffunc g int p_a,int p_b\n",
                   "#deffunc f\n\treturn\n\t  ; docs\n#deffunc g int p_a, int p_b\n", docs);
            docs.blank_lines_before_deffunc = 1;
            expect("#deffunc f\n\treturn\n\t  ; docs\n#deffunc g\n",
                   "#deffunc f\n\treturn\n\n\t  ; docs\n#deffunc g\n", docs);
            docs = {};
            docs.blank_lines_before_deffunc = 1;
            docs.block_comments = hspfmt::BlockComments::Lines;
            expect("#deffunc f\nreturn\n  /* docs\nmore */\n#deffunc g\nreturn\n",
                   "#deffunc f\n    return\n\n; docs\n;more \n#deffunc g\n    return\n", docs);
            docs.block_comments = hspfmt::BlockComments::Block;
            expect("#deffunc f\nreturn\n; docs\n; more\n#deffunc g\nreturn\n",
                   "#deffunc f\n    return\n\n/* docs\n more*/\n#deffunc g\n    return\n", docs);
        }
        expect("*main\ngoto *main\na=a*2+-1\n", "*main\ngoto *main\na = a * 2 + -1\n");
        {
            hspfmt::Options labels;
            labels.indent_labels = true;
            expect("*first\nx=1\n; docs\n// second line\n  *second\nx=2\n",
                   "*first\n    x = 1\n; docs\n// second line\n*second\n    x = 2\n", labels);
            expect("*first\nx=1\n  /* docs\n    keep interior */\n*second\nx=2\n",
                   "*first\n    x = 1\n/* docs\n    keep interior */\n*second\n    x = 2\n", labels);
            expect("*first\nx=1\n; body comment\n\n*second\nx=2\n",
                   "*first\n    x = 1\n    ; body comment\n\n*second\n    x = 2\n", labels);
            expect("*first\n; operand comment\nx=*second\n",
                   "*first\n    ; operand comment\n    x = *second\n", labels);
            expect("*first\n; hspfmt: ignore\n  ; keep docs\n*second\n",
                   "*first\n; hspfmt: ignore\n  ; keep docs\n*second\n", labels);
            expect("*first\n; docs\n; hspfmt: ignore\n  *second\n",
                   "*first\n    ; docs\n; hspfmt: ignore\n  *second\n", labels);
            expect("; hspfmt: off\n  ; docs\n  *second\n; hspfmt: on\n",
                   "; hspfmt: off\n  ; docs\n  *second\n; hspfmt: on\n", labels);
            expect("\xef\xbb\xbf" "  ; docs\r\n  *first", "\xef\xbb\xbf" "; docs\r\n*first", labels);
            labels.tabs = true;
            labels.base_indent = 2;
            expect("*first\n; docs\n*second\nx=2\n",
                   "*first\n; docs\n*second\n\t\tx = 2\n", labels);
            labels.preserve_indent = true;
            labels.blank_lines_before_deffunc = 1;
            expect("*first\n\t  ; docs\n\t*second\n",
                   "*first\n\t  ; docs\n\t*second\n", labels);
            labels = {};
            labels.base_indent = 1;
            labels.block_comments = hspfmt::BlockComments::Lines;
            expect("/* docs\nmore */\n*first\nx=1\n",
                   "; docs\n;more \n*first\n    x = 1\n", labels);
        }
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
                label_options.operator_spacing = compact ? hspfmt::Spacing::Compact : hspfmt::Spacing::Space;
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
        expect("; hspfmt: ignore\n\t a    = 1  \nx=2\n",
               "; hspfmt: ignore\n\t a    = 1  \nx = 2\n");
        expect("\xef\xbb\xbf; hspfmt: ignore\r\n a   =1\r\nx=2",
               "\xef\xbb\xbf; hspfmt: ignore\r\n a   =1\r\nx = 2");
        expect("; hspfmt: ignore\na   =1", "; hspfmt: ignore\na   =1");
        expect("; hspfmt: ignore", "; hspfmt: ignore");
        // Trailing blanks after a marker must not silently disable it.
        expect("; hspfmt: off \t\nx=1\n; hspfmt: on \r\nx=2\n; hspfmt: ignore  \ny  =1\nz=1\n",
               "; hspfmt: off \t\nx=1\n; hspfmt: on \r\nx = 2\n; hspfmt: ignore  \ny  =1\nz = 1\n");
        {
            hspfmt::Options normalized;
            normalized.full_width_spaces = hspfmt::FullWidthSpaces::Normalize;
            expect("; hspfmt: off \nmes\xe3\x80\x80" "1\n; hspfmt: on \nmes\xe3\x80\x80" "1\n",
                   "; hspfmt: off \nmes\xe3\x80\x80" "1\n; hspfmt: on \nmes 1\n", normalized);
        }
        {
            // Assignment operators cannot start a command argument, so compact spacing applies.
            hspfmt::Options compact;
            compact.operator_spacing = hspfmt::Spacing::Compact;
            expect("a += 1\nb+=1\nc -= d * 2\nd \\= 2\nfoo -1\na +2\nx = 1\n",
                   "a+=1\nb+=1\nc-=d*2\nd\\=2\nfoo -1\na +2\nx=1\n", compact);
        }
        expect("; hspfmt: ignore\n  \nx=1\n", "; hspfmt: ignore\n  \nx = 1\n");
        expect("; hspfmt: ignore\n  ; kept\nx=1\n", "; hspfmt: ignore\n  ; kept\nx = 1\n");
        expect("; hspfmt: ignore\n; hspfmt: ignore\nx=1\ny=2\n",
               "; hspfmt: ignore\n; hspfmt: ignore\nx=1\ny = 2\n");
        expect("; hspfmt: off\n; hspfmt: ignore\nx=1\ny=2\n; hspfmt: on\nz=3\n",
               "; hspfmt: off\n; hspfmt: ignore\nx=1\ny=2\n; hspfmt: on\nz = 3\n");
        expect("mes \"; hspfmt: ignore\"\na=1 ; hspfmt: ignore\nb=2\n"
               "// hspfmt: ignore\nc=3\n/* hspfmt: ignore */\nd=4\n",
               "mes \"; hspfmt: ignore\"\na = 1 ; hspfmt: ignore\nb = 2\n"
               "// hspfmt: ignore\nc=3\n/* hspfmt: ignore */\nd = 4\n");
        // Both line comment markers are accepted and can be mixed.
        expect("// hspfmt: off\nx=1\n; hspfmt: on\ny=2\n; hspfmt: off\nz=3\n// hspfmt: on \nw=4\n",
               "// hspfmt: off\nx=1\n; hspfmt: on\ny = 2\n; hspfmt: off\nz=3\n// hspfmt: on \nw = 4\n");
        expect("//hspfmt: off\nx=1\n;hspfmt: ignore\ny=2\n", "//hspfmt: off\nx = 1\n;hspfmt: ignore\ny = 2\n");
        expect("; hspfmt: ignore\n repeat  2\nx=1\n; hspfmt: ignore\n  loop\ny=2\n",
               "; hspfmt: ignore\n repeat  2\n    x = 1\n; hspfmt: ignore\n  loop\ny = 2\n");
        expect("; hspfmt: ignore\n if a {\nx=1\n; hspfmt: ignore\n  }\ny=2\n",
               "; hspfmt: ignore\n if a {\n    x = 1\n; hspfmt: ignore\n  }\ny = 2\n");
        expect("; hspfmt: ignore\ns={\"\n; hspfmt: ignore\n\"}\nx=1\n",
               "; hspfmt: ignore\ns={\"\n; hspfmt: ignore\n\"}\nx = 1\n");
        diagnostic_cases("; hspfmt: ignore\nfoo*bar\nx=1\n",
                         "; hspfmt: ignore\nfoo*bar\nx = 1\n", {});
        hspfmt::Options ignored_options;
        ignored_options.comment_style = hspfmt::CommentStyle::C;
        ignored_options.block_comments = hspfmt::BlockComments::Lines;
        ignored_options.operator_style = hspfmt::OperatorStyle::C;
        ignored_options.increment_style = hspfmt::OperatorStyle::C;
        ignored_options.condition_parens = hspfmt::Parentheses::Add;
        ignored_options.repeat_parens = hspfmt::Parentheses::Add;
        ignored_options.hsp_numeric_prefixes = true;
        ignored_options.short_if = true;
        ignored_options.blank_lines_before_module = 2;
        expect("; hspfmt: ignore\nif a=0x1 { x+ } ; kept\n"
               "; hspfmt: ignore\nrepeat 2\nx+\nloop\n"
               "; hspfmt: ignore\n/* kept\n block */\n"
               "; hspfmt: ignore\n#module m\n#global\nx=0x1 ; changed\n",
               "; hspfmt: ignore\nif a=0x1 { x+ } ; kept\n"
               "; hspfmt: ignore\nrepeat 2\n    x++\nloop\n"
               "; hspfmt: ignore\n/* kept\n block */\n"
               "; hspfmt: ignore\n#module m\n#global\nx = $1 // changed\n", ignored_options);
        ignored_options.block_comments = hspfmt::BlockComments::Block;
        expect("; before\n; hspfmt: ignore\n; kept\n; after\n",
               "/* before*/\n; hspfmt: ignore\n; kept\n/* after*/\n", ignored_options);
        rejects("; hspfmt: ignore\nrepeat\n");
        hspfmt::Options options;
        options.hsp_numeric_prefixes = true;
        expect("a=0x00Ff:b=0b0010:s=\"0xff\"\n", "a = $00Ff : b = %0010 : s = \"0xff\"\n", options);
        expect("a=0b_0010:b=0b__10_01\n", "a = %_0010 : b = %__10_01\n", options);
        options = {};
        options.short_if = true;
        expect("if (flag) { foo=bar:baz=1 }\n", "if flag : foo = bar : baz = 1\n", options);
        expect("if (flag) { foo=bar } : baz=1\n", "if (flag) { foo = bar } : baz = 1\n", options);
        expect("if a { if b { x=1 } }\n", "if a { if b { x = 1 } }\n", options);
        expect("if a { mes 1 }\n", "if a : mes 1\n", options);
        expect("if a { x=1 } ; note\n", "if a { x = 1 } ; note\n", options);
        expect("if a { x=1 }\nelse { x=2 }\n", "if a { x = 1 }\nelse { x = 2 }\n", options);
        expect("if a { x=1 }\r\n  ELSE { x=2 }",
               "if a { x = 1 }\r\nELSE { x = 2 }", options);
        expect("if a { x=1 }\n/* note */ else { x=2 }\n",
               "if a { x = 1 }\n/* note */ else { x = 2 }\n", options);
        expect("if a {\nif b { x=1 }\nelse { x=2 }\n}\n",
               "if a {\n    if b { x = 1 }\n    else { x = 2 }\n}\n", options);
        expect("if a { x=1 }\n; note\ny=2\n", "if a : x = 1\n; note\ny = 2\n", options);
        expect("if e=0 { uy-- }\nif flag { x+=2:y++ }\nif z=0 { return }\n",
               "if e = 0 : uy--\nif flag : x += 2 : y++\nif z = 0 : return\n", options);
        expect("if z=0 {\n  return\n}\nif flag {\nx=1\n}\n",
               "if z = 0 : return\nif flag : x = 1\n", options);
        expect("if z {\nreturn 2+3\n}\n", "if z : return 2 + 3\n", options);
        diagnostic_cases("if z {\nreturn\n}\nfoo*bar\n", "if z : return\nfoo*bar\n",
                         {{4, "foo*bar"}}, options);
        expect("if flag {\nx=1\ny=2\n}\n", "if flag {\n    x = 1\n    y = 2\n}\n", options);
        expect("if flag {\nmes 1\n}\n", "if flag : mes 1\n", options);
        expect("if flag {\nx(0)=1\n}\n", "if flag : x(0) = 1\n", options);
        expect("if flag { custom }\nif flag { custom@mod x,2 }\n",
               "if flag : custom\nif flag : custom@mod x, 2\n", options);
        expect("if flag { x(0)+=2:x.1++:mes f(1,2):gosub *done }\n",
               "if flag : x(0) += 2 : x.1++ : mes f(1, 2) : gosub *done\n", options);
        expect("if flag {\ngoto *done\n}\n", "if flag : goto *done\n", options);
        expect("if flag {\ncustom a,,b\n}\n", "if flag : custom a,, b\n", options);
        expect("if flag {\ncustom 1 : mes 2\n}\n", "if flag : custom 1 : mes 2\n", options);
        expect("if a { if b : x=1 }\n", "if a { if b : x = 1 }\n", options);
        expect("if a {\nif b : x=1\n}\n", "if a {\n    if b : x = 1\n}\n", options);
        expect("if a { mes 1 : if b : x=1 }\n", "if a { mes 1 : if b : x = 1 }\n", options);
        expect("if a { repeat 1 : x=1 : loop }\n", "if a { repeat 1 : x = 1 : loop }\n", options);
        expect("if a { while b : x=1 : wend }\n", "if a { while b : x = 1 : wend }\n", options);
        expect("if a { for i,0,2 : x=1 : next }\n", "if a { for i, 0, 2 : x = 1 : next }\n", options);
        expect("if a { do : x=1 : until b }\n", "if a { do : x = 1 : until b }\n", options);
        expect("if a { foreach x : mes x(cnt) : loop }\n",
               "if a { foreach x : mes x(cnt) : loop }\n", options);
        expect("if a { switch x : case 1 : mes 1 : swbreak : default : mes 2 : swend }\n",
               "if a { switch x : case 1 : mes 1 : swbreak : default : mes 2 : swend }\n", options);
        expect("if a {\n*label\n}\n", "if a {\n*label\n}\n", options);
        expect("if a {\n#define value 1\n}\n", "if a {\n#define value 1\n}\n", options);
        expect("if a { custom 1 }\nelse { custom 2 }\n",
               "if a { custom 1 }\nelse { custom 2 }\n", options);
        expect("if a {\ncustom 1\\\n,2\n}\n", "if a {\ncustom 1\\\n,2\n}\n", options);
        expect("if flag {\nreturn\n} else {\nx=1\n}\n",
               "if flag {\n    return\n} else {\n    x = 1\n}\n", options);
        expect("if flag {\nreturn\n}\n; note\nelse { x=1 }\n",
               "if flag {\n    return\n}\n; note\nelse { x = 1 }\n", options);
        expect("if flag {\nreturn ; note\n}\n", "if flag {\n    return ; note\n}\n", options);
        expect("if flag {\n; note\nreturn\n}\n", "if flag {\n    ; note\n    return\n}\n", options);
        expect("if flag {\n\nreturn\n}\n", "if flag {\n\n    return\n}\n", options);
        expect("; hspfmt: ignore\nif flag {\nreturn\n}\n",
               "; hspfmt: ignore\nif flag {\n    return\n}\n", options);
        expect("if flag {\n; hspfmt: ignore\nreturn\n}\n",
               "if flag {\n; hspfmt: ignore\nreturn\n}\n", options);
        expect("if flag {\nreturn\n; hspfmt: ignore\n}\n",
               "if flag {\n    return\n; hspfmt: ignore\n}\n", options);
        expect("; hspfmt: off\nif flag {\n  return\n}\n; hspfmt: on\n",
               "; hspfmt: off\nif flag {\n  return\n}\n; hspfmt: on\n", options);
        options.condition_parens = hspfmt::Parentheses::Remove;
        expect("if (e=0) { uy-- }\nif ((z=0)) {\n  return\n}\n",
               "if e = 0 : uy--\nif z = 0 : return\n", options);
        expect("\xef\xbb\xbfif (z=0) {\r\nreturn\r\n}", "\xef\xbb\xbfif z = 0 : return", options);
        expect("if ((a)+(b)) {\nreturn\n}\n", "if (a) + (b) : return\n", options);
        options.condition_parens = hspfmt::Parentheses::Add;
        expect("if e=0 { uy-- }\nif z=0 {\nreturn\n}\n",
               "if (e = 0) : uy--\nif (z = 0) : return\n", options);
        options = {};
        options.short_if = true;
        options.tabs = true;
        expect("#deffunc f\nif z=0 {\nreturn\n}\n", "#deffunc f\n\tif z = 0 : return\n", options);
        options.preserve_indent = true;
        expect("\t  if flag {\n  return\n }\n", "\t  if flag : return\n", options);
        options = {};
        options.short_if = true;
        options.condition_parens = hspfmt::Parentheses::Remove;
        options.line_width = 16;
        expect("if (flag) {\nreturn\n}\n", "if flag : return\n", options);
        options.line_width = 15;
        expect("if (flag) {\nreturn\n}\n", "if flag {\n    return\n}\n", options);
        options = {};
        options.short_if = true;
        options.line_width = 10;
        expect("if (flag) { x=1 }\n", "if (flag) { x = 1 }\n", options);
        options = {};
        options.tabs = true;
        expect("repeat\na=1\nloop\n", "repeat\n\ta = 1\nloop\n", options);
        options.base_indent = 1;
        expect("x=1\n*main\nrepeat\nx=2\nloop\n#module m\n#deffunc f\nx=3\n#global\nx=4\n",
               "\tx = 1\n*main\n\trepeat\n\t\tx = 2\n\tloop\n#module m\n#deffunc f\n\tx = 3\n#global\n\tx = 4\n", options);
        options.loop_indent = 0;
        expect("repeat 2\nx=1\nif flag {\nx=2\n}\nloop\n",
               "\trepeat 2\n\tx = 1\n\tif flag {\n\t\tx = 2\n\t}\n\tloop\n", options);
        expect("while n\nfor i,0,3\ndo\nforeach a\nx=1\nloop\nuntil flag\nnext\nwend\n",
               "\twhile n\n\tfor i, 0, 3\n\tdo\n\tforeach a\n\tx = 1\n\tloop\n\tuntil flag\n\tnext\n\twend\n", options);
        expect("switch x\ncase 1\nrepeat 2\nx=1\nloop\nswbreak\ndefault\nx=2\nswend\n",
               "\tswitch x\n\t\tcase 1\n\t\t\trepeat 2\n\t\t\tx = 1\n\t\t\tloop\n\t\t\tswbreak\n\t\tdefault\n\t\t\tx = 2\n\tswend\n", options);
        expect("#if A\nrepeat 2\n#else\nrepeat 3\n#endif\nx=1\nloop\n",
               "#if A\n\trepeat 2\n#else\n\trepeat 3\n#endif\n\tx = 1\n\tloop\n", options);
        rejects("repeat\n", options);
        rejects("loop\n", options);
        options = {};
        options.indent_width = 2;
        options.base_indent = 2;
        options.loop_indent = 2;
        expect("x=1\nrepeat\nx=2\nloop\n", "    x = 1\n    repeat\n        x = 2\n    loop\n", options);
        options.preserve_indent = true;
        expect("\tx=1\n repeat\nx=2\n loop\n", "\tx = 1\n repeat\nx = 2\n loop\n", options);
        options = {};
        options.base_indent = 16;
        expect("x=1\n", std::string(64, ' ') + "x = 1\n", options);
        options.base_indent = 17;
        rejects("x=1\n", options);
        options = {};
        options.loop_indent = 16;
        expect("repeat\nx=1\nloop\n", "repeat\n" + std::string(64, ' ') + "x = 1\nloop\n", options);
        options.loop_indent = 17;
        rejects("x=1\n", options);
        options = {};
        options.tabs = true;
        options.preserve_indent = true;
        options.indent_labels = true;
        expect("\t*main\n\trepeat 2\n \t a=1\n\tloop\n\t\t; aligned\n",
               "\t*main\n\trepeat 2\n \t a = 1\n\tloop\n\t\t; aligned\n", options);
        expect("#module m\n#deffunc f\n\t  x=1\n#global\n  x=2\n",
               "#module m\n#deffunc f\n\t  x = 1\n#global\n  x = 2\n", options);
        expect("\xef\xbb\xbf\t x=1\r\n\tmes x", "\xef\xbb\xbf\t x = 1\r\n\tmes x", options);
        rejects("\trepeat\n", options);
        options.short_if = true;
        options.line_width = 18;
        expect("\t  if flag { x=1 }\n", "\t  if flag : x = 1\n", options);
        options.line_width = 17;
        expect("\t  if flag { x=1 }\n", "\t  if flag { x = 1 }\n", options);
        options = {};
        options.operator_spacing = hspfmt::Spacing::Preserve;
        expect("a=x+(y-1)\na = x +  (y)\na=x\t+\t(y)\n",
               "a=x+(y-1)\na = x +  (y)\na=x\t+\t(y)\n", options);
        expect("a=1\nb = 2+3 * 4\nc\t=\t5 +6\nx=a - -1\n",
               "a=1\nb = 2+3 * 4\nc\t=\t5 +6\nx=a - -1\n", options);
        diagnostic_cases("foo*bar\nfoo\t* bar\n", "foo*bar\nfoo\t* bar\n", {}, options);
        options.operator_style = hspfmt::OperatorStyle::C;
        expect("x=a=b & c!d\n", "x=a==b && c!=d\n", options);
        options = {};
        options.comma_spacing = hspfmt::Spacing::Preserve;
        expect("#deffunc f str p_path,int p_access ,\tlocal p_tmp\n",
               "#deffunc f str p_path,int p_access ,\tlocal p_tmp\n", options);
        expect("mes 1,, 3 ,4\nx=f(1,\t2 ,3)\n",
               "mes 1,, 3 ,4\nx = f(1,\t2 ,3)\n", options);
        options.comma_spacing = hspfmt::Spacing::Compact;
        expect("#defcfunc local f int p_a , int p_b  ; keep\n",
               "#defcfunc local f int p_a,int p_b  ; keep\n", options);
        expect("mes -1, -2, *label\nx=f(1 , 2)\n", "mes -1,-2,*label\nx = f(1,2)\n", options);
        options = {};
        options.colon_spacing = hspfmt::Spacing::Preserve;
        expect("a=1:b=2 :c=3\nmes 1\t:\tmes 2\n",
               "a = 1:b = 2 :c = 3\nmes 1\t:\tmes 2\n", options);
        options.colon_spacing = hspfmt::Spacing::Compact;
        expect("redraw : boxf\na=1 : b=2\n", "redraw:boxf\na = 1:b = 2\n", options);
        options = {};
        options.comment_spacing = hspfmt::Spacing::Preserve;
        expect("a=1\t\t; aligned\nmes 1// adjacent\nx=2 /* block */\n",
               "a = 1\t\t; aligned\nmes 1// adjacent\nx = 2 /* block */\n", options);
        options.comment_style = hspfmt::CommentStyle::C;
        expect("a=1\t\t; aligned\n", "a = 1\t\t// aligned\n", options);
        options.comment_spacing = hspfmt::Spacing::Compact;
        expect("mes 1 ; tail\nx=a/ /* block */b\n",
               "mes 1// tail\nx = a / /* block */ b\n", options);
        options = {};
        options.preserve_indent = true;
        options.operator_spacing = hspfmt::Spacing::Preserve;
        options.comma_spacing = hspfmt::Spacing::Preserve;
        options.colon_spacing = hspfmt::Spacing::Preserve;
        options.comment_spacing = hspfmt::Spacing::Preserve;
        expect("\tt1=\"\":t2=\"\"\n\tmesbox t1,160,32,0\t\t; aligned\n\t\t\t; continuation\n*tmprt\n\tobjprm 0,t1\t\t; aligned\n",
               "\tt1=\"\":t2=\"\"\n\tmesbox t1,160,32,0\t\t; aligned\n\t\t\t; continuation\n*tmprt\n\tobjprm 0,t1\t\t; aligned\n", options);
        options.encoding = hspfmt::Encoding::Cp932;
        expect("\t\x95\x5c=1:mes \x95\x5c\t; \x95\x5c\r\n",
               "\t\x95\x5c=1:mes \x95\x5c\t; \x95\x5c\r\n", options);
        options = {};
        options.operator_spacing = hspfmt::Spacing::Compact;
        expect("a = x + (y - 1)\na = (x) * (y)\na = - (y)\n",
               "a=x+(y-1)\na=(x)*(y)\na=-(y)\n", options);
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
        // Markers keep their spelling while other comments are converted.
        expect("// hspfmt: off\nx=1 // kept\n// hspfmt: on\nx=2 // tail\n",
               "// hspfmt: off\nx=1 // kept\n// hspfmt: on\nx = 2 ; tail\n", options);
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
        expect("if ((flag)) : x=1\nwhile ((a)+(b))\nx=2\nwend\n", "if flag : x = 1\nwhile (a) + (b)\n    x = 2\nwend\n", options);
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
        options.operator_spacing = hspfmt::Spacing::Compact;
        expect("x=a=b & c! -d\n", "x=a==b&&c!=-d\n", options);
        options = {};
        options.operator_style = hspfmt::OperatorStyle::Hsp;
        expect("a=b\nx=a==b\nif a==b && a!=c || b==c : x=1\n",
               "a = b\nx = a = b\nif a = b & a ! c | b = c : x = 1\n", options);
        expect("x==b\na(0)==b==c\n", "x == b\na(0) == b = c\n", options);
        expect("x=a<=b && a>=b\na+=1\n", "x = a <= b & a >= b\na += 1\n", options);
        options.operator_spacing = hspfmt::Spacing::Compact;
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
        // Errors report the physical line of the failure, or where an unterminated construct starts.
        rejects_at("x=1\r\ny=2\r\nmes \"open\r\n", 3);
        {
            hspfmt::Options utf8_mode;
            utf8_mode.encoding = hspfmt::Encoding::Utf8;
            rejects_at("x=1\r\rmes \"a\xc0\"\n", 3, utf8_mode);
        }
        rejects_at("x=1\r\rmes \"a\xff\"\n", 3);
        rejects_at("s={\"\n\n\"} : wend\n", 1);
        rejects_at("x=1\nrepeat\nwhile 1\nwend\n", 2);
        rejects_at("x=1\n\nloop\n", 3);
        rejects_at("switch a\nswend\ncase 1\n", 3);
        rejects_at("x=1\n#ifdef A\nrepeat\nloop\n", 2);
        rejects_at("repeat\n#if A\nloop\n#else\n#endif\nloop\n", 5);

        // Encoding auto-detection tests (ASCII, UTF-8, CP932, Undetermined)
        if (hspfmt::detect_encoding("a = 1\n") != hspfmt::Encoding::Utf8)
            throw std::runtime_error("ASCII should detect as UTF-8");
        if (hspfmt::detect_encoding("\xef\xbb\xbf" "a = 1\n") != hspfmt::Encoding::Utf8)
            throw std::runtime_error("BOM should detect as UTF-8");
        if (hspfmt::detect_encoding(u8"mes \"日本語\"\n") != hspfmt::Encoding::Utf8)
            throw std::runtime_error("UTF-8 Japanese should detect as UTF-8");
        if (hspfmt::detect_encoding(u8"mes \"\u6f22\u5b57\"\n") != hspfmt::Encoding::Utf8)
            throw std::runtime_error("UTF-8 Kanji should detect as UTF-8");
        if (hspfmt::detect_encoding(u8"\u3000mes 1\n") != hspfmt::Encoding::Utf8)
            throw std::runtime_error("UTF-8 full-width space should detect as UTF-8");
        if (hspfmt::detect_encoding("mes \"\x82\xb1\x82\xf1\x82\xc9\x82\xbf\x82\xcd\"\n") != hspfmt::Encoding::Cp932)
            throw std::runtime_error("CP932 hiragana should detect as CP932");
        if (hspfmt::detect_encoding("mes \"\x8a\xbf\x8e\x9a\"\n") != hspfmt::Encoding::Cp932)
            throw std::runtime_error("CP932 kanji should detect as CP932");
        if (hspfmt::detect_encoding("mes \"\x95\x5c\"\n") != hspfmt::Encoding::Cp932)
            throw std::runtime_error("CP932 table character with 0x5c should detect as CP932");
        if (hspfmt::detect_encoding("mes \"\xb1\xb2\xb3\"\n") != hspfmt::Encoding::Cp932)
            throw std::runtime_error("CP932 half-width kana should detect as CP932");
        if (hspfmt::detect_encoding("\x81\x40mes 1\n") != hspfmt::Encoding::Cp932)
            throw std::runtime_error("CP932 full-width space should detect as CP932");

        // Undetermined rejection: ambiguous or neither valid
        rejects("mes \"\xc3\xa9\"\n"); // ambiguous between UTF-8 (é) and CP932 (ﾃｩ)
        rejects("mes \"\xff\xff\"\n"); // neither valid UTF-8 nor valid CP932
        // Explicit encoding accepts ambiguous input
        {
            hspfmt::Options explicit_u8;
            explicit_u8.encoding = hspfmt::Encoding::Utf8;
            expect("mes \"\xc3\xa9\":a=1\n", "mes \"\xc3\xa9\" : a = 1\n", explicit_u8);
            hspfmt::Options explicit_cp;
            explicit_cp.encoding = hspfmt::Encoding::Cp932;
            expect("mes \"\xc3\xa9\":a=1\n", "mes \"\xc3\xa9\" : a = 1\n", explicit_cp);
        }

        // Format with default auto-encoding
        expect(u8"mes \"日本語\":a=1\n", u8"mes \"日本語\" : a = 1\n");
        expect("mes \"\x95\x5c\":a=1\n", "mes \"\x95\x5c\" : a = 1\n");
        expect("mes \"\x82\xb1\x82\xf1\x82\xc9\x82\xbf\x82\xcd\":a=1\n", "mes \"\x82\xb1\x82\xf1\x82\xc9\x82\xbf\x82\xcd\" : a = 1\n");

        {
            hspfmt::Options norm;
            norm.full_width_spaces = hspfmt::FullWidthSpaces::Normalize;
            expect(std::string(u8"\u3000") + "x=1\n", "x = 1\n", norm);
            expect(std::string("\x81\x40") + "x=1\n", "x = 1\n", norm);
            expect(std::string("mes") + u8"\u3000" + "1\n", "mes 1\n", norm);
            expect(std::string("mes") + "\x81\x40" + "1\n", "mes 1\n", norm);
        }
        std::cout << count << " formatter cases passed\n";
        return 0;
    } catch (const std::exception &error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
