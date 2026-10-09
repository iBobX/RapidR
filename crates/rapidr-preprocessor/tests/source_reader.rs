//! A host without a disk gives its files to the preprocessor (RapidR Studio
//! in a browser: the page's store), so `$INCLUDE` follows them as on the
//! desktop — the language service's and the designer's analysis included.
//! (Its own test binary: the reader is the process's.)

use std::path::Path;

use rapidr_preprocessor::{preprocess_source, read_source, set_source_reader, source_exists, PreprocessOptions};

fn page_store(path: &Path) -> Option<Vec<u8>> {
    match path.to_string_lossy().replace('\\', "/").as_str() {
        "examples/multi/main.rr" => Some(b"$INCLUDE \"Form2.rr\"\nForm2.Show\n".to_vec()),
        "examples/multi/Form2.rr" => Some(b"CREATE Form2 AS RForm\nEND CREATE\n".to_vec()),
        _ => None,
    }
}

#[test]
fn includes_are_read_through_the_hosts_reader() {
    set_source_reader(Some(page_store));
    assert!(source_exists(Path::new("examples/multi/Form2.rr")));
    assert!(!source_exists(Path::new("examples/multi/Form3.rr")));
    let main = read_source(Path::new("examples/multi/main.rr")).unwrap();
    let out = preprocess_source(&main, "examples/multi", Some("examples/multi/main.rr".into()), PreprocessOptions::default()).unwrap();
    assert!(out.source.starts_with("CREATE Form2 AS RForm\nEND CREATE\n") && out.source.ends_with("Form2.Show\n"), "{:?}", out.source);
    // a file the page doesn't have is missing, as on a disk
    let missing = preprocess_source("$INCLUDE \"Form3.rr\"\n", "examples/multi", Some("examples/multi/main.rr".into()), PreprocessOptions::default());
    assert!(missing.is_err());
    set_source_reader(None);
    assert!(read_source(Path::new("examples/multi/main.rr")).is_err(), "the disk again");
}
