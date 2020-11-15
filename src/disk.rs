use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::io::SeekFrom;

use crate::utils::{Transaction, read_usize, write_usize};
use crate::Space;
use csv::StringRecord;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DiskFormat {
    Bin,
    CSV
}

pub struct Disk {
    path: String,
    format: DiskFormat
}

pub struct DiskIterator {
    offset: u64,
    file: File,
    csv_reader: csv::Reader<File>,
    format: DiskFormat
}

impl DiskIterator {
    fn binary_iterate(&mut self) -> Option<Transaction> {
        self.file.seek(SeekFrom::Start(self.offset)).unwrap();

        let mut t_size_bytes = vec![0u8; Transaction::UINT_SIZE()];
        match self.file.read_exact(&mut t_size_bytes) {
            Ok(_) => {},
            Err(e) => return None
        };
        let t_size = read_usize(t_size_bytes.as_slice());

        if t_size == 0 {
            return None;
        }

        self.file.seek(SeekFrom::Start(self.offset + Transaction::UINT_SIZE() as u64 )).unwrap();//skip the size bytes
        let mut t_bytes = vec![0u8; t_size];
        self.file.read_exact(&mut t_bytes);

        self.offset += (t_size + Transaction::UINT_SIZE()) as u64;
        let t = Transaction::from(t_bytes.to_vec());

        Some(t)
    }

    fn csv_iterate(&mut self) -> Option<Transaction> {
        match self.csv_reader.records().next() {
            Some(t) => Some(t.unwrap().deserialize(None).unwrap()),
            None => None
        }
    }
}

impl Iterator for DiskIterator {
    type Item = Transaction;

    fn next(&mut self) -> Option<Transaction> {
        match self.format {
            DiskFormat::Bin => self.binary_iterate(),
            DiskFormat::CSV => self.csv_iterate()
        }
    }
}

impl Disk {
    pub fn new(path: &str, format: DiskFormat) -> Disk {
        if !Path::new(path).exists() {
            File::create(path.to_owned()).unwrap();
        }

        Disk { path: path.to_string(), format }
    }

    pub fn log_transaction(&self, t: &Transaction) {
        let mut file = match OpenOptions::new().append(true).open(self.path.to_owned()) {
            Err(why) => panic!("couldn't open database file: {}", why),
            Ok(file) => file,
        };

        match self.format {
            DiskFormat::Bin => {
                let t_bytes: Vec<u8> = t.to_bytes();
                let t_size: Vec<u8> = write_usize(t_bytes.len().to_owned());
                let bytes: Vec<u8> = [t_size, t_bytes].concat();
                file.write_all(bytes.as_slice() );
            },
            DiskFormat::CSV => {
                let mut wtr = csv::WriterBuilder::new()
                    .has_headers(false).double_quote(true).from_writer(file);
                wtr.serialize(t);
                wtr.flush();
            }
        };
    }

    pub fn load_transactions(&self) -> DiskIterator {
        match self.format {
            DiskFormat::Bin => {
                DiskIterator {
                    offset: 0,
                    format: self.format.to_owned(),
                    csv_reader: csv::Reader::from_reader(File::open("./null").unwrap()),
                    file: match OpenOptions::new().read(true).open(self.path.to_owned()) {
                        Err(why) => panic!("couldn't open database file: {}", why),
                        Ok(file) => file,
                    }
                }
            },
            DiskFormat::CSV => {
                DiskIterator {
                    offset: 0,
                    format: self.format.to_owned(),
                    csv_reader: csv::ReaderBuilder::new().has_headers(false).double_quote(true).from_reader(match OpenOptions::new().read(true).open(self.path.to_owned()) {
                        Err(why) => panic!("couldn't open database file: {}", why),
                        Ok(file) => file,
                    }),
                    file: File::open("./null").unwrap()
                }
            }
        }
    }

    pub fn clean(&self, space: &Space) {
        println!("not implemented yet");
    }
}