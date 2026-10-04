use hspfmt::config::{find_config, parse_config, parse_formatting_option};
use hspfmt::{format, format_utf8, lex, BlockComments, DiagnosticKind, Encoding, Options};
use std::path::{Path, PathBuf};

#[test]
fn utf8_entry_point_handles_ambiguous_bytes_and_overrides_encoding() {
    let source = "mes \"😀\"\r\n";
    assert!(format(source.as_bytes(), &Options::default(), None).is_err());
    let options = Options {
        encoding: Encoding::Cp932,
        ..Options::default()
    };
    assert_eq!(format_utf8(source, &options, None).unwrap(), source);
}

#[test]
fn errors_distinguish_input_positions_from_option_errors() {
    let error = lex(b";x\r\nmes \"bad\n", Encoding::Utf8).unwrap_err();
    assert_eq!(error.line(), Some(2));
    assert_eq!(error.byte_offset(), Some(8));
    assert_eq!(error.message(), "newline in quoted string");

    let error = format(b"; x\r\n\rrepeat\n", &Options::default(), None).unwrap_err();
    assert_eq!(error.line(), Some(3));
    assert_eq!(error.byte_offset(), Some(6));
    assert_eq!(
        error.to_string(),
        "line 3: unterminated block, expected loop"
    );

    let options = Options {
        indent_width: 17,
        ..Options::default()
    };
    let error = format(b"", &options, None).unwrap_err();
    assert_eq!(error.line(), None);
    assert_eq!(error.byte_offset(), None);
    assert_eq!(error.to_string(), "indent width must be between 0 and 16");
}

#[test]
fn diagnostics_refer_to_original_lines_after_comment_conversion() {
    let source = b"\xef\xbb\xbf/* heading\r\nbody */\r\nfoo*bar\r\n";
    let options = Options {
        block_comments: BlockComments::Lines,
        ..Options::default()
    };
    let mut diagnostics = Vec::new();
    let output = format(source, &options, Some(&mut diagnostics)).unwrap();
    assert_eq!(output, b"\xef\xbb\xbf; heading\r\n;body \r\nfoo*bar\r\n");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].line, 3);
    assert_eq!(diagnostics[0].byte_offset, 24);
    assert_eq!(diagnostics[0].source, b"foo*bar");
    assert_eq!(
        diagnostics[0].kind,
        DiagnosticKind::AmbiguousLabelOrMultiplication
    );
}

#[test]
fn config_parsing_is_available_without_file_io() {
    let mut options = Options::default();
    parse_config(
        b"# editor settings\nindent=2\r\ncomma-spacing=compact\ntabs\n",
        &mut options,
    )
    .unwrap();
    assert!(parse_formatting_option("--base-indent=1", &mut options).unwrap());
    assert!(!parse_formatting_option("--write", &mut options).unwrap());
    assert_eq!(
        format(b"repeat\nmes 1,2\nloop\n", &options, None).unwrap(),
        b"\trepeat\n\t\tmes 1,2\n\tloop\n"
    );

    let error = parse_config(b"# header\n\tunknown=1\r\n", &mut options).unwrap_err();
    assert_eq!(error.line(), Some(2));
    assert_eq!(error.byte_offset(), Some(10));
    assert_eq!(
        error.message(),
        "invalid or unsupported config option: --unknown=1"
    );
    let error = parse_config(b"indent=2\n\xff\n", &mut options).unwrap_err();
    assert_eq!(error.line(), Some(2));
    assert_eq!(error.message(), "config options must be ASCII");

    // A failed config leaves earlier lines unapplied.
    assert_eq!(options.indent_width, 2);
    parse_config(b"indent=3\nunknown\n", &mut options).unwrap_err();
    assert_eq!(options.indent_width, 2);
}

#[test]
fn config_discovery_preserves_cli_precedence_without_file_io() {
    let directory = Path::new("project");
    let explicit = Path::new("presets/custom.hspfmt");
    assert_eq!(
        find_config(directory, Some(explicit), true, |_| panic!(
            "must not probe"
        )),
        None
    );
    assert_eq!(
        find_config(directory, Some(explicit), false, |_| panic!(
            "must not probe"
        )),
        Some(explicit.to_path_buf())
    );
    assert_eq!(
        find_config(directory, None, false, |path| path
            == directory.join(".hspfmt")),
        Some(PathBuf::from("project/.hspfmt"))
    );
    assert_eq!(find_config(directory, None, false, |_| false), None);
}
