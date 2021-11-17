#[test]
fn cli_test() {
    let t = trycmd::TestCases::new();
    t.case("tests/cmd/*.md")
        .case("tests/cmd/*.trycmd")
        .case("tests/cmd/*.toml");
}
