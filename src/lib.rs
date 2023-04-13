pub mod commands {
    use anyhow::{ anyhow, Result };
    use std::{ fs, os::unix::prelude::OsStrExt };

    use std::io::prelude::*;
    use std::path::Path;
    use flate2::read::ZlibDecoder;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use sha1::{ Sha1, Digest };
    use hex;

    const BLOB_MODE: &[u8] = "100644 ".as_bytes();
    const TREE_MODE: &[u8] = "40000 ".as_bytes();

    /// Command to init git repository in current folder
    pub fn init() -> String {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
        "Initialized git directory".to_string()
    }

    fn find_root_folder() -> Result<String> {
        let mut prefix_path = String::from("./");
        // Check depth 256 for a .git folder
        for _ in 0..256 {
            if Path::new(&format!("{prefix_path}.git")).is_dir() {
                return Ok(prefix_path);
            }
            prefix_path += "../";
        }

        Err(anyhow!("Cannot find .git folder!"))
    }

    fn compute_path_from_sha(sha: &String) -> Result<String> {
        let path = find_root_folder()? + ".git/objects/" + &sha[0..2] + "/" + &sha[2..sha.len()];
        Ok(path)
    }

    fn add_blob_prefix(text: &String) -> String {
        String::from("blob ") + text.len().to_string().as_str() + "\0" + text.as_str()
    }

    /// Open file and print binary data in pretty way
    pub fn cat_file_print(sha: &String) -> Result<String> {
        // Compute path to blob
        let path: String = compute_path_from_sha(sha)?;

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
    /// Trait AsRef<Path> is for ability to call function with path in [String] or [Path] object
    pub fn hash_object_write<T: AsRef<Path>>(file_path: &T) -> Result<String> {
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
        let path_to_save: String = compute_path_from_sha(&hash)?;
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
        let path: String = compute_path_from_sha(sha)?;

        // Read binary and decompress data
        let bytes: Vec<u8> = fs::read(path)?;
        let mut decoder = ZlibDecoder::new(bytes.as_slice());
        let mut bytes_decoded: Vec<u8> = Vec::new();
        decoder.read_to_end(&mut bytes_decoded)?;

        // Parse text and extract filenames
        let result: Vec<_> = parse_tree(&bytes_decoded)?
            .into_iter()
            .map(|obj| obj.0)
            .collect();
        Ok(result.join("\n"))
    }

    /// Returns tuples (<file name>, <mode>, <SHA-1>) from correct tree object
    fn parse_tree(binary: &Vec<u8>) -> Result<Vec<(String, String, String)>> {
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

    /// Create a tree object from a working directory
    pub fn write_tree() -> Result<String> {
        // Find root folder and create tree starting from it
        let basic_path: String = find_root_folder()?;
        write_tree_with_path(&basic_path)
    }

    /// Recursive function to create subtrees
    fn write_tree_with_path<T: AsRef<Path>>(folder_path: &T) -> Result<String> {
        // Get folder entries and sort them
        let mut entries: Vec<_> = fs
            ::read_dir(folder_path)?
            .map(|e| e.unwrap())
            .collect();
        entries.sort_by_key(|dir| { dir.file_name() });

        // Accumulate data in contents vec
        let mut contents: Vec<u8> = Vec::new();

        // Go trough dir entries
        for entry in entries {
            let e_path = entry.path();
            let file_name = e_path.file_name().unwrap();
            if e_path.is_dir() {
                if e_path.ends_with(".git") {
                    continue; // TODO: Parse .gitignore?
                }

                let mut sub_tree_sha: Vec<u8> = hex::decode(write_tree_with_path(&e_path)?)?;
                contents.append(&mut TREE_MODE.to_vec()); // Add mode prefix
                contents.append(&mut file_name.as_bytes().to_vec()); // Add file name
                contents.push(0_u8); // Add NULL char
                contents.append(&mut sub_tree_sha); // Add tree sha
            } else {
                let mut blob_sha: Vec<u8> = hex::decode(hash_object_write(&e_path)?)?;
                contents.append(&mut BLOB_MODE.to_vec()); // Add mode prefix
                contents.append(&mut file_name.as_bytes().to_vec()); // Add file name
                contents.push(0_u8); // Add NULL char
                contents.append(&mut blob_sha); // Add blob sha
            }
        }

        // Add header to the text
        let mut text: Vec<u8> = (String::from("tree ") + contents.len().to_string().as_str() + "\0")
            .as_bytes()
            .to_vec();
        text.append(&mut contents);

        // Generate Hash and encode it to hex
        let hash = hex::encode(Sha1::new().chain_update(text.as_slice()).finalize());

        // Compress data
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(text.as_slice())?;
        let encoded_text: Vec<u8> = encoder.finish()?;

        // Create path
        let path_to_save: String = compute_path_from_sha(&hash)?;
        std::fs::create_dir_all(Path::new(&path_to_save).parent().unwrap())?;

        // Save object
        let mut obj: fs::File = fs::File::create(path_to_save)?;
        obj.write_all(encoded_text.as_slice())?;

        // Print hash
        Ok(hash)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn check_sha_convert() {
            let sha = String::from("e673d1b7eaa0aa01b5bc2442d570a765bdaae751");
            let path = compute_path_from_sha(&sha).unwrap();
            assert_eq!(path, "./.git/objects/e6/73d1b7eaa0aa01b5bc2442d570a765bdaae751");
        }

        #[test]
        fn check_blob_prefix() {
            let contents = "hello world!".to_string();
            let res = add_blob_prefix(&contents);
            assert_eq!(res, "blob 12\0hello world!");
        }
    }
}