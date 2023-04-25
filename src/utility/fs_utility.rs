use anyhow::{ anyhow, Result };
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use crate::utility::other_util::*;

pub fn find_root_folder() -> Result<String> {
    let mut prefix_path = String::from("./");
    // Check depth 256 for a .git folder
    for _ in 0..256 {
        if Path::new(&format!("{prefix_path}.git")).is_dir() {
            return Ok(prefix_path);
        }
        prefix_path += "../";
    }

    Err(anyhow!("Cannot find .git folder!"))
}

pub fn compute_path_from_sha(sha: &String) -> Result<String> {
    let path = find_root_folder()? + ".git/objects/" + &sha[0..2] + "/" + &sha[2..sha.len()];
    Ok(path)
}

/// Moves in data and writes it into corresponding object
/// Returns SHA for object
pub fn write_data(data: Vec<u8>) -> Result<String> {
    // Generate Hash and encode it to hex
    let hash = get_hash_from_data(&data);

    // Compress data
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_slice())?;
    let encoded_text: Vec<u8> = encoder.finish()?;

    // Create path
    let path_to_save: String = compute_path_from_sha(&hash)?;
    std::fs::create_dir_all(Path::new(&path_to_save).parent().unwrap())?;

    // Save object
    let mut obj: fs::File = fs::File::create(path_to_save)?;
    obj.write_all(encoded_text.as_slice())?;

    Ok(hash)
}