use anyhow::{ anyhow, bail, Result, Ok };
use reqwest::blocking::{ Response, Client, self };
use std::io::prelude::*;

/// Request pack refs from remote repo
pub fn request_refs(repo_url: &str) -> Result<String> {
    // Send blocking request to upload pack
    let mut res = blocking::get(repo_url.to_owned() + "/info/refs?service=git-upload-pack")?;

    // Check if we get correct response
    if res.status() != 200 {
        bail!("Cannot reach: {}", res.status());
    }
    if res.headers()["content-type"] != "application/x-git-upload-pack-advertisement" {
        bail!("Response from host is not valid!");
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
            bail!("Incorrect service response!");
        }

        // Server must set last line as 0000, so we just check and remove it
        if let Some(_end_line) = pkt_lines.pop() {
            if _end_line != "0000" {
                bail!("Incorrect response ending!");
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

/// Create request body to receive packs
pub fn create_pack_request_body(refs: &[(String, String)]) -> Result<String> {
    let mut want_list: String = String::new();

    // Generate "want" lines
    // Add capabilitiy 'multi_ack' to the first ref to allow server find last diff
    let first_line = format!("want {} multi_ack\n", refs[0].0);
    want_list.push_str(format!("{:04x}{}", first_line.len() + 4, first_line).as_str());
    for (sha, _name) in &refs[1..] {
        let want_line = format!("want {}\n", sha);
        want_list.push_str(format!("{:04x}{}", want_line.len() + 4, want_line).as_str());
    }
    // Final lines fixed
    want_list.push_str("0000");
    want_list.push_str(format!("{:04x}done\n", 9).as_str());

    Ok(want_list)
}

/// Send request to recieve packs (return binary returned from the HOST)
pub fn send_request_for_packs(repo_url: &str, request_body: &str) -> Result<Vec<u8>> {
    let request_url: String = format!("{}/git-upload-pack", repo_url);

    let client = Client::new();
    let mut res: Response = client
        .post(request_url)
        .header("content-type", "application/x-git-upload-pack-request")
        .body(request_body.to_owned())
        .send()?;

    // Read response body
    let mut body: Vec<u8> = Vec::new();
    res.read_to_end(&mut body)?;

    // Debug
    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());
    // println!("Body:\n{:?}", String::from_utf8_lossy(&body));

    // Return PACK body
    Ok(body[8..].to_vec())
}