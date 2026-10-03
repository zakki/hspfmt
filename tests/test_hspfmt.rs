use hspfmt::*;

fn expect(input: &[u8], expected: &[u8], options: Options) {
    let restored: Vec<u8> = lex(input, options.encoding).unwrap().into_iter().flat_map(|token| input[token.begin..token.end].to_vec()).collect();
    assert_eq!(restored, input, "roundtrip failed");

    let actual = format(input, &options, None).unwrap();
    assert_eq!(actual.as_bytes(), expected, "actual did not match expected for input: {:?}", String::from_utf8_lossy(input));

    let idempotent = format(actual.as_bytes(), &options, None).unwrap();
    assert_eq!(idempotent, actual, "not idempotent");
}

#[test]
fn test_hspfmt() {
    let options = Options::default();
    expect(b"", b"", options.clone());
    expect(b"a=1:b=2\n", b"a = 1 : b = 2\n", options.clone());
}
