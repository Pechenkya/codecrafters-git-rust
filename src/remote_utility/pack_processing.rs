use anyhow::{ anyhow, Result, Ok };
use std::io::prelude::*;

/// Check recieved data structure and return metadata
pub fn validate_and_get_heart(bytes: &[u8]) -> Result<Vec<u8>> {
    println!("{:?}", &bytes[..4]);
    if &bytes[..4] != b"PACK" {
        return Err(anyhow!("Incorrect PACK structure"));
    }

    Ok(vec![])
}