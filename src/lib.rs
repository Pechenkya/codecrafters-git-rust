pub mod commands {
    use anyhow::{ anyhow, Result };
    use std::fs;

    use std::io::prelude::*;
    use std::path::Path;
    use flate2::read::ZlibDecoder;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use sha1::{ Sha1, Digest };
    use hex;

    /// Command to init git repository in current folder
    pub fn init() -> String {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
        "Initialized git directory".to_string()
    }

    fn compute_path_from_sha(sha: &String) -> String {
        let mut prefix_path = String::from("./");
        while !Path::new(&(prefix_path.clone() + ".git")).is_dir() {
            prefix_path += "../";
        }
        prefix_path + ".git/objects/" + &sha[0..2] + "/" + &sha[2..sha.len()]
    }

    fn add_blob_prefix(text: &String) -> String {
        String::from("blob ") + text.len().to_string().as_str() + "\0" + text.as_str()
    }

    /// Open file and print binary data in pretty way
    pub fn cat_file_print(sha: &String) -> Result<String> {
        // Compute path to blob
        let path: String = compute_path_from_sha(sha);

        // Read binary
        let bytes: Vec<u8> = fs::read(path)?;

        // Decompress data and read it to string
        let mut decoder = ZlibDecoder::new(bytes.as_slice());
        let mut buff_string = String::new();
        decoder.read_to_string(&mut buff_string)?;

        // Divide data (header, text)
        if let Some((header, data)) = buff_string.split_once('\0') {
            // Print out object content
            if let Some(("blob", _size)) = header.split_once(' ') {
                Ok(data.to_owned())
            } else {
                Err(anyhow!("Not a blob!"))
            }
        } else {
            Err(anyhow!("Unrecognised file structure!"))
        }
    }

    /// Create a blob from a file
    pub fn hash_object_write(file_path: &String) -> Result<String> {
        // Get data from file and format it according to git rules
        let bytes: Vec<u8> = fs::read(file_path)?;
        let with_header: String = add_blob_prefix(&String::from_utf8(bytes)?);

        // Generate Hash and encode it to hex
        let hash = hex::encode(Sha1::new().chain_update(with_header.as_bytes()).finalize());

        // Compress data
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(with_header.as_bytes())?;
        let encoded_text: Vec<u8> = encoder.finish()?;

        // Create path
        let path_to_save: String = compute_path_from_sha(&hash);
        std::fs::create_dir_all(Path::new(&path_to_save).parent().unwrap())?;

        // Save object
        let mut obj: fs::File = fs::File::create(path_to_save)?;
        obj.write_all(encoded_text.as_slice())?;

        // Print hash
        Ok(hash)
    }

    /// Read a tree object
    pub fn read_tree_names(sha: &String) -> Result<String> {
        // Compute path to object
        let path: String = compute_path_from_sha(sha);

        // Read binary and decompress data
        let bytes: Vec<u8> = fs::read(path)?;
        let mut decoder = ZlibDecoder::new(bytes.as_slice());
        let mut bytes_decoded: Vec<u8> = Vec::new();
        decoder.read_to_end(&mut bytes_decoded)?;

        // Convert to string and divide it into blocks
        let buff_string = String::from_utf8_lossy(bytes_decoded.as_slice());
        let blocks: Vec<_> = buff_string.split('\0').collect();

        // If header block is for correct tree -> parse blocks to find file/folder names
        let mut contents: Vec<&str> = Vec::new();
        if let ("tree", _tree_size) = blocks[0].split_once(' ').unwrap() {
            for text in &blocks[1..blocks.len() - 1] {
                let (_, file_name) = text.rsplit_once(' ').unwrap();
                contents.push(file_name);
            }
        } else {
            return Err(anyhow!("Not a tree object!"));
        }

        // Sort output and print it
        contents.sort();
        let out = contents.join("\n");

        Ok(out)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn check_sha_convert() {
            let sha = String::from("e673d1b7eaa0aa01b5bc2442d570a765bdaae751");
            let path = compute_path_from_sha(&sha);
            assert_eq!(path, "./.git/objects/e6/73d1b7eaa0aa01b5bc2442d570a765bdaae751");
        }
    }
}