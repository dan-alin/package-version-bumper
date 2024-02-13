use serde_json::Value;
use std::env;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};

fn main() -> std::io::Result<()> {
    // Open the JSON file for reading and writing
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("package.json")?;
    let mut reader = BufReader::new(&file);

    // Parse the JSON content into a serde_json::Value object
    let mut json_content = String::new();
    reader.read_to_string(&mut json_content)?;
    let package: Value = serde_json::from_str(&json_content)?;

    // Retrieve command-line arguments
    let args: Vec<String> = env::args().collect();

    // Extract the version from the JSON object
    let mut new_version = match package.get("version") {
        Some(version) => version.as_str().unwrap_or("0.0.1").to_string(),
        None => {
            println!("'version' field not found in package.json.");
            return Ok(());
        }
    };

    // Determine which part of the version to update based on CLI arguments
    if args.len() > 1 {
        let version_part = &args[1]; // Assuming the first argument specifies which part to update

        match version_part.as_str() {
            "major" => {
                // Update major version
                // Logic to increment major version
                new_version = increment_major_version(&new_version); // Implement your logic here
                println!("new version {}", new_version);
            }
            "minor" => {
                // Update minor version
                // Logic to increment minor version
                new_version = increment_minor_version(&new_version); // Implement your logic here
            }
            "patch" => {
                // Update patch version
                // Logic to increment patch version
                new_version = increment_patch_version(&new_version); // Implement your logic here
            }
            _ => {
                println!("Invalid version part specified. Usage: ./program_name major|minor|patch");
                return Ok(());
            }
        }
    }
    // Find the position of the line containing the "version" key
    let mut line = String::new();
    let mut current_position = 0;
    let mut version_line_position = None;
    while reader.read_line(&mut line)? > 0 {
        println!("{}", line);
        if line.trim().starts_with(r#""version""#) {
            // Record the position of the line containing the version
            version_line_position = Some(current_position);
            break;
        }
        current_position += line.len() as u64;
        line.clear();
    }

    // If "version" line found, update it
    if let Some(position) = version_line_position {
        // Seek to the position of the "version" line
        file.seek(SeekFrom::Start(position))?;

        // Write the new version value
        file.write_all(format!("\"version\": \"{}\"", new_version).as_bytes())?;
    } else {
        println!("No line containing \"version\" found in the file.");
    }

    Ok(())
}

fn increment_major_version(current_version: &str) -> String {
    // Logic to increment major version
    // Example: Splitting version string and incrementing major version
    let parts: Vec<&str> = current_version.split('.').collect();
    let major: u64 = parts[0].parse().unwrap_or(0);
    format!("{}.{}.{}", major + 1, 0, 0)
}

fn increment_minor_version(current_version: &str) -> String {
    // Logic to increment minor version
    let parts: Vec<&str> = current_version.split('.').collect();
    let major: u64 = parts[0].parse().unwrap_or(0);
    let minor: u64 = parts[1].parse().unwrap_or(0);
    format!("{}.{}.{}", major, minor + 1, 0)
}

fn increment_patch_version(current_version: &str) -> String {
    // Logic to increment patch version
    // Example: Splitting version string and incrementing patch version
    let parts: Vec<&str> = current_version.split('.').collect();
    let major: u64 = parts[0].parse().unwrap_or(0);
    let minor: u64 = parts[1].parse().unwrap_or(0);
    let patch: u64 = parts[2].parse().unwrap_or(0);
    format!("{}.{}.{}", major, minor, patch + 1)
}
