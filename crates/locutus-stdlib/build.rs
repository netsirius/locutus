extern crate flatc_rust; // or just `use flatc_rust;` with Rust 2018 edition.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../../schemas/flatbuffers/client_request.fbs");
    flatc_rust::run(flatc_rust::Args {
        inputs: &[Path::new("../../schemas/flatbuffers/client_request.fbs")],
        out_dir: Path::new("../../target/flatbuffers/"),
        extra: &["--gen-all"],
        ..Default::default()
    })
    .expect("flatc");
}
