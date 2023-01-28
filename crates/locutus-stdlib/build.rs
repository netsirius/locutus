use std::process::Command;

fn main() {
    let status = Command::new("flatc")
        .arg("--rust")
        .arg("--gen-object-api")
        .arg("-o")
        .arg("../../target/flatbuffers/")
        .arg("../../schemas/flatbuffers/client_request.fbs")
        .status()
        .unwrap();
    assert!(status.success());
}
