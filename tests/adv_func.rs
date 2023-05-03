use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use assert_fs::prelude::*; // Temp file and file assertion
use std::process::Command; // Run programs
use std::env;
use std::fs::{ self, File };
use std::io::Write;

const CRATE_NAME: &str = "git-starter-rust";
const SHA_REGEX: &str = "[0-9a-f]{40}";

#[test]
fn init_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;

    let temp_folder = assert_fs::TempDir::new()?;
    env::set_current_dir(temp_folder.path())?;

    cmd.arg("init");
    // Check output
    cmd.assert().success().stdout(predicate::str::contains("Initialized git directory"));

    // Check folder structure
    temp_folder.child(".git/HEAD").assert(predicate::path::exists());
    temp_folder.child(".git/objects").assert(predicate::path::exists());
    temp_folder.child(".git/refs").assert(predicate::path::exists());

    // Check file contents
    let text_data = &fs::read(temp_folder.child(".git/HEAD"))?;
    assert_eq!("ref: refs/heads/master\n", &String::from_utf8_lossy(text_data));

    Ok(())
}

#[test]
fn write_blob() -> Result<(), Box<dyn std::error::Error>> {
    const TEST_FILE: &str = "donkey";
    const TEST_DATA: &str = "dooby donkey dumpty";
    const EXPECTED_OUT: &str = "768a28c158afde23d938dcbadcaa325fc2c31353\n";
    let mut check_cmd = Command::new("git");

    let temp_folder = assert_fs::TempDir::new()?;
    env::set_current_dir(temp_folder.path())?;

    let mut temp_file: File = File::create(TEST_FILE)?;
    temp_file.write_all(TEST_DATA.as_bytes())?;

    // Init
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.arg("init");
    cmd.assert().success().stdout(predicate::str::contains("Initialized git directory"));

    // Call hash object
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.args(["hash-object", "-w", TEST_FILE]);

    // Check output
    cmd.assert().success().stdout(predicate::str::is_match(SHA_REGEX)?);
    let returned_sha = cmd.output()?.stdout;

    assert_eq!(&String::from_utf8_lossy(&returned_sha), EXPECTED_OUT);

    // Check hashed object
    check_cmd.args(["cat-file", "-p", &EXPECTED_OUT[..EXPECTED_OUT.len() - 1]]);
    check_cmd.assert().success().stdout(predicate::eq(TEST_DATA));

    Ok(())
}