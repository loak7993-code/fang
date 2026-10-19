//! Fixture tests: every .leo file under tests/fixtures must lex cleanly.

use leo_core::Lexer;

#[test]
fn all_fixture_files_lex() {
    let mut checked = 0;
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |e| e == "leo") {
            let src = std::fs::read_to_string(&path).unwrap();
            let toks = Lexer::new(&src)
                .tokenize()
                .unwrap_or_else(|e| panic!("{} failed to lex: {}", path.display(), e));
            assert!(toks.len() > 1, "{} produced no tokens", path.display());
            assert_eq!(toks.last().unwrap().kind, leo_core::TokKind::Eof);
            checked += 1;
        }
    }
    assert!(checked >= 3, "expected at least 3 fixture files, found {}", checked);
}
