use serial_test::serial; // To call tests sequentially
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use assert_fs::{ prelude::*, TempDir }; // Temp file and file assertion
use std::process::Command; // Run programs
use std::env;
use std::fs::{ self, File };
use std::io::Write;
use rand::prelude::*;
use folder_compare::FolderCompare;
use rand::distributions::{ Alphanumeric, DistString };

const CRATE_NAME: &str = "git-starter-rust";
const SHA_REGEX: &str = "[0-9a-f]{40}";

const TEST_REPO_1: &str = "https://github.com/codecrafters-io/git-sample-1";
const TEST_REPO_2: &str = "https://github.com/codecrafters-io/git-sample-2";
const TEST_REPO_3: &str = "https://github.com/codecrafters-io/git-sample-3";

#[serial(comm)]
#[test]
fn init_cmd() -> Result<(), Box<dyn std::error::Error>> {
    println!("------------ INIT -------------");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;

    let temp_folder = assert_fs::TempDir::new()?;
    env::set_current_dir(temp_folder.path())?;

    // Check output
    cmd.arg("init");
    cmd.assert().success().stdout(predicate::str::contains("Initialized git directory"));

    // Check folder structure
    temp_folder.child(".git/HEAD").assert(predicate::path::exists());
    temp_folder.child(".git/objects").assert(predicate::path::exists());
    temp_folder.child(".git/refs").assert(predicate::path::exists());

    // Check file contents
    let text_data = &fs::read(temp_folder.child(".git/HEAD"))?;
    assert_eq!("ref: refs/heads/master\n", &String::from_utf8_lossy(text_data));

    temp_folder.close()?;
    env::set_current_dir("/")?;

    Ok(())
}

#[serial(comm)]
#[test]
fn read_blob() -> Result<(), Box<dyn std::error::Error>> {
    println!("------------ READ BLOB -------------");
    const TEST_FILE: &str = "Greetings";
    const TEST_DATA: &str = "Hello fren!\nNice to meet ya!";
    let mut git_cmd = Command::new("git");

    let temp_folder = assert_fs::TempDir::new()?;
    env::set_current_dir(temp_folder.path())?;

    let mut temp_file: File = File::create(TEST_FILE)?;
    temp_file.write_all(TEST_DATA.as_bytes())?;

    // Init
    print!("Initialising directory");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.arg("init");
    cmd.assert().success().stdout(predicate::str::contains("Initialized git directory"));
    println!(" - OK");

    // Call git's manual hash object
    print!("Hashing object with git");
    git_cmd.args(["hash-object", "-w", TEST_FILE]);

    let return_bytes = &git_cmd.output()?.stdout[..40];
    let sha = String::from_utf8_lossy(return_bytes);
    println!(" - OK");
    println!("Returned Sha: {}", sha);

    // Read contents and check
    print!("Calling cat-file");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.args(["cat-file", "-p", &sha]);
    cmd.assert().success().stdout(predicate::eq(TEST_DATA));
    println!(" - OK");

    temp_folder.close()?;
    env::set_current_dir("/")?;

    Ok(())
}

#[serial(comm)]
#[test]
fn write_blob() -> Result<(), Box<dyn std::error::Error>> {
    println!("------------ WRITE BLOB -------------");
    const TEST_FILE: &str = "donkey";
    const TEST_DATA: &str = "dooby donkey dumpty";
    const EXPECTED_OUT: &str = "768a28c158afde23d938dcbadcaa325fc2c31353";
    let mut check_cmd = Command::new("git");

    let temp_folder = assert_fs::TempDir::new()?;
    env::set_current_dir(temp_folder.path())?;

    let mut temp_file: File = File::create(TEST_FILE)?;
    temp_file.write_all(TEST_DATA.as_bytes())?;

    // Init
    print!("Initialising directory");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.arg("init");
    cmd.assert().success().stdout(predicate::str::contains("Initialized git directory"));
    println!(" - OK");

    // Call hash object
    print!("Calling hash-object");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.args(["hash-object", "-w", TEST_FILE]);

    // Check output
    cmd.assert().success().stdout(predicate::str::is_match(SHA_REGEX)?);
    let returned_sha = cmd.output()?.stdout;

    assert_eq!(&String::from_utf8_lossy(&returned_sha[..40]), EXPECTED_OUT);
    println!(" - OK");

    // Check hashed object
    print!("Checking hashed data");
    check_cmd.args(["cat-file", "-p", &EXPECTED_OUT]);
    check_cmd.assert().success().stdout(predicate::eq(TEST_DATA));
    println!(" - OK");

    temp_folder.close()?;
    env::set_current_dir("/")?;

    Ok(())
}

struct TestDirEntry {
    pub name: String,
    #[allow(dead_code)]
    pub is_dir: bool,
}

/// Returns top-level test dir entries sorted by name
fn write_random_contents(dir: &TempDir) -> Result<Vec<TestDirEntry>, Box<dyn std::error::Error>> {
    let mut res: Vec<TestDirEntry> = Vec::new();

    // Write top level
    let mut temp_file: File = File::create("text")?;
    temp_file.write_all(Alphanumeric.sample_string(&mut thread_rng(), 20).as_bytes())?;
    fs::create_dir_all(dir.child("dir1"))?;
    fs::create_dir_all(dir.child("dir2"))?;
    res.append(
        &mut vec![
            TestDirEntry { name: "text".to_string(), is_dir: false },
            TestDirEntry { name: "dir1".to_string(), is_dir: true },
            TestDirEntry { name: "dir2".to_string(), is_dir: true }
        ]
    );

    // Write low level
    let mut temp_file: File = File::create(dir.child("dir1/foo"))?;
    temp_file.write_all(Alphanumeric.sample_string(&mut thread_rng(), 20).as_bytes())?;

    let mut temp_file: File = File::create(dir.child("dir2/bar"))?;
    temp_file.write_all(Alphanumeric.sample_string(&mut thread_rng(), 20).as_bytes())?;

    res.sort_by_key(|e| e.name.clone());
    Ok(res)
}

