use crate::utility::fs_utility::*;
use crate::utility::other_util::*;

use anyhow::{ anyhow, bail, Result };
use std::{ fs, path::Path };
use std::io::prelude::*;

const HEAD_PATH: &str = ".git/HEAD";

/// Function to write all refs from commit
/// To call we must be right in the working directory!
pub fn write_refs(refs: &Vec<(String, String)>) -> Result<()> {
    let head_hash = &refs.get(0).ok_or_else(|| anyhow!("Cannot get HEAD ref"))?.0;
    fs::create_dir_all(".git/")?;

    for (hash, path) in &refs[1..] {
        if hash == head_hash {
            // Write ref into head
            // Save ref
            let mut obj: fs::File = fs::File::create(".git/HEAD")?;
            obj.write_all(format!("ref: {path}\n").as_bytes())?;
        }

        let full_path = format!(".git/{path}");
        fs::create_dir_all(
            Path::new(&full_path)
                .parent()
                .ok_or_else(|| anyhow!("Cannot create dirs..."))?
        )?;

        // Save ref
        let mut obj: fs::File = fs::File::create(full_path)?;
        obj.write_all(format!("{hash}\n").as_bytes())?;
    }

    // Detached head
    if !fs::metadata(".git/HEAD").is_ok() {
        let mut obj: fs::File = fs::File::create(".git/HEAD")?;
        obj.write_all(format!("{head_hash}\n").as_bytes())?;
    }

    Ok(())
}

enum HeadRef {
    Sha(String),
    Ref(String),
}

/// Returns reference saved in HEAD
/// To call we must be right in the working directory
fn get_head_ref() -> Result<HeadRef> {
    let bytes: Vec<u8> = match fs::read(HEAD_PATH) {
        Ok(data) => data,
        Err(_) => {
            bail!("Cannot open '{}'", HEAD_PATH);
        }
    };
    let head_contentents = String::from_utf8(bytes)?;
    // println!("HEAD: {head_contentents}");

    if head_contentents.starts_with("ref: ") {
        Ok(HeadRef::Ref(head_contentents[5..head_contentents.len() - 1].to_string()))
    } else if head_contentents.len() == 41 {
        Ok(HeadRef::Sha(head_contentents[..40].to_string()))
    } else {
        bail!("Incorrect head contents: {head_contentents}!")
    }
}

/// Returns referenced commit from HEAD
/// To call we must be right in the working directory
fn get_head_commit_sha() -> Result<String> {
    match get_head_ref()? {
        HeadRef::Sha(hash) => Ok(hash),
        HeadRef::Ref(referenced_file) => {
            let bytes: Vec<u8> = match fs::read(format!(".git/{referenced_file}")) {
                Ok(data) => data,
                Err(_) => {
                    bail!("Cannot open '{}'", referenced_file);
                }
            };
            Ok(String::from_utf8(bytes)?[..40].to_owned())
        }
    }
}

/// Function to checkout to HEAD
/// To call we must be right in the working directory
pub fn checkout_head() -> Result<()> {
    // Get commit referenced by HEAD
    let commit_hash = get_head_commit_sha()?;
    // println!("commit: {commit_hash:?}");

    let commit = String::from_utf8(read_data_decompressed(&commit_hash)?)?;
    // println!("commit data:{commit}");

    let (_, commit_contents) = commit
        .split_once('\0')
        .ok_or_else(|| anyhow!("Cannot parse commit!"))?;
    let (tree_data, _) = commit_contents
        .split_once('\n')
        .ok_or_else(|| anyhow!("Cannot parse commit!"))?;
    let (obj_type, tree_hash) = tree_data
        .split_once(' ')
        .ok_or_else(|| anyhow!("Cannot parse commit!"))?;
    if obj_type != "tree" {
        bail!("Incorrect commit object structure!");
    }
    // println!("tree: {tree_hash}");

    let basic_path: String = String::from(".");
    checkout_tree(tree_hash, basic_path)
}

/// Checkout to full tree object
fn checkout_tree(tree_hash: &str, path: String) -> Result<()> {
    // Create folder if it's missing
    fs::create_dir_all(&path)?;

    // Read data from tree object
    let bytes_decoded: Vec<u8> = read_data_decompressed(tree_hash)?;

    // Parse tree
    for (filename, _mode, sha) in parse_tree(&bytes_decoded)? {
        // println!("{filename}, {_mode}, {sha}");
        let object_contents: Vec<u8> = read_data_decompressed(&sha)?;
        let mut slices_itr = object_contents.split_inclusive(|c| *c == b'\0');

        let header = slices_itr.next().ok_or_else(|| anyhow!("Cannot divide header!"))?;

        if header.starts_with(b"tree") {
            // Go to inner tree
            checkout_tree(&sha, format!("{path}/{filename}"))?;
        } else if header.starts_with(b"blob") {
            // Create file and save data
            let mut obj: fs::File = fs::File::create(format!("{path}/{filename}"))?;

            if let Some(binary) = slices_itr.next() {
                obj.write_all(binary)?;
            }
        } else {
            bail!("Checkout wasn't successfull, wrong header!");
        }
    }

    Ok(())
}

fn format_config(repo_url: &str, branch: &str, head_ref: &str) -> String {
    format!(
        "[core]
\trepositoryformatversion = 0
\tfilemode = true
\tbare = false
\tlogallrefupdates = true
[remote \"origin\"]
\turl = {repo_url}
\tfetch = +refs/heads/*:refs/remotes/origin/*
[branch \"{branch}\"]
\tremote = origin
\tmerge = {head_ref}\n"
    )
}

/// Write config file after clone
/// To call we must be right in the working directory
pub fn write_config(repo_url: &str) -> Result<()> {
    if let HeadRef::Ref(head_ref) = get_head_ref()? {
        let mut cfg_file: fs::File = fs::File::create(format!(".git/config"))?;
        let branch = head_ref
            .rsplit_once('/')
            .ok_or_else(|| anyhow!("Cannot separate branch name!"))?.1;
        let write_data = format_config(repo_url, branch, &head_ref);
        cfg_file.write_all(write_data.as_bytes())?;
    } else {
        bail!("Not a ref inside HEAD after clone!");
    }
    Ok(())
}

/// Generate index file (stage files)
#[allow(dead_code)]
pub fn stage_tree() -> Result<()> {
    Ok(())
}