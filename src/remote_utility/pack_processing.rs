use anyhow::{ anyhow, Result, Ok };
use std::io::{ prelude::*, Cursor, SeekFrom };
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
        obj_data: Vec<u8>,
    },
    Delta {
        obj_type: Vec<u8>,
        obj_ref: Vec<u8>,
        delta: Vec<u8>,
    },
    UNSUPPORTED,
}

/// Check recieved data structure and return objects
pub fn validate_and_get_heart(bytes: &[u8]) -> Result<Vec<ParsedObject>> {
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

    let mut buff: Cursor<&[u8]> = Cursor::new(&bytes[..bytes.len() - 20]);
    buff.seek(SeekFrom::Current(12))?;

    let mut parsed_objects: Vec<ParsedObject> = Vec::new();
    // Go through all objects in PACK
    for _obj_id in 0..object_amount {
        let obj: ParsedObject = parse_object(&mut buff)?;
        parsed_objects.push(obj);
    }

    Ok(parsed_objects)
}

/// Parse single object
fn parse_object(buff: &mut Cursor<&[u8]>) -> Result<ParsedObject> {
    let (obj_size, obj_type_id) = get_size_and_typeid(buff)?;
    let obj_type: Vec<u8> = OBJ_TYPES[obj_type_id as usize].to_vec();

    if 1 <= obj_type_id && obj_type_id <= 4 {
        let mut obj_data_encoded: Vec<u8> = Vec::new();
        obj_data_encoded.resize(obj_size, 0);
        buff.read_exact(obj_data_encoded.as_mut_slice())?;

        println!(
            "{}\n{}/{}: {:?}",
            String::from_utf8(obj_type.clone())?,
            obj_data_encoded.len(),
            obj_size,
            obj_data_encoded
        );

        let mut decoder = ZlibDecoder::new(obj_data_encoded.as_slice());
        let mut obj_data: Vec<u8> = Vec::new();
        decoder.read_to_end(&mut obj_data)?;

        println!("Data: {}", String::from_utf8(obj_data.clone())?);

        Ok(ParsedObject::Default { obj_type, obj_data })
    } else if obj_type_id == 7 {
        let mut obj_ref: [u8; 20] = [0; 20];
        buff.read_exact(&mut obj_ref)?;
        println!("got here!");
        Ok(ParsedObject::Delta { obj_type, obj_ref: obj_ref.to_vec(), delta: vec![] })
    } else {
        Ok(ParsedObject::UNSUPPORTED)
    }
}

/// Function to consume unary byte from Buff
fn consume_byte(buff: &mut Cursor<&[u8]>) -> Result<u8> {
    if let Some(byte) = buff.bytes().next().transpose()? {
        Ok(byte)
    } else {
        Err(anyhow!("Cannot get byte!"))
    }
}

/// Parse object for size and typeid
fn get_size_and_typeid(buff: &mut Cursor<&[u8]>) -> Result<(usize, u8)> {
    // Parse first byte to get start info
    let mut byte: u8 = consume_byte(buff)?;
    let typeid: u8 = (byte & 0b01110000_u8) >> 4;
    let mut size: usize = (byte & 0b00001111_u8) as usize;

    let mut bits_to_shift = 4; // First 4 bits are already taken
    while (byte & 0b10000000_u8) != 0 {
        print!("{byte}, ");
        byte = consume_byte(buff)?;
        // Take 7 free bits and mark them as occupied
        size += ((byte & 0b01111111_u8) as usize) << bits_to_shift;
        bits_to_shift += 7;
    }

    Ok((size, typeid))
}