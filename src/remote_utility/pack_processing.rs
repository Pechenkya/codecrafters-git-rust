use crate::utility::*;

use anyhow::{ anyhow, Result, Ok };
use bytes::{ Bytes, Buf };
use std::io::prelude::*;
use sha1::{ Sha1, Digest };
use flate2::read::ZlibDecoder;
// use flate2::write::ZlibEncoder;

const OBJ_TYPES: [&[u8]; 8] = [
    b"INVALID",
    b"commit",
    b"tree",
    b"blob",
    b"tag",
    b"RESERVED",
    b"obj_ofs_delta",
    b"obj_ref_delta",
];

/// Structure to process parsed objects
pub enum ParsedObject {
    Default {
        obj_type: Vec<u8>,
        hash: String,
        obj_data: Vec<u8>,
    },
    Delta {
        obj_type: Vec<u8>,
        obj_ref: String,
        delta: Vec<u8>,
    },
    Unsupported(u8),
}

/// Check recieved data structure and return objects
pub fn validate_and_get_heart(bytes: Vec<u8>) -> Result<Vec<ParsedObject>> {
    // Check PACK signature
    if &bytes[..4] != b"PACK" {
        return Err(anyhow!("Incorrect PACK structure"));
    }

    // Get version (next 4 bytes), current implementation supports version 0002
    let _version = u32::from_be_bytes(bytes[4..8].try_into()?);
    // println!("Version: {_version}");

    // Get object count (next 4 bytes)
    let object_amount = u32::from_be_bytes(bytes[8..12].try_into()?);
    // println!("Object amount: {object_amount}");

    // Compare Checksum
    let checksum = hex::encode(
        Sha1::new()
            .chain_update(&bytes[..bytes.len() - 20])
            .finalize()
    );
    if checksum != hex::encode(&bytes[bytes.len() - 20..]) {
        return Err(anyhow!("CheckSum is not correct!"));
    }

    let mut buff: Bytes = Bytes::from(bytes[12..bytes.len() - 20].to_owned());

    let mut parsed_objects: Vec<ParsedObject> = Vec::new();
    // Go through all objects in PACK
    for _obj_id in 0..object_amount {
        let obj: ParsedObject = parse_object(&mut buff)?;
        match obj {
            ParsedObject::Unsupported(id) => {
                return Err(anyhow!("Unsupported object type! ID: {id}"));
            }
            ParsedObject::Default { .. } => parsed_objects.push(obj),
            ParsedObject::Delta { obj_type, obj_ref, delta } => {
                println!("{}: {obj_ref}\n{delta:?}", String::from_utf8(obj_type.clone())?);
            }
        }
    }

    Ok(parsed_objects)
}

/// Parse single object
fn parse_object(buff: &mut Bytes) -> Result<ParsedObject> {
    let (_obj_size, obj_type_id) = get_size_and_typeid(buff)?;
    let obj_type: Vec<u8> = OBJ_TYPES[obj_type_id as usize].to_vec();

    if (1..=4).contains(&obj_type_id) {
        // Try to decompress and drop consumed data
        let (consumed_amt, decoded_data) = decompress_all(buff.clone())?;
        buff.advance(consumed_amt);

        let hash = hex::encode(
            Sha1::new()
                .chain_update(
                    other_util::add_data_prefix(&obj_type, decoded_data.to_vec()).as_slice()
                )
                .finalize()
        );

        // Debug
        println!("{} : {}", String::from_utf8(obj_type.clone())?, hash);

        #[allow(unsafe_code)]
        let buff_string = unsafe { String::from_utf8_unchecked(decoded_data.to_vec()) };
        println!("Data: {}", buff_string);
        // Debug end

        Ok(ParsedObject::Default { obj_type, hash, obj_data: decoded_data.to_vec() })
    } else if obj_type_id == 7 {
        // Get ref to other object
        let hash: String = read_20_bytes_to_string(buff)?;
        let (consumed_amt, decoded_data) = decompress_all(buff.clone())?;
        buff.advance(consumed_amt);

        Ok(ParsedObject::Delta { obj_type, obj_ref: hash, delta: decoded_data.to_vec() })
    } else {
        Ok(ParsedObject::Unsupported(obj_type_id))
    }
}

/// Function to consume unary byte from Buff
fn consume_byte(buff: &mut Bytes) -> u8 {
    return buff.get_u8();
}

fn read_20_bytes_to_string(buff: &mut Bytes) -> Result<String> {
    let mut tmp_buff: [u8; 20] = [0; 20];
    buff.copy_to_slice(&mut tmp_buff);
    return Ok(hex::encode(&tmp_buff));
}

fn decompress_all(data: Bytes) -> Result<(usize, Bytes)> {
    let mut decoder = ZlibDecoder::new(data.as_ref());
    let mut decoded_data: Vec<u8> = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok((decoder.total_in() as usize, Bytes::from(decoded_data)))
}

/// Parse object for size and typeid
fn get_size_and_typeid(buff: &mut Bytes) -> Result<(usize, u8)> {
    // Parse first byte to get start info
    let mut byte: u8 = consume_byte(buff);
    let typeid: u8 = (byte & 0b01110000_u8) >> 4;
    let mut size: usize = (byte & 0b00001111_u8) as usize;

    let mut bits_to_shift = 4; // First 4 bits are already taken
    while (byte & 0b10000000_u8) != 0 {
        byte = consume_byte(buff);
        // Take 7 free bits and mark them as occupied
        size |= ((byte & 0b01111111_u8) as usize) << bits_to_shift;
        bits_to_shift += 7;
    }

    Ok((size, typeid))
}

/// Parse delta object
fn get_delta_size(buff: &mut Bytes) -> usize {
    // Parse first byte to get start info
    let mut byte: u8 = consume_byte(buff);
    let mut size: usize = (byte & 0b01111111_u8) as usize;

    let mut bits_to_shift = 4; // First 4 bits are already taken
    while (byte & 0b10000000_u8) != 0 {
        byte = consume_byte(buff);
        // Take 7 free bits and mark them as occupied
        size |= ((byte & 0b01111111_u8) as usize) << bits_to_shift;
        bits_to_shift += 7;
    }

    size
}