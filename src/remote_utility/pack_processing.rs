use anyhow::{ anyhow, Result, Ok };
use std::io::{ prelude::*, BufReader };
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

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

    let mut buff: BufReader<&[u8]> = std::io::BufReader::new(bytes);
    buff.consume(12);

    let mut parsed_objects: Vec<ParsedObject> = Vec::new();
    // Go through all objects in PACK
    for obj_id in 0..object_amount {
        let obj: ParsedObject = parse_object(&mut buff)?;
        parsed_objects.push(obj);
    }

    Ok(parsed_objects)
}

/// Parse single object
fn parse_object(buff: &mut BufReader<&[u8]>) -> Result<ParsedObject> {
    let (obj_size, obj_type_id) = get_size_and_typeid(buff)?;
    let obj_type: Vec<u8> = OBJ_TYPES[obj_type_id as usize].to_vec();

    if 1 <= obj_type_id && obj_type_id <= 4 {
        let mut obj_data: Vec<u8> = Vec::new();
        obj_data.resize(obj_size, 0);
        buff.read_exact(obj_data.as_mut_slice())?;
        Ok(ParsedObject::Default { obj_type, obj_data })
    } else if obj_type_id == 7 {
        let mut obj_ref: [u8; 20] = [0; 20];
        buff.read_exact(&mut obj_ref);
        // Ok(ParsedObject::Delta { obj_type, obj_ref: obj_ref.to_vec(), delta: () })
        Ok(ParsedObject::UNSUPPORTED)
    } else {
        Ok(ParsedObject::UNSUPPORTED)
    }
}

/// Function to consume unary byte from Buff
fn consume_byte(buff: &mut BufReader<&[u8]>) -> Result<u8> {
    if let Some(byte) = buff.bytes().next().transpose()? {
        Ok(byte)
    } else {
        Err(anyhow!("Cannot get byte!"))
    }
}

/// Parse object for size and typeid
fn get_size_and_typeid(buff: &mut BufReader<&[u8]>) -> Result<(usize, u8)> {
    // Parse first byte to get start info
    let fb: u8 = consume_byte(buff)?;
    let typeid: u8 = (fb & 0b01110000_u8) >> 4;
    let mut size: usize = (fb & 0b00001111_u8) as usize;

    let mut bits_to_shift = 4; // First 4 bits are already taken
    loop {
        let byte: u8 = consume_byte(buff)?;
        size += ((byte & 0b01111111_u8) as usize) << bits_to_shift;
        bits_to_shift += 7; // Take 7 free bits and mark them as occupied

        if (byte & 0b10000000_u8) == 0 {
            break;
        }
    }

    Ok((size, typeid))
}