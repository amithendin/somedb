use crate::disk::DiskFormat;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::Path;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub file_name: String,
    pub file_format: DiskFormat,
    pub port: u32
}


impl Config {
    pub fn new () -> Config {
        let path = "./config.json";

        if !Path::new(path).exists() {
            let config = Config {
                file_name: "./db.bin".to_owned(),
                file_format: DiskFormat::Bin,
                port: 4000
            };

            match OpenOptions::new().create(true).write(true).open(path.to_owned()) {
                Err(why) => panic!("couldn't open config file: {}", why),
                Ok(mut file) => {
                    file.write_all(serde_json::to_string_pretty(&config).unwrap().as_bytes());
                }
            };

            config

        }else {
            let file = match File::open(path) {
                Ok(file) => file,
                Err(why) => panic!("couldn't open config file: {}", why)
            };

            let reader = BufReader::new(file);
            let config: Config = serde_json::from_reader(reader).unwrap();

            config
        }
    }
}