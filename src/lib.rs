pub mod commands {
    use anyhow::Result;
    use std::fs;

    use std::io::prelude::*;
    use flate2::read::ZlibDecoder;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use sha1::{ Sha1, Digest };
    use hex;

    /// Command to init git repository in current folder
    pub fn init() {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
        println!("Initialized git directory")
    }

    fn compute_path_from_sha(sha: &String) -> String {
        String::from(".git/objects/") + &sha[0..2] + "/" + &sha[2..sha.len()]
    }

    fn add_blob_prefix(text: &String) -> String {
        String::from("blob ") + text.len().to_string().as_str() + "\0" + text.as_str()
    }

    /// Open file and print binary data in pretty way
    pub fn cat_file_print(sha: &String) -> Result<()> {
        // Compute path to blob
        let path: String = compute_path_from_sha(sha);

        // Read binary
        let bytes: Vec<u8> = fs::read(path)?;

        // Decompress data and read it to string
        let mut decoder = ZlibDecoder::new(bytes.as_slice());
        let mut buff_string = String::new();
        decoder.read_to_string(&mut buff_string)?;

        // Divide data (header, text)
        if let Some((_, data)) = buff_string.split_once('\x00') {
            // Print out file content
            print!("{data}");
        }

        Ok(())
    }

    /// Create a blob from a file
    pub fn hash_object_write(file_path: &String) -> Result<()> {
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
        std::fs::create_dir_all(std::path::Path::new(&path_to_save).parent().unwrap())?;

        // Save object
        let mut obj: fs::File = fs::File::create(path_to_save)?;
        obj.write_all(encoded_text.as_slice())?;

        // Print hash
        println!("{hash}");
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn check_sha_convert() {
            let sha = String::from("e673d1b7eaa0aa01b5bc2442d570a765bdaae751");
            let path = compute_path_from_sha(&sha);
            assert_eq!(path, ".git/objects/e6/73d1b7eaa0aa01b5bc2442d570a765bdaae751");
        }
    }
}