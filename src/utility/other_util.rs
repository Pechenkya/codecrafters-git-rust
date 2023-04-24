use anyhow::Result;
use std::time::SystemTime;

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