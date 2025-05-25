#![allow(clippy::shadow_unrelated)]

use std::fs::{create_dir, read_to_string, set_permissions, File, Permissions};
use std::io::prelude::*;
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use obsidian_export::{ExportError, Exporter, FrontmatterStrategy};
use pretty_assertions::assert_eq;
use tempfile::TempDir;
mod utils;
use utils::relative_path;
use walkdir::WalkDir;

#[test]
fn test_main_variants_with_default_options() {
    let root = PathBuf::from("tests/testdata/input/main-samples/");
    let result = Exporter::new(root.clone())
        .run()
        .expect("exporter returned error");

    let walker = WalkDir::new("tests/testdata/expected/main-samples/")
        // Without sorting here, different test runs may trigger the first assertion failure in
        // unpredictable order.
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .into_iter();
    for entry in walker {
        let entry = entry.unwrap();
        if entry.metadata().unwrap().is_dir() {
            continue;
        };
        let filename = entry.file_name().to_string_lossy().into_owned();
        let expected = read_to_string(entry.path()).unwrap_or_else(|_| {
            panic!(
                "failed to read {} from testdata/expected/main-samples/",
                entry.path().display()
            )
        });
        let note_path = relative_path(
            &PathBuf::from("tests/testdata/expected/main-samples/"),
            entry.path(),
        );
        let actual = result.get(&note_path).unwrap().to_string();
        dbg!(&actual);
        dbg!(&expected);
        assert_eq!(
            expected, actual,
            "{} does not have expected content",
            filename
        );
    }
}

#[test]
fn test_frontmatter_never() {
    let root = PathBuf::from("tests/testdata/input/main-samples/");
    let mut exporter = Exporter::new(root.clone());
    exporter.frontmatter_strategy(FrontmatterStrategy::Never);
    let result = exporter.run().expect("exporter returned error");

    let expected = "Note with frontmatter.\n";
    let actual = result
        .get(&PathBuf::from("note-with-frontmatter.md"))
        .unwrap()
        .to_string();

    assert_eq!(expected, actual);
}

#[test]
fn test_frontmatter_always() {
    let mut exporter = Exporter::new(PathBuf::from("tests/testdata/input/main-samples/"));
    exporter.frontmatter_strategy(FrontmatterStrategy::Always);
    let result = exporter.run().expect("exporter returned error");

    // Note without frontmatter should have empty frontmatter added.
    let expected = "---\n---\n\nNote without frontmatter.\n";
    let actual = result
        .get(&PathBuf::from("note-without-frontmatter.md"))
        .unwrap()
        .to_string();
    assert_eq!(expected, actual);

    // Note with frontmatter should remain untouched.
    let expected = "---\nFoo: bar\n---\n\nNote with frontmatter.\n";
    let actual = result
        .get(&PathBuf::from("note-with-frontmatter.md"))
        .unwrap()
        .to_string();

    assert_eq!(expected, actual);
}

#[test]
fn test_exclude() {
    let result = Exporter::new(PathBuf::from("tests/testdata/input/main-samples/"))
        .run()
        .expect("exporter returned error");

    let excluded_note = PathBuf::from("excluded-note.md");
    assert!(
        !result.contains_key(&excluded_note),
        "exluded-note.md was found in tmpdir, but should be absent due to .export-ignore rules"
    );
}

#[test]
fn test_single_file() {
    let result = Exporter::new(PathBuf::from("tests/testdata/input/single-file/note.md"))
        .run()
        .unwrap();

    assert_eq!(
        read_to_string("tests/testdata/expected/single-file/note.md").unwrap(),
        result.get(&PathBuf::from("note.md")).unwrap().to_string(),
    );
}

#[test]
fn test_single_file_from_start_at() {
    let root = PathBuf::from("tests/testdata/input/single-file/");
    let result = Exporter::new(root.clone())
        .start_at(root.join("note.md"))
        .run()
        .unwrap();

    assert_eq!(
        read_to_string("tests/testdata/expected/single-file/start-at.md").unwrap(),
        result.get(&PathBuf::from("note.md")).unwrap().to_string(),
    );
}

#[test]
fn test_start_at_subdir() {
    let mut exporter = Exporter::new(PathBuf::from("tests/testdata/input/start-at/"));
    exporter.start_at(PathBuf::from("tests/testdata/input/start-at/subdir"));
    let result = exporter.run().unwrap();

    let expected = if cfg!(windows) {
        read_to_string("tests/testdata/expected/start-at/subdir/Note B.md")
            .unwrap()
            .replace('/', "\\")
    } else {
        read_to_string("tests/testdata/expected/start-at/subdir/Note B.md").unwrap()
    };

    assert_eq!(
        expected,
        result.get(&PathBuf::from("Note B.md")).unwrap().to_string()
    );
}

