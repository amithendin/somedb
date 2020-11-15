use std::net::{TcpStream};
use std::io::{Read, Write};
use std::str::from_utf8;

use crate::utils::*;
use std::error::Error;

pub struct Client {
    addr: String
}

impl Client {
    pub fn new(addr: &str) -> Client {
        Client {
            addr: addr.to_owned()
        }
    }

    fn send(&self, cmd: u8, obj: usize, key: String, val: String, other_obj: usize) -> Result<Vec<u8>, String> {
        let t = Transaction::new(Command::from(cmd), obj, key, val, other_obj);

        match TcpStream::connect(self.addr.as_str()) {
            Ok(mut stream) => {
                let mut data = t.to_bytes();
                let mut data_size = write_usize(data.len());
                data_size.append(&mut data);

                stream.write(&data_size).unwrap();

                let mut data = [0 as u8; 1000];

                match stream.read(&mut data) {
                    Ok(size) => {
                        Ok(data[0..size].to_vec())
                    },
                    Err(e) => Err(format!("{}", e))
                }
            },
            Err(e) => Err(format!("{}", e))
        }
    }

    pub fn create(&self) -> usize {
        match self.send(0, 0, String::new(), String::new(), 0) {
            Ok(bytes) => read_usize(bytes.as_slice()),
            Err(e) => panic!(e)
        }
    }

    pub fn set(&self, obj: usize, key: &str, val: &str) -> (usize, String) {
        match self.send( 1, obj, key.to_owned(), val.to_owned(), 0) {
            Ok(bytes) => {
                let id_bytes = &bytes[..Transaction::UINT_SIZE()];
                let id = read_usize(id_bytes);
                (id, from_utf8(&bytes[Transaction::UINT_SIZE()..]).unwrap().to_string())
            },
            Err(e) => panic!(e)
        }
    }

    pub fn get_str(&self, obj: usize, key: &str) -> (usize, String) {
        match self.send( 2, obj, key.to_owned(), String::new(), 0) {
            Ok(bytes) => {
                let id_bytes = &bytes[..Transaction::UINT_SIZE()];
                let id = read_usize(id_bytes);
                (id, from_utf8(&bytes[Transaction::UINT_SIZE()..]).unwrap().to_string())
            },
            Err(e) => panic!(e)
        }
    }

    pub fn get_usize(&self, obj: usize, key: &str) -> (usize, String) {
        match self.send( 2, obj, key.to_owned(), String::new(), 0) {
            Ok(bytes) => {
                let id_bytes = &bytes[..Transaction::UINT_SIZE()];
                let id = read_usize(id_bytes);
                (id, read_usize(bytes.as_slice()).to_string())
            },
            Err(e) => panic!(e)
        }
    }

    pub fn get_obj(&self, obj: usize) -> (usize, String) {
        match self.send( 2, obj, String::new(), String::new(), 0) {
            Ok(bytes) => {
                let id_bytes = &bytes[..Transaction::UINT_SIZE()];
                let id = read_usize(id_bytes);
                (id, from_utf8(&bytes[Transaction::UINT_SIZE()..]).unwrap().to_string())
            },
            Err(e) => panic!(e)
        }
    }

    pub fn link(&self, obj: usize, key: &str, othr: usize) -> (usize, String) {
        match self.send( 3, obj, key.to_owned(), String::new(), othr) {
            Ok(bytes) => {
                let id_bytes = &bytes[..Transaction::UINT_SIZE()];
                let id = read_usize(id_bytes);
                (id, from_utf8(&bytes[Transaction::UINT_SIZE()..]).unwrap().to_string())
            },
            Err(e) => panic!(e)
        }
    }
}