use anyhow::Context;
use anyhow::{ anyhow, Result };
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use crate::utility::other_util::*;

pub fn create_path_and_move_there<T: AsRef<Path>>(path: &T) -> Result<()> {
    fs::create_dir_all(path)?;
    env::set_current_dir(path)?;

    Ok(())
}

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

pub fn compute_path_from_sha(sha: &str) -> Result<String> {
    let path = find_root_folder()? + ".git/objects/" + &sha[..2] + "/" + &sha[2..sha.len()];
    Ok(path)
}

/// Receive decompressed binary data from object
pub fn read_data_decompressed(sha: &str) -> Result<Vec<u8>> {
    // Compute path to blob
    let path: String = compute_path_from_sha(sha)?;

    // Read binary
    let bytes: Vec<u8> = fs::read(path).with_context(|| format!("Object {} is not found", sha))?;

    // Decompress data and read it to string
    let mut decoder = ZlibDecoder::new(bytes.as_slice());
    let mut bytes_decoded: Vec<u8> = Vec::new();
    decoder.read_to_end(&mut bytes_decoded)?;

    Ok(bytes_decoded)
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
    std::fs::create_dir_all(
        Path::new(&path_to_save).parent().ok_or(anyhow!("Corrupted file path"))?
    )?;

    // Save object
    let mut obj: fs::File = fs::File::create(path_to_save)?;
    obj.write_all(encoded_text.as_slice())?;

    Ok(hash)
}