#[test]
fn test_start_at_file_within_subdir_destination_is_dir() {
    let mut exporter = Exporter::new(PathBuf::from("tests/testdata/input/start-at/"));
    exporter.start_at(PathBuf::from(
        "tests/testdata/input/start-at/subdir/Note B.md",
    ));
    let result = exporter.run().unwrap();

    let expected = if cfg!(windows) {
        read_to_string("tests/testdata/expected/start-at/single-file/Note B.md")
            .unwrap()
            .replace('/', "\\")
    } else {
        read_to_string("tests/testdata/expected/start-at/single-file/Note B.md").unwrap()
    };

    assert_eq!(
        expected,
        result.get(&PathBuf::from("Note B.md")).unwrap().to_string()
    );
}

#[test]
fn test_not_existing_source() {
    let err = Exporter::new(PathBuf::from("tests/testdata/no-such-file.md"))
        .run()
        .unwrap_err();

    match err {
        ExportError::PathDoesNotExist { .. } => {}
        _ => panic!("Wrong error variant: {:?}", err),
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_source_no_permissions() {
    let tmp_dir = TempDir::new().expect("failed to make tempdir");
    let src = tmp_dir.path().to_path_buf().join("source.md");

    let mut file = File::create(&src).unwrap();
    file.write_all(b"Foo").unwrap();
    set_permissions(&src, Permissions::from_mode(0o000)).unwrap();

    match Exporter::new(src).run().unwrap_err() {
        ExportError::FileExportError { source, .. } => match *source {
            ExportError::ReadError { .. } => {}
            _ => panic!("Wrong error variant for source, got: {:?}", source),
        },
        err => panic!("Wrong error variant: {:?}", err),
    }
}

#[test]
fn test_infinite_recursion() {
    let err = Exporter::new(PathBuf::from("tests/testdata/input/infinite-recursion/"))
        .run()
        .unwrap_err();

    match err {
        ExportError::FileExportError { source, .. } => match *source {
            ExportError::RecursionLimitExceeded { .. } => {}
            _ => panic!("Wrong error variant for source, got: {:?}", source),
        },
        err => panic!("Wrong error variant: {:?}", err),
    }
}

#[test]
fn test_no_recursive_embeds() {
    let mut exporter = Exporter::new(PathBuf::from("tests/testdata/input/infinite-recursion/"));
    exporter.process_embeds_recursively(false);
    let result = exporter.run().expect("exporter returned error");

    assert_eq!(
        read_to_string("tests/testdata/expected/infinite-recursion/Note A.md").unwrap(),
        result.get(&PathBuf::from("Note A.md")).unwrap().to_string(),
    );
}

#[test]
fn test_non_ascii_filenames() {
    let result = Exporter::new(PathBuf::from("tests/testdata/input/non-ascii/"))
        .run()
        .expect("exporter returned error");

    let walker = WalkDir::new("tests/testdata/expected/non-ascii/")
        // Without sorting here, different test runs may trigger the first assertion failure in
        // unpredictable order.
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .into_iter();
    for entry in walker {
        let entry = entry.unwrap();
        if entry.metadata().unwrap().is_dir() {
            continue;
        };
        let filename = entry.file_name().to_string_lossy().into_owned();
        let expected = read_to_string(entry.path()).unwrap_or_else(|_| {
            panic!(
                "failed to read {} from testdata/expected/non-ascii/",
                entry.path().display()
            )
        });
        let actual = result.get(&PathBuf::from(&filename)).unwrap().to_string();

        assert_eq!(
            expected, actual,
            "{} does not have expected content",
            filename
        );
    }
}

#[test]
fn test_same_filename_different_directories() {
    let result = Exporter::new(PathBuf::from(
        "tests/testdata/input/same-filename-different-directories",
    ))
    .run()
    .unwrap();

    let expected = if cfg!(windows) {
        read_to_string("tests/testdata/expected/same-filename-different-directories/Note.md")
            .unwrap()
            .replace('/', "\\")
    } else {
        read_to_string("tests/testdata/expected/same-filename-different-directories/Note.md")
            .unwrap()
    };

    let actual = result.get(&PathBuf::from("Note.md")).unwrap().to_string();
    assert_eq!(expected, actual);
}
