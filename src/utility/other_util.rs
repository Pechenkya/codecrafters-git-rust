use anyhow::{ anyhow, Result };
use std::time::SystemTime;
use sha1::{ Sha1, Digest };

// Hardcoded constants
const COMMITER_NAME: &[u8] = b"Petro Bondar";
const COMMITER_EMAIL: &[u8] = b"pb@gmail.com";
const COMMITER_AFTER_STAMP: &[u8] = b"-0700";

pub fn add_data_prefix(prefix: &[u8], mut text: Vec<u8>) -> Vec<u8> {
    let mut result = prefix.to_vec();
    result.push(b' ');
    result.append(&mut text.len().to_string().into_bytes());
    result.push(b'\0');
    result.append(&mut text);
    result
}

pub fn get_time_stamp_string() -> Result<String> {
    Ok(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs().to_string())
}

pub fn append_commiter_data(contents: &mut Vec<u8>, timestamp: &String) {
    contents.append(&mut COMMITER_NAME.to_vec());
    contents.push(b' ');
    contents.push(b'<');
    contents.append(&mut COMMITER_EMAIL.to_vec());
    contents.push(b'>');
    contents.push(b' ');
    contents.append(&mut timestamp.as_bytes().to_vec());
    contents.push(b' ');
    contents.append(&mut COMMITER_AFTER_STAMP.to_vec());
    contents.push(b'\n');
}

pub fn get_hash_from_data(data: &[u8]) -> String {
    hex::encode(Sha1::new().chain_update(data).finalize())
}

/// Returns tuples (<file name>, <mode>, <SHA-1>) from correct tree object
pub fn parse_tree(binary: &[u8]) -> Result<Vec<(String, String, String)>> {
    // Convert to string and divide it into blocks
    #[allow(unsafe_code)]
    let buff_string = unsafe { String::from_utf8_unchecked(binary.to_vec()) };
    let (header, mut text) = buff_string
        .split_once('\0')
        .ok_or(anyhow!("Cannot separate header!"))?;

    // If header block is for correct tree -> parse text to find tuples (<file name>, <mode>, <SHA-1>)
    let mut contents: Vec<(String, String, String)> = Vec::new();
    if let ("tree", _tree_size) = header.split_once(' ').ok_or(anyhow!("Not a tree type!"))? {
        // Simple parse with unchecked string
        while !text.is_empty() {
            // Check if struct is correct and we can extract mode
            if let Some((_mode, rest)) = text.split_once(' ') {
                // Extract filename
                let (file_name, rest) = rest
                    .split_once('\0')
                    .ok_or(anyhow!("Cannot separate file name!"))?;

                // Extract SHA-1
                let (_sha, rem) = rest.split_at(20);
                text = rem;

                // Add content
                contents.push((file_name.to_string(), _mode.to_string(), hex::encode(_sha)));

                // Debug log
                // println!("{_mode} {file_name}: {}", hex::encode(_sha));
            } else {
                return Err(anyhow!("Not a tree object!"));
            }
        }
    } else {
        return Err(anyhow!("Not a tree object!"));
    }

    Ok(contents)
}