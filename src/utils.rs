use std::fs::{self, read_dir};
use std::io::{Read, Write};
use std::path::Path;
use std::{
    fs::{remove_dir, remove_file, File},
    io::BufReader,
};

use anyhow::{format_err, Result};
use solana_sdk::signature::Keypair;

pub fn validate_url_address(url: &str) -> Result<(), String> {
    if url.trim().len() != url.len() {
        Err(String::from("URL cannot have leading and trailing space"))
    } else if !url.starts_with("http") {
        Err(String::from("URL should start with http or https prefix"))
    } else {
        Ok(())
    }
}

pub fn validate_input_for_space(input: &str) -> Result<(), String> {
    if input.trim().len() != input.len() {
        Err(String::from("Input cannot have leading and trailing space"))
    } else {
        Ok(())
    }
}

pub fn read_keypair_file(s: &str) -> Result<Keypair> {
    solana_sdk::signature::read_keypair_file(s)
        .map_err(|_| format_err!("Failed to read keypair from {}", s))
}

pub fn write_keypair_file(keypair: &Keypair, outfile: &str) -> Result<String> {
    solana_sdk::signature::write_keypair_file(keypair, outfile)
        .map_err(|_| format_err!("Failed to write keypair to {:?}", outfile))
}

pub fn write_file(dir_name: &str, file_name: &str, content: &str) -> Result<(), String> {
    let dir_name = dir_name.trim();
    let file_name = file_name.trim();
    let content = content.trim();

    if dir_name.is_empty() {
        return Err(String::from("The given dir name is empty"));
    }

    if file_name.is_empty() {
        return Err(String::from("The given file name is empty"));
    }

    if content.is_empty() {
        return Err(String::from("The given content is empty"));
    }

    if !Path::new(dir_name).exists() {
        if let Err(err) = fs::create_dir(dir_name) {
            return Err(err.to_string());
        }
    }

    let path = Path::new(dir_name).join(file_name);
    let mut output = match File::create(path) {
        Ok(it) => it,
        Err(err) => return Err(err.to_string()),
    };

    let result = output.write_all(content.as_bytes());

    if result.is_ok() {
        Ok(())
    } else {
        Err(String::from(
            "The given content was not written into a file",
        ))
    }
}

pub fn read_file(path: &str) -> Result<String, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err(String::from("The given path is empty"));
    }

    if !Path::new(path).exists() {
        let msg = "The given path dosn't exist: ".to_string() + path;
        return Err(msg);
    }

    let input = File::open(path).unwrap();
    let mut buffered = BufReader::new(input);

    let mut content = String::new();
    let result = buffered.read_to_string(&mut content);

    if content.is_empty() {
        return Err(String::from("The content is empty"));
    }

    if result.is_ok() {
        Ok(content)
    } else {
        Err(String::from("Failed to read file"))
    }
}

pub fn remove_dir_and_files(dir_name: &str) -> Result<(), String> {
    if dir_name.is_empty() {
        return Err("The given dir name is empty!".to_string());
    }

    if !Path::new(dir_name).exists() {
        return Err("The given dir doesn't exist!".to_string());
    }

    if !Path::new(dir_name).is_dir() {
        return Err("The given dir name isn't a directory!".to_string());
    }

    for entry in read_dir(dir_name).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            remove_file(path).unwrap();
        }
    }

    let is_empty_dir = Path::new(dir_name).read_dir().unwrap().next().is_none();
    if is_empty_dir {
        if let Err(err) = remove_dir(dir_name) {
            return Err(err.to_string());
        }
    }

    Ok(())
}

