extern crate byteorder;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate csv;

mod space;
use space::{Space, Node, Entity, Value};

mod utils;
use utils::*;

mod disk;
use disk::Disk;

mod client;
mod config;

use config::Config;

fn ent_to_json(ent: &Entity, space: &Space) -> String {
    let mut json = String::from("{");

    for (name, id) in &ent.props {
        let prop_str = match space.nodes.get(id) {
            Some(n) => {
                match n {
                    Node::Entity(sub) => format!("\"{}\": {},", name, ent_to_json(sub, space)),
                    Node::Value(v) => format!("\"{}\":\"{}\",", name, v.val.escape_default().to_owned())
                }
            },
            None => format!("\"{}\": null,", name)
        };

        json.push_str(prop_str.as_str());
    }

    json.pop();
    json.push('}');

    json
}

fn execute_transaction(space: &mut Space, t: &Transaction) -> Vec<u8> {
    let mut id_bytes = vec![0u8; Transaction::UINT_SIZE()];

    match t.cmd {
        Command::Create => {
            let new_obj = space.create();
             write_usize(new_obj)
        },
        Command::Set => {
            space.set(t.obj, t.key.as_str(), t.val.as_str());
            [id_bytes, "ok".as_bytes().to_vec()].concat()
        },
        Command::Get => {
            let n = space.get(t.obj, t.key.as_str());
            match n {
                Some((id, node)) => {
                    id_bytes = write_usize(id);
                    match node {
                        Node::Entity(ent) => {
                            let str = ent_to_json(ent, &space);
                            [id_bytes, write_string(str)].concat()
                        }
                        Node::Value(v) => {
                            [id_bytes, write_string(v.val.to_owned())].concat()
                        }
                    }
                },
                None => [id_bytes, "null".as_bytes().to_vec()].concat()
            }
        },
        Command::Link => {
            space.link(t.obj, t.key.as_str(), t.othr);
            [id_bytes,"ok".as_bytes().to_vec()].concat()
        },

    }
}

use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use std::intrinsics::write_bytes;
use crate::disk::DiskFormat;

fn handle_connection(disk: &Disk, space: &mut Space, mut stream: TcpStream) {
    let mut data_size = 0;
    let mut data_size_buf = [0u8; 8];

    match stream.read_exact(&mut data_size_buf) {
        Ok(_) => {
            data_size = read_usize(&data_size_buf);
            println!("[RX] {} ( {} bytes )", stream.peer_addr().unwrap(), data_size);
        },
        Err(e) => {
            stream.shutdown(Shutdown::Both).unwrap();
            println!("RX error occurred, terminating connection with {} because {}", stream.peer_addr().unwrap(), e);
            return;
        }
    };

    let mut data = vec![0u8; data_size];
    match stream.read_exact(&mut data) {
        Ok(_) => {
            let t = Transaction::from(data);
            let resp = execute_transaction(space, &t);
            disk.log_transaction(&t);

            println!("[TX] {} ( {} bytes )",stream.peer_addr().unwrap(), resp.len());
            stream.write(resp.as_slice()).unwrap();
        },
        Err(e) => {
            println!("RX error occurred, terminating connection with {} because {}", stream.peer_addr().unwrap(), e);
            stream.shutdown(Shutdown::Both).unwrap();
        }
    }
}

fn main() {
    let config = Config::new();
    let mut space = Space::new();
    let disk = Disk::new(config.file_name.as_str(), config.file_format);

    println!("loading transactions from database file");
    let mut cnt: usize = 0;
    for t in disk.load_transactions() {
        execute_transaction(&mut space, &t);
        cnt += 1;
    }
    println!("loaded {} transactions. starting server", cnt);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(addr.as_str()).unwrap();
    // accept connections and process them, spawning a new thread for each one
    println!("Server listening on port {}", config.port);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // connection succeeded
                handle_connection(&disk, &mut space, stream);
            }
            Err(e) => {
                println!("Error: {}", e);
                /* connection failed */
            }
        }
    }
    // close the socket server
    drop(listener);
}

#[cfg(test)]
mod tests {
    use crate::client::Client;
    use crate::utils::*;
    use serde_json::Value;
    
    #[test]
    fn fetch() {
        let db = Client::new("localhost:4000");
        println!("{}", db.get_obj(1).1);
    }

    #[test]
    fn save_json() {
        let db = Client::new("localhost:4000");

        fn encode_json(obj: Value, db: &Client) -> String {
            match obj {
                Value::String(val) => val,
                Value::Bool(val) => val.to_string(),
                Value::Number(val) => val.to_string(),
                Value::Null => String::from("null"),
                Value::Array(arr) => {
                    let dbarr = db.create();

                    let mut i= 0;
                    for val in arr {
                        let v = encode_json(val.to_owned(), db);
                        let key = format!("{}", i);

                        match val {
                            Value::Object(_) => db.link(dbarr, key.as_str(), v.parse::<usize>().unwrap()),
                            Value::Array(_) => db.link(dbarr, key.as_str(), v.parse::<usize>().unwrap()),
                            _=> db.set(dbarr, key.as_str(), v.as_str())
                        };

                        i += 1;
                    }

                    dbarr.to_string()
                },
                Value::Object(obj) => {
                    let dbobj: usize = db.create();

                    for (key, val) in obj {
                        let v = encode_json(val.to_owned(), db);

                        match val {
                            Value::Object(_) | Value::Array(_) => db.link(dbobj, key.as_str(), v.parse::<usize>().unwrap()),
                            _=> db.set(dbobj, key.as_str(), v.as_str())
                        };
                    }

                    dbobj.to_string()
                }
            }
        }

        let val = json!({
            "name": "amit",
            "proffesion": "developer \"lol\"\t\n\"kk\"",
            "age": 24,
            "stats": {
                "employment": "null",
                "thebest": true
            },
            "achivements": [
                {
                    "title": "somedb",
                    "type": "database",
                    "hits": 45312
                },
                {
                    "title": "aob",
                    "type": "videogame / 2d dungeon \"crawler\" ",
                    "hits": 100234
                },
                {
                    "title": "sequencetree",
                    "type": "collection",
                    "hits": 7105421
                }
            ]
        });

        let val2 = json!({
            "name": "advert one",
            "keywords": "abc def ghi jkl mno",
            "timestamp": 442314,
            "stats": {
                "clicks": 43304,
                "impr": 786,
                "conv": 112
            }
        });

        let id = encode_json(val, &db);

        println!("object id = {}", id);
    }

    #[test]
    fn client() {
        let db = Client::new("localhost:4000");
        let obj = db.create();
        println!("obj {}", obj);

        let sub_obj = db.create();
        println!("sub_obj {:?}", sub_obj);
        println!("set {:?}", db.set(sub_obj, "age", "55"));
        println!("set {:?}", db.set(sub_obj, "name", "timothy \"the greate\" bourn"));

        println!("link {:?}", db.link(obj, "child", sub_obj));

        println!("get {:?}", db.get_str(obj, "child"));
    }

    #[test]
    fn serialize() {
        let t = Transaction::new(Command::from(3), 15453332589748683533, "child".to_owned(), String::new(), 8693387624441552404);
        let nt = t.to_bytes();
        let gt = Transaction::from(nt);

        println!("t = {:?}\ngt = {:?}",t, gt);

        assert!(true);
    }
}