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

/// Function to checkout to HEAD
/// To call we must be right in the working directory
pub fn checkout_head() -> Result<()> {
    let bytes: Vec<u8> = match fs::read(HEAD_PATH) {
        Ok(data) => data,
        Err(_) => {
            bail!("Cannot open '{}'", HEAD_PATH);
        }
    };
    let head_contentents: String = String::from_utf8(bytes)?;
    // println!("HEAD: {head_contentents}");

    // Get commit referenced by HEAD
    let commit_hash = if head_contentents.starts_with("ref: ") {
        let additional_path: &str = &head_contentents[5..head_contentents.len() - 1];
        let bytes: Vec<u8> = match fs::read(format!(".git/{additional_path}")) {
            Ok(data) => data,
            Err(_) => {
                bail!("Cannot open '{}'", additional_path);
            }
        };
        String::from_utf8(bytes)?[..40].to_owned()
    } else {
        head_contentents[..40].to_owned()
    };
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