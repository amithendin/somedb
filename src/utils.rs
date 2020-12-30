use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use std::io::Cursor;
use serde::{Serialize, Serializer, Deserialize, Deserializer};

const UNIT_SIZE: u8 = 8;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Create,
    Set,
    Get,
    Link,
    GetRaw
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s = u8::deserialize(deserializer)?;
        let command = Command::from(s);
        Ok(command)
    }
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_u8(Command::to_u8(self))
    }
}

impl Command {
    pub fn from(n: u8) -> Command {
        match n {
            0 => Command::Create,
            1 => Command::Set,
            2 => Command::Get,
            3 => Command::Link,
            4 => Command::GetRaw,
            _=> panic!("unsupported command {}", n)
        }
    }

    pub fn to_u8(c: &Command) -> u8 {
        match c {
            Command::Create => 0,
            Command::Set => 1,
            Command::Get => 2,
            Command::Link => 3,
            Command::GetRaw => 4
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    pub cmd: Command,
    pub obj: usize,
    pub key: String,
    pub val: String,
    pub othr: usize
}

impl Transaction {
    pub fn new (cmd: Command, obj: usize, key: String, val: String, othr: usize) -> Transaction {
        Transaction { cmd, obj, key, val, othr }
    }

    pub fn UINT_SIZE() -> usize {
        8//std::mem::size_of::<usize>()
    }

    pub fn from(data: Vec<u8>) -> Transaction {
        let cmd = Command::from( data[0] );

        let usize_size = Transaction::UINT_SIZE();

        if data[0] == 0 {
            Self::new(cmd, 0, String::new(), String::new(), 0)

        }else {
            let obj = read_usize( &data[1 .. (usize_size + 1)] );

            let key_size = read_usize( &data[(usize_size + 1) .. (usize_size*2 + 1)] );
            let key = read_string(&data[(usize_size*2 + 1) .. (usize_size*2 + 1 + key_size)]);

            if data[0] == 2 {
                Self::new(cmd, obj, key, String::new(), 0)

            }else if data[0] == 1 {
                let value_size = read_usize(&data[(usize_size * 2 + 1 + key_size)..(usize_size * 3 + 1 + key_size)]);
                let value = read_string(&data[(usize_size * 3 + 1 + key_size)..(usize_size * 3 + 1 + key_size + value_size)]);

                Self::new(cmd, obj, key, value, 0)

            }else if data[0] == 3 || data[0] == 4 {
                let other_node = read_usize(&data[(usize_size * 3 + 1 + key_size)..(usize_size * 4 + 1 + key_size)]);

                Self::new(cmd, obj, key, String::new(), other_node)

            }else {
                panic!("unsupported command: {}", data[0])
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];

        bytes.push(Command::to_u8(&self.cmd) );

        let mut obj = write_usize(self.obj.to_owned());
        bytes.append(&mut obj);

        let mut key = write_string(self.key.to_owned());
        let mut key_size = write_usize(key.len());
        bytes.append(&mut key_size);
        bytes.append(&mut key);

        let mut value = write_string(self.val.to_owned());
        let mut value_size = write_usize(value.len());
        bytes.append(&mut value_size);
        bytes.append(&mut value);

        let mut other_obj = write_usize(self.othr.to_owned());
        bytes.append(&mut other_obj);

        bytes
    }

    pub fn to_string(&self) -> String {
        format!("{},{},\"{}\",\"{}\",{}\n", Command::to_u8(&self.cmd), self.obj, self.key, self.val, self.othr)
    }
}

pub fn write_usize(n: usize) -> Vec<u8> {
    let mut wtr = vec![];
    wtr.write_u64::<LittleEndian>(n as u64).unwrap();
    wtr
}

pub fn write_string(s: String) -> Vec<u8> {
    s.as_bytes().to_vec()
}

pub fn read_usize(data: &[u8]) -> usize {
    let mut rdr = Cursor::new(data);
    rdr.read_u64::<LittleEndian>().unwrap() as usize
}

pub fn read_string(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}