pub fn is_initialized(dir_name: &str) -> bool {
    Path::new(dir_name).exists() && Path::new(dir_name).is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_address_ok() {
        let url = "http://localhost:8899";
        let result = validate_url_address(url);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_validate_url_address_failed() {
        let url = "localhost:8899";
        let result = validate_url_address(url);
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_validate_input_for_space_ok() {
        let input = "some input";
        let result = validate_input_for_space(input);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_validate_input_for_space_failed() {
        let input = "  ";
        let result = validate_input_for_space(input);
        assert_eq!(result.is_err(), true);

        let input = " some other input";
        let result = validate_input_for_space(input);
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_read_keypair_file_ok() {
        let outfile = "test_read_keypair_file_ok.json";
        let keypair = Keypair::new();
        let result = write_keypair_file(&keypair, outfile);

        if result.is_ok() {
            let result = read_keypair_file(outfile);
            assert_eq!(result.is_ok(), true);

            let result = remove_file(Path::new(outfile));
            assert_eq!(result.is_ok(), true);
        } else {
            panic!("test failed, check implementation of write_keypair_file function and its params for correctness");
        }
    }

    #[test]
    fn test_read_keypair_file_failed() {
        let outfile = "";
        let keypair = Keypair::new();
        let result = write_keypair_file(&keypair, outfile);

        assert_eq!(result.is_err(), true);

        let outfile = "test_read_keypair_file_failed.json";
        let keypair = Keypair::new();
        let result = write_keypair_file(&keypair, outfile);

        if result.is_ok() {
            let outfile = "";
            let result = read_keypair_file(outfile);
            assert_eq!(result.is_err(), true);

            let outfile = "test_read_keypair_file_failed.json";
            if Path::new(outfile).exists() {
                let result = remove_file(Path::new(outfile));
                assert_eq!(result.is_ok(), true);
            }
        } else {
            panic!("test failed, check implementation of write_keypair_file function and its params for correctness");
        }
    }

    #[test]
    fn test_write_keypair_file_ok() {
        let outfile = "test_write_keypair_file_ok.json";
        let keypair = Keypair::new();
        let result = write_keypair_file(&keypair, outfile);
        assert_eq!(result.is_ok(), true);

        let result = remove_file(Path::new(outfile));
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_write_keypair_file_failed() {
        let outfile = "";
        let keypair = Keypair::new();
        let result = write_keypair_file(&keypair, outfile);
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_write_file_ok() {
        let dir_name = "test_write_file_ok";
        let file_name = "test_write_file_ok.json";
        let result = write_file(dir_name, file_name, "market_pubkey");
        assert_eq!(result.is_ok(), true);

        if result.is_ok() {
            let result = remove_dir_and_files(dir_name);
            assert_eq!(result.is_ok(), true);
        }
    }

    #[test]
    fn test_write_file_failed() {
        let dir_name = "test_write_file_failed";
        let outfile = "test_write_file_failed.json";
        let result = write_file("", "", "market_pubkey");
        assert_eq!(result.is_err(), true);

        let result = write_file("", outfile, "market_pubkey");
        assert_eq!(result.is_err(), true);

        let result = write_file(dir_name, " ", "market_pubkey");
        assert_eq!(result.is_err(), true);

        let result = write_file(dir_name, outfile, "");
        assert_eq!(result.is_err(), true);

        let result = write_file(dir_name, outfile, "   ");
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_read_file_ok() {
        let dir_name = "test_read_file_ok";
        let outfile = "test_read_file_ok.json";
        let result = write_file(dir_name, outfile, "market_pubkey");

        if result.is_ok() {
            let path = dir_name.to_string() + "/" + outfile;
            let result = read_file(path.as_str());
            assert_eq!(result.is_ok(), true);

            let result = remove_dir_and_files(dir_name);
            assert_eq!(result.is_ok(), true);
        } else {
            panic!("Unable to create a file");
        }
    }

    #[test]
    fn test_read_file_failed() {
        let dir_name = "test_read_file_failed";
        let outfile = "test_read_file_failed.json";
        let result = write_file(" ", "", "market_pubkey");
        assert_eq!(result.is_err(), true);

        let result = write_file(dir_name, outfile, "market_pubkey");
        if result.is_ok() {
            let result = read_file(" ");
            assert_eq!(result.is_err(), true);

            let result = remove_dir_and_files(dir_name);
            assert_eq!(result.is_ok(), true);
        } else {
            panic!("Unable to create a file: {:?}", result.err().unwrap());
        }

        let path = dir_name.to_string() + "/"  + outfile;
        let result = read_file(path.as_str());
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_remove_dir_and_files_ok() {
        let dir_name = "test_remove_dir_and_files_ok";
        let file_name1 = "test_remove_dir_and_files_ok_1.json";
        let file_name2 = "test_remove_dir_and_files_ok_2.json";
        let file_name3 = "test_remove_dir_and_files_ok_3.json";

        let result = write_file(dir_name, file_name1, "test_remove_dir_and_files_ok_1");
        assert_eq!(result.is_ok(), true);

        let result = write_file(dir_name, file_name2, "test_remove_dir_and_files_ok_2");
        assert_eq!(result.is_ok(), true);

        let result = write_file(dir_name, file_name3, "test_remove_dir_and_files_ok_3");
        assert_eq!(result.is_ok(), true);

        let result = remove_dir_and_files(dir_name);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_remove_dir_and_files_failed() {
        let dir_name = "";
        let result = remove_dir_and_files(dir_name);
        assert_eq!(result.is_err(), true);
    }
}
