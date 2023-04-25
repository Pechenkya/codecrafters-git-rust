mod remote_utility;
mod utility;
mod checkout;

pub mod commands {
    use crate::remote_utility::{ *, pack_processing::UnpackedObject };
    use crate::utility::*;
    use crate::checkout::*;

    use anyhow::{ anyhow, Result };
    use std::{ fs, os::unix::prelude::OsStrExt };
    use std::path::Path;
    use hex;

    // Hardcoded constants
    const BLOB_MODE: &[u8] = b"100644 ";
    const TREE_MODE: &[u8] = b"40000 ";

    /// Command to init git repository in current folder
    pub fn init() -> String {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
        "Initialized git directory".to_string()
    }

    /// Open file and print binary data in pretty way
    pub fn cat_file_print(sha: &String) -> Result<String> {
        // Read object data
        let buff_string: String = String::from_utf8(fs_utility::read_data_decompressed(sha)?)?;

        // Divide data (header, text)
        if let Some((header, data)) = buff_string.split_once('\0') {
            // Print out object content
            if let Some(("blob", _size)) | Some(("commit", _size)) = header.split_once(' ') {
                Ok(data.to_owned())
            } else {
                Err(anyhow!("Not implemented for this object!")) // TODO: Implement for tree
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
        let text: Vec<u8> = other_util::add_data_prefix(b"blob", bytes);

        // Write data into object
        let hash = fs_utility::write_data(text)?;

        // Print hash
        Ok(hash)
    }

    /// Read a tree object
    pub fn read_tree_names(sha: &String) -> Result<String> {
        // Read data from object
        let bytes_decoded: Vec<u8> = fs_utility::read_data_decompressed(sha)?;

        // Parse text and extract filenames
        let result: Vec<_> = other_util
            ::parse_tree(&bytes_decoded)?
            .into_iter()
            .map(|obj| obj.0)
            .collect();
        Ok(result.join("\n"))
    }

    /// Create a tree object from a working directory
    pub fn write_tree() -> Result<String> {
        // Find root folder and create tree starting from it
        let basic_path: String = fs_utility::find_root_folder()?;
        write_tree_with_path(&basic_path)
    }

    /// Recursive function to create subtrees
    fn write_tree_with_path<T: AsRef<Path>>(folder_path: &T) -> Result<String> {
        // Get folder entries and sort them
        let mut entries: Vec<_> = fs
            ::read_dir(folder_path)?
            .filter_map(|e| e.ok())
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
        let text: Vec<u8> = other_util::add_data_prefix(b"tree", contents);

        // Write data into object
        let hash = fs_utility::write_data(text)?;

        // Print hash
        Ok(hash)
    }

    /// Function to create commit from tree
    pub fn create_commit_with_message(
        tree_sha: &String,
        parent_sha: &String,
        message: &String
    ) -> Result<String> {
        // Variable to store commit text
        let mut contents: Vec<u8> = Vec::new();

        // Create timestamp
        let timestamp: String = other_util::get_time_stamp_string()?;

        // Add tree sha
        contents.append(&mut b"tree ".to_vec());
        contents.append(&mut tree_sha.as_bytes().to_vec());
        contents.push(b'\n');

        // Add parent sha
        contents.append(&mut b"parent ".to_vec());
        contents.append(&mut parent_sha.as_bytes().to_vec());
        contents.push(b'\n');

        // Add author and commiter (hardcoded using consts)
        contents.append(&mut b"author ".to_vec());
        other_util::append_commiter_data(&mut contents, &timestamp);
        contents.append(&mut b"commiter ".to_vec());
        other_util::append_commiter_data(&mut contents, &timestamp);

        // Add message
        contents.push(b'\n');
        contents.append(&mut message.as_bytes().to_vec());
        contents.push(b'\n');

        // Add header to the text
        let text: Vec<u8> = other_util::add_data_prefix(b"commit", contents);

        // Write data into object
        let hash = fs_utility::write_data(text)?;

        // Print hash
        Ok(hash)
    }

    /// Command to clone remote repo <repo_url> into folder <folder_path>
    pub fn clone_repo<T: AsRef<Path> + std::fmt::Display>(
        repo_url: &str,
        folder_path: &T
    ) -> Result<String> {
        // Request and parse references
        let response_body: String = remote_communication::request_refs(repo_url)?;
        let (refs_response, aux_resp): (
            Vec<(String, String)>,
            String,
        ) = remote_communication::parse_refs_resp_and_check(&response_body)?;

        // Debug
        // println!("{:?}\n {}", refs_response, aux_resp);

        // Check if we can request packs
        if
            !(
                aux_resp.contains("allow-tip-sha1-in-want") ||
                aux_resp.contains("allow-reachable-sha1-in-want")
            )
        {
            return Err(anyhow!("Server does not advertise required capabilities!"));
        }

        // Create body for a pack request
        let request_body: String = remote_communication::create_pack_request_body(&refs_response)?;
        // Debug
        // println!("{request_body}");

        // contents: [PACK][4 bytes - version][4 bytes - object amount][..heart..][20 bytes - SHA1 checksum]
        let pack_binary: Vec<u8> = remote_communication::send_request_for_packs(
            repo_url,
            &request_body
        )?;
        // Debug
        // println!("{pack_binary:?}");

        // Receive all Objects from PACK
        let objects: Vec<UnpackedObject> = pack_processing::validate_and_get_heart(pack_binary)?;
        // Debug
        // objects.iter().for_each(|obj| println!("{obj}"));

        //
        fs_utility::create_path_and_move_there(folder_path)?;
        init();

        // Write all objects
        for UnpackedObject { obj_type, contents, .. } in objects {
            let text: Vec<u8> = other_util::add_data_prefix(obj_type.as_slice(), contents);
            fs_utility::write_data(text)?;
        }

        // Checkout HEAD
        write_refs(&refs_response)?;
        checkout_head()?;

        Ok(format!("Repository '{repo_url}' succesfully cloned into '{folder_path}'"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[allow(dead_code)]
        const TEST_REPO_1: &str = "https://github.com/codecrafters-io/git-sample-1";
        #[allow(dead_code)]
        const TEST_REPO_2: &str = "https://github.com/codecrafters-io/git-sample-2";
        #[allow(dead_code)]
        const TEST_REPO_3: &str = "https://github.com/codecrafters-io/git-sample-3";

        #[test]
        fn send_request_to_clone() {
            let res = clone_repo(&TEST_REPO_1.to_string(), &"/tmp/clone_repo_test".to_string());
            println!("{:?}", res);
            assert!(res.is_ok());
        }

        #[test]
        fn check_sha_convert() {
            let sha = String::from("e673d1b7eaa0aa01b5bc2442d570a765bdaae751");
            let path = fs_utility::compute_path_from_sha(&sha).unwrap();
            assert_eq!(path, "./.git/objects/e6/73d1b7eaa0aa01b5bc2442d570a765bdaae751");
        }

        #[test]
        fn check_blob_prefix() {
            let contents = "hello world!".as_bytes().to_vec();
            let res = other_util::add_data_prefix(b"blob", contents);
            assert_eq!(res, "blob 12\0hello world!".as_bytes().to_vec());
        }
    }
}