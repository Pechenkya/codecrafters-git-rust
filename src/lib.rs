pub mod commands {
    use anyhow::Result;
    use std::fs;

    use std::io::prelude::*;
    use flate2::read::ZlibDecoder;

    /// Command to init git repository in current folder
    pub fn init() {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
        println!("Initialized git directory")
    }

    fn compute_path_from_hash(hash: &String) -> String {
        String::from(".git/objects/") + &hash[0..2] + "/" + &hash[2..hash.len()]
    }

    /// Open file and print binary data in pretty way
    pub fn cat_file_pretty(hash: &String) -> Result<()> {
        // Compute path to blob
        let path = compute_path_from_hash(hash);

        // Read binary
        let bytes = std::fs::read(path)?;

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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn check_hash_convert() {
            let hash = String::from("e673d1b7eaa0aa01b5bc2442d570a765bdaae751");
            let path = compute_path_from_hash(&hash);
            assert_eq!(path, ".git/objects/e6/73d1b7eaa0aa01b5bc2442d570a765bdaae751");
        }
    }
}