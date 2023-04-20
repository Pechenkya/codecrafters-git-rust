use anyhow::{ anyhow, Result, Ok };
use std::io::prelude::*;

/// Request pack refs from remote repo
pub fn request_refs(repo_url: &str) -> Result<String> {
    // Send blocking request to upload pack
    let mut res = reqwest::blocking::get(
        repo_url.to_owned() + "/info/refs?service=git-upload-pack"
    )?;

    // Check if we get correct response
    if res.status().to_string() != "200 OK" {
        return Err(anyhow!("Cannot reach: {}", res.status()));
    }
    if res.headers()["content-type"] != "application/x-git-upload-pack-advertisement" {
        return Err(anyhow!("Response from host is not valid!"));
    }

    // Read response body
    let mut body: String = String::new();
    res.read_to_string(&mut body)?;

    // Debug purposes
    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());
    // println!("Body:\n{}", body);

    // Return body (list of refs)
    Ok(body.to_string())
}

/// Parse server response into refs -> Returns tuple (<sha-ref vec>, advertised)
pub fn parse_refs_resp_and_check(text: &str) -> Result<(Vec<(String, String)>, String)> {
    // Separate into pkt-lines
    let mut pkt_lines: Vec<String> = text.split('\n').map(String::from).collect();

    // Check response structure according to git rules
    let first_line: String = pkt_lines.remove(0);
    if let Some((first_five, service_resp)) = first_line.split_once(' ') {
        // Check service first line
        if
            first_five.len() != 5 &&
            !first_five[..4].chars().all(|c| c.is_ascii_alphanumeric()) &&
            !first_five.ends_with('#') &&
            service_resp != "service=git-upload-pack"
        {
            return Err(anyhow!("Incorrect service response!"));
        }

        // Server must set last line as 0000, so we just check and remove it
        if let Some(_end_line) = pkt_lines.pop() {
            if _end_line != "0000" {
                return Err(anyhow!("Incorrect response ending!"));
            }

            // Parse first resp line and remove first 4 response bytes
            if let Some((first_ref, additional)) = pkt_lines[0].clone().split_once('\0') {
                pkt_lines[0] = first_ref[4..].to_string();

                // Create list of pairs sha-name and return it
                let result: Vec<(String, String)> = pkt_lines
                    .into_iter()
                    .map(|s| {
                        let (sha, name) = s[4..].split_once(' ').unwrap();
                        (sha.to_string(), name.to_string())
                    })
                    .collect();

                Ok((result, additional.to_string()))
            } else {
                Err(anyhow!("Incorrect response structure!"))
            }
        } else {
            Err(anyhow!("Incorrect response ending!"))
        }
    } else {
        Err(anyhow!("Incorrect response pkt-line structure!"))
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
/// Create request to receive packs
pub fn create_pack_requests(refs: &Vec<(String, String)>) -> Result<String> {
    let mut want_list: Vec<String> = Vec::new();

    // Generate "want" lines
    for (sha, name) in refs {
        want_list.push(format!("want {}", sha));
    }

    Ok("".to_string())
}