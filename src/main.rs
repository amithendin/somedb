extern crate byteorder;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate csv;
extern crate sequencetree;

use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use std::intrinsics::write_bytes;
use threadpool::ThreadPool;
use std::sync::{Arc, RwLock, PoisonError, RwLockReadGuard, RwLockWriteGuard};

use crate::disk::DiskFormat;

mod space;
use space::{Space, Node, Entity, Value};

mod utils;
use utils::*;

mod disk;
use disk::Disk;

mod client;
mod config;

use config::Config;

fn ent_to_json(ent: &Entity, space: &Space, shallowmode: bool) -> String {
    let mut json = String::from("{");

    for (name, id) in &ent.props {
        let prop_str = match space.nodes.get(id) {
            Some(n) => {
                match n {
                    Node::Entity(sub) => {
                        if shallowmode {
                            format!("\"{}\": {},", name, id)
                        }else {
                            format!("\"{}\": {},", name, ent_to_json(sub, space, shallowmode))
                        }
                    },
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

fn _exec_read(space: &RwLockReadGuard<Space>, t: &Transaction, keys: Vec<&str>, ki: usize, curr_obj: usize) -> Vec<u8> {
    let mut id_bytes = vec![0u8; Transaction::UINT_SIZE()];

    if ki > keys.len() - 1 {
        println!("exit 1 {}/{} {:?}", ki, keys.len(), keys);
        return [id_bytes, "null".as_bytes().to_vec()].concat()
    }

    let curr_key = keys[ki];

    let cmd = t.cmd.clone();
    match t.cmd {
        Command::Get | Command::GetRaw => {
            let n = space.get(curr_obj, curr_key);
            println!("{:?} {} '{}'",n,curr_obj,curr_key);
            match n {
                Some((id, node)) => {
                    id_bytes = write_usize(id);
                    match node {
                        Node::Entity(ent) => {
                            if ki == keys.len() - 1 || curr_key.len() == 0 {
                                let str = ent_to_json(ent, &space, cmd == Command::GetRaw);
                                [id_bytes, write_string(str)].concat()
                            }else {
                                _exec_read(space, t, keys, ki+1, id)
                            }
                        }
                        Node::Value(v) => {
                            if ki == keys.len() - 1 || curr_key.len() == 0 {
                                [id_bytes, write_string(v.val.to_owned())].concat()
                            }else {
                                println!("exit 2 {}/{} {:?}", ki, keys.len(), keys);
                                [id_bytes, "null".as_bytes().to_vec()].concat()
                            }
                        }
                    }
                },
                None => { println!("exit 3 {}/{} {:?}", ki, keys.len(), keys); [id_bytes, "null".as_bytes().to_vec()].concat() }
            }
        },
        _ => panic!("wrong function buddy. You need execute_write"),
    }
}

fn execute_read(space: &RwLockReadGuard<Space>, t: &Transaction) -> Vec<u8> {
    _exec_read(space, t, t.key.split(".").collect(), 0, t.obj)
}

fn _exec_write(space: &mut RwLockWriteGuard<Space>, t: &Transaction, keys: Vec<&str>, ki: usize, curr_obj: usize) -> Vec<u8> {
    let mut id_bytes = vec![0u8; Transaction::UINT_SIZE()];

    if ki > keys.len() - 1{
        return [id_bytes, "fail".as_bytes().to_vec()].concat()
    }

    let curr_key = keys[ki];

    if ki == keys.len() - 1 || curr_key.len() == 0 {
        return match t.cmd {
            Command::Create => {

                let new_obj = space.create();
                write_usize(new_obj)
            },
            Command::Set => {
                space.set(curr_obj, curr_key, t.val.as_str());
                [id_bytes, "ok".as_bytes().to_vec()].concat()
            },
            Command::Link => {
                space.link(curr_obj, curr_key, t.othr);
                [id_bytes,"ok".as_bytes().to_vec()].concat()
            },
            _ => panic!("wrong function buddy. You need execute_read")
        }
    }

    let n = space.get(curr_obj, curr_key);
    match n {
        Some((id, node)) => {
            id_bytes = write_usize(id);

            match node {
                Node::Entity(ent) => {
                    _exec_write(space, t, keys, ki+1, id)
                }
                Node::Value(v) => {
                    [id_bytes, "fail".as_bytes().to_vec()].concat()
                }
            }
        },
        None => [id_bytes, "fail".as_bytes().to_vec()].concat()
    }
}

fn execute_write(space: &mut RwLockWriteGuard<Space>, t: &Transaction) -> Vec<u8> {
    _exec_write(space, t, t.key.split(".").collect(), 0, t.obj)
}

fn connection_to_transaction (stream: &mut TcpStream) -> Result<Transaction, String> {
    let mut data_size = 0;
    let mut data_size_buf = [0u8; 8];

    match stream.read_exact(&mut data_size_buf) {
        Ok(_) => {
            data_size = read_usize(&data_size_buf);
            println!("[RX] {} ( {} bytes )", stream.peer_addr().unwrap(), data_size);
        },
        Err(e) => return Err(format!("{}", e))
    };

    let mut data = vec![0u8; data_size];
    match stream.read_exact(&mut data) {
        Ok(_) => Ok(Transaction::from(data)),
        Err(e) => Err(format!("{}", e))
    }
}

fn main() {
    let config = Config::new();
    let mut space = Space::new();
    let disk = Disk::new(config.file_name.as_str(), config.file_format);

    let threadpool = ThreadPool::new(config.threads);

    let space_lock = Arc::new(RwLock::new(space));
    let disk_lock = Arc::new(RwLock::new(disk));

    println!("loading transactions from database file");
    let mut cnt: usize = 0;
    for t in disk_lock.read().unwrap().load_transactions() {
        match t.cmd {
            Command::Get | Command::GetRaw => {
                let readable_space = match space_lock.read() {
                    Ok(s) => s,
                    Err(e) => panic!("Space lock read error {}",e)
                };

                let resp = execute_read(&readable_space, &t);
                resp
            },
            Command::Create | Command::Set | Command::Link => {
                let mut writeable_space = match space_lock.write() {
                    Ok(s) => s,
                    Err(e) => panic!("Space lock write error {}",e)
                };
                let resp = execute_write(&mut writeable_space, &t);

                resp
            }
        };
        cnt += 1;
    }
    println!("loaded {} transactions. starting server", cnt);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(addr.as_str()).unwrap();
    // accept connections and process them, spawning a new thread for each one
    println!("Server listening on port {}", config.port);

    for connection in listener.incoming() {
        let space_lock_clone = Arc::clone(&space_lock);
        let disk_lock_clone = Arc::clone(&disk_lock);

        threadpool.execute(move || {
            let mut stream = match connection {
                Ok(s) => s,
                Err(e) => {
                    println!("Error: {}", e);
                    return;
                }
            };

            let t = match connection_to_transaction(&mut stream) {
                Ok(t) => t,
                Err(e) => {
                    println!("RX error occurred, terminating connection with {} because {}", stream.peer_addr().unwrap(), e);
                    stream.shutdown(Shutdown::Both).unwrap();
                    return;
                }
            };

            let resp = match t.cmd {
                Command::Get | Command::GetRaw => {
                    let readable_space = match space_lock_clone.read() {
                        Ok(s) => s,
                        Err(e) => panic!("Space lock read error {}",e)
                    };

                    let resp = execute_read(&readable_space, &t);
                    resp
                },
                Command::Create | Command::Set | Command::Link => {
                    let mut writeable_space = match space_lock_clone.write() {
                        Ok(s) => s,
                        Err(e) => panic!("Space lock write error {}",e)
                    };

                    let resp = execute_write(&mut writeable_space, &t);

                    match disk_lock_clone.write() {
                        Ok(disk) => disk.log_transaction(&t),
                        Err(e) => panic!("Disk lock write error {}",e)
                    };

                    resp
                }
            };

            println!("[TX] {} ( {} bytes )",stream.peer_addr().unwrap(), resp.len());
            stream.write(resp.as_slice()).unwrap();
        });

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
        //db.set(1, "achivements.1.title", "aob-new");
        println!("{:?}", db.get_str(1, "achivements.0"));
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