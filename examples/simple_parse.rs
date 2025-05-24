fn main() {
    let source = std::path::PathBuf::from("tests/testdata/input/main-samples/");
    let mut exporter = obsidian_export::Exporter::new(source);
    let result = exporter.run().unwrap();
    println!("{result:?}");
}
