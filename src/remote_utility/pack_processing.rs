use crate::utility::*;

use anyhow::{ anyhow, Result };
use bytes::{ Bytes, Buf };
use std::{ io::prelude::*, collections::HashMap };
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
enum ParsedObject {
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

pub struct UnpackedObject {
    pub obj_type: Vec<u8>,
    pub hash: String,
    pub contents: Vec<u8>,
}

impl std::fmt::Display for UnpackedObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        #[allow(unsafe_code)]
        let buff_string = unsafe { String::from_utf8_unchecked(self.contents.clone()) };
        write!(
            f,
            "{} - {}\n{}",
            String::from_utf8(self.obj_type.clone()).unwrap(),
            self.hash,
            buff_string
        )
    }
}

/// Check recieved data structure and return objects
pub fn validate_and_get_heart(bytes: Vec<u8>) -> Result<Vec<UnpackedObject>> {
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
    let checksum = other_util::get_hash_from_data(&bytes[..bytes.len() - 20]);
    if checksum != hex::encode(&bytes[bytes.len() - 20..]) {
        return Err(anyhow!("CheckSum is not correct!"));
    }

    // Continue wotking through Bytes iterator
    let mut buff: Bytes = Bytes::from(bytes[12..bytes.len() - 20].to_owned());

    let mut unpacked_objects: Vec<UnpackedObject> = Vec::new();
    let mut ref_to_id: HashMap<String, usize> = HashMap::new();
    // Go through all objects in PACK
    for _obj_id in 0..object_amount {
        let obj: ParsedObject = parse_object(&mut buff)?;
        match obj {
            ParsedObject::Unsupported(id) => {
                return Err(anyhow!("Unsupported object type! ID: {id}"));
            }
            ParsedObject::Default { obj_type, hash, obj_data } => {
                ref_to_id.insert(hash.clone(), _obj_id as usize);
                unpacked_objects.push(UnpackedObject { obj_type, hash, contents: obj_data });
            }
            ParsedObject::Delta { obj_type: _ot, obj_ref, delta } => {
                let obj_id: usize = *ref_to_id
                    .get(&obj_ref)
                    .ok_or(anyhow!("No such object in list: {}!", obj_ref))?;

                // println!("{}: {obj_ref}\n{delta:?}", String::from_utf8(_ot.clone())?);

                // Prepare data to apply delta
                let mut dlt_iter: Bytes = Bytes::from(delta);
                let _: usize = get_delta_size(&mut dlt_iter); // Skip source size
                let target_size: usize = get_delta_size(&mut dlt_iter);
                let refered_object_data: Bytes = Bytes::from(
                    unpacked_objects
                        .get(obj_id)
                        .ok_or(anyhow!("Expected to find object {obj_ref} by id {obj_id}"))?
                        .contents.clone()
                );

                // Apply delta and store new object
                let updated_data: Vec<u8> = apply_delta(
                    &mut dlt_iter,
                    &refered_object_data,
                    target_size
                )?;
                let obj_type = unpacked_objects.get(obj_id).unwrap().obj_type.clone();

                let hash = other_util::get_hash_from_data(
                    other_util::add_data_prefix(&obj_type, updated_data.clone()).as_slice()
                );

                ref_to_id.insert(hash.clone(), _obj_id as usize);
                unpacked_objects.push(UnpackedObject { obj_type, hash, contents: updated_data });
            }
        }
    }

    Ok(unpacked_objects)
}

/// Parse single object
fn parse_object(buff: &mut Bytes) -> Result<ParsedObject> {
    let (_obj_size, obj_type_id) = get_size_and_typeid(buff)?;
    let obj_type: Vec<u8> = OBJ_TYPES[obj_type_id as usize].to_vec();

    if (1..=4).contains(&obj_type_id) {
        // Try to decompress and drop consumed data
        let (consumed_amt, decoded_data) = decompress_all(buff.clone())?;
        buff.advance(consumed_amt);

        let hash = other_util::get_hash_from_data(
            other_util::add_data_prefix(&obj_type, decoded_data.to_vec()).as_slice()
        );

        // Debug
        // println!("{} : {}", String::from_utf8(obj_type.clone())?, hash);

        // #[allow(unsafe_code)]
        // let buff_string = unsafe { String::from_utf8_unchecked(decoded_data.to_vec()) };
        // println!("Data: {}", buff_string);
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
    buff.get_u8()
}

fn read_20_bytes_to_string(buff: &mut Bytes) -> Result<String> {
    let mut tmp_buff: [u8; 20] = [0; 20];
    buff.copy_to_slice(&mut tmp_buff);
    Ok(hex::encode(tmp_buff))
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

    let mut bits_to_shift = 7; // First 4 bits are already taken
    while (byte & 0b10000000_u8) != 0 {
        byte = consume_byte(buff);
        // Take 7 free bits and mark them as occupied
        size |= ((byte & 0b01111111_u8) as usize) << bits_to_shift;
        bits_to_shift += 7;
    }

    size
}

/// Apply delta to existing object and return res_buff
fn apply_delta(dlt_buff: &mut Bytes, obj_buff: &[u8], target_size: usize) -> Result<Vec<u8>> {
    // To store result
    let mut res: Vec<u8> = Vec::new();

    // Go through all bytes in delta
    while !dlt_buff.is_empty() {
        let byte: u8 = consume_byte(dlt_buff);

        // if MSB is 1 -> Go to [Copy mode], else -> Go to [Insert mode]
        if (byte & 0b10000000_u8) != 0 {
            let mut shift: usize = 0;
            let mut amount: usize = 0;

            // Go through bits and get copy info
            if (byte & 0b00000001_u8) != 0 {
                shift |= consume_byte(dlt_buff) as usize;
            }
            if (byte & 0b00000010_u8) != 0 {
                shift |= (consume_byte(dlt_buff) as usize) << 8;
            }
            if (byte & 0b00000100_u8) != 0 {
                shift |= (consume_byte(dlt_buff) as usize) << 16;
            }
            if (byte & 0b00001000_u8) != 0 {
                shift |= (consume_byte(dlt_buff) as usize) << 24;
            }
            if (byte & 0b00010000_u8) != 0 {
                amount |= consume_byte(dlt_buff) as usize;
            }
            if (byte & 0b00100000_u8) != 0 {
                amount |= (consume_byte(dlt_buff) as usize) << 8;
            }
            if (byte & 0b01000000_u8) != 0 {
                amount |= (consume_byte(dlt_buff) as usize) << 16;
            }

            res.append(&mut obj_buff[shift..shift + amount].to_vec());
        } else {
            // Get <byte> bytes and append ot to result buffer
            let mut tmp_buff: Vec<u8> = vec![0; byte as usize];
            dlt_buff.copy_to_slice(&mut tmp_buff);
            res.append(&mut tmp_buff);
        }
    }

    // Compare expected size and real size
    if target_size != res.len() {
        return Err(anyhow!("Unexpected Target size: {}. Expected: {target_size}", res.len()));
    }

    Ok(res)
}