#[serial(comm)]
#[test]
fn read_tree() -> Result<(), Box<dyn std::error::Error>> {
    println!("------------ READ TREE -------------");
    let temp_folder = assert_fs::TempDir::new()?;
    env::set_current_dir(temp_folder.path())?;

    // Init
    print!("Initialising directory");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.arg("init");
    cmd.assert().success().stdout(predicate::str::contains("Initialized git directory"));
    println!(" - OK");

    // Setup dir
    print!("Setup dir");
    let expected_result: Vec<_> = write_random_contents(&temp_folder)?
        .into_iter()
        .map(|v| v.name)
        .collect();
    println!(" - OK");

    // Commit and write tree
    print!("Calling git commit and write-tree");
    let mut cmd = Command::new("git");
    cmd.args(["add", "."]);
    cmd.assert().success();
    let mut cmd = Command::new("git");
    cmd.args(["commit", "-m", "Write tree"]);
    cmd.assert().success();
    print!("Calling git write-tree");
    let mut cmd = Command::new("git");
    cmd.arg("write-tree");

    // Check output
    cmd.assert().success().stdout(predicate::str::is_match(SHA_REGEX)?);
    let mut returned_sha = cmd.output()?.stdout;
    returned_sha.pop();
    println!(" - OK");

    // Call read tree
    print!("Calling your ls-tree");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.args(["ls-tree", "--name-only", &String::from_utf8(returned_sha)?]);

    // Check output
    cmd.assert()
        .success()
        .stdout(predicate::eq(format!("{}\n", expected_result.join("\n"))));
    println!(" - OK");

    temp_folder.close()?;
    env::set_current_dir("/")?;

    Ok(())
}

#[serial(comm)]
#[test]
fn write_tree() -> Result<(), Box<dyn std::error::Error>> {
    println!("------------ WRITE TREE -------------");
    let temp_folder = assert_fs::TempDir::new()?;
    env::set_current_dir(temp_folder.path())?;

    // Init
    print!("Initialising directory");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.arg("init");
    cmd.assert().success().stdout(predicate::str::contains("Initialized git directory"));
    println!(" - OK");

    // Setup dir
    print!("Setup dir");
    let expected_result: Vec<_> = write_random_contents(&temp_folder)?
        .into_iter()
        .map(|v| v.name)
        .collect();
    println!(" - OK");

    // Call write tree
    print!("Calling yourgit write-tree");
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;

    cmd.arg("write-tree");
    cmd.assert().success().stdout(predicate::str::is_match(SHA_REGEX)?);

    let mut returned_sha = cmd.output()?.stdout;
    returned_sha.pop();
    println!(" - OK");

    // Check tree
    print!("Calling git ls-tree");
    let mut cmd = Command::new("git");
    cmd.args(["ls-tree", "--name-only", &String::from_utf8(returned_sha)?]);

    // Check output
    cmd.assert()
        .success()
        .stdout(predicate::eq(format!("{}\n", expected_result.join("\n"))));
    println!(" - OK");

    temp_folder.close()?;
    env::set_current_dir("/")?;

    Ok(())
}

#[serial(comm)]
#[test]
fn clone_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("------------ CLONE -------------");
    let repo_id = thread_rng().gen_range(1..=3);
    let remote_repo = match repo_id {
        1 => TEST_REPO_1,
        2 => TEST_REPO_2,
        3 => TEST_REPO_3,
        _ => "UNEXPECTED RANGE",
    };
    println!("Executing clone with TEST_REPO_{repo_id}");

    // Clone with default git
    print!("Clonning with git clone");
    let temp_folder_2 = assert_fs::TempDir::new()?;
    let mut check_cmd = Command::new("git");
    check_cmd.args(["clone", remote_repo, temp_folder_2.to_str().unwrap()]).spawn()?;
    println!(" - OK");

    // Clone with my git
    print!("Clonning with mygit clone");
    let temp_folder_1 = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.args(["clone", remote_repo, temp_folder_1.to_str().unwrap()]);
    cmd.assert().success();
    println!(" - OK");

    // Compare working trees in both folders
    print!("Validating working tree contents");
    let excluded = vec![".git".to_string()];
    let cmp_result = FolderCompare::new(
        temp_folder_1.path(),
        temp_folder_2.path(),
        &excluded
    ).unwrap();
    assert!(cmp_result.changed_files.is_empty());
    assert!(cmp_result.new_files.is_empty());
    println!(" - OK");

    // Check object/HEAD/refs contents
    print!("Validating git refs");
    let mut summary_diff = Vec::new();
    let cmp_result = FolderCompare::new(
        temp_folder_1.child(".git/refs/heads").path(),
        temp_folder_2.child(".git/refs/heads").path(),
        &vec![]
    ).unwrap();
    summary_diff.extend(cmp_result.changed_files);
    summary_diff.extend(cmp_result.new_files);
    println!(" - OK");

    // Validating of HEAD commit
    print!("Validating HEAD");
    assert_eq!(
        fs::read(temp_folder_1.child(".git/HEAD"))?,
        fs::read(temp_folder_2.child(".git/HEAD"))?
    );
    println!(" - OK"); // Could be skipped (according to implementation)

    temp_folder_1.close()?;
    temp_folder_2.close()?;

    Ok(())
}