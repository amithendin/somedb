use std::collections::HashMap;
use std::thread::sleep;

#[derive(Debug)]
pub struct Entity {
    pub props: HashMap<String, usize>
}

#[derive(Debug)]
pub struct Value {
    pub val: String
}

#[derive(Debug)]
pub enum Node {
    Entity(Entity),
    Value(Value)
}

pub struct Space {
    pub nodes: HashMap<usize, Node>,
    pub reverse: HashMap<String, usize>,
    id_cnt: usize
}

impl Space {
    pub fn new() -> Space {
        Space {
            nodes: HashMap::new(),
            reverse: HashMap::new(),
            id_cnt: 0
        }
    }

    fn gen_id(&mut self) -> usize {
        self.id_cnt += 1;
        self.id_cnt
    }

    pub fn create(&mut self) -> usize {

        let id = self.gen_id();
        //println!("created object with id {}",id);
        self.nodes.insert(id, Node::Entity( Entity { props: HashMap::new() } ) );

        id
    }

    fn create_prop(&mut self, val: String) -> usize {
        let id = self.gen_id();

        self.reverse.insert(val.to_owned(), id);
        self.nodes.insert(id, Node::Value( Value { val } ) );

        id
    }

    fn upsert_prop(&mut self, value: &str) -> usize {
        match self.reverse.get(value) {
            Some(id) => id.to_owned(),
            None => self.create_prop(value.to_string())
        }
    }

    pub fn set(&mut self, obj: usize, key: &str, value: &str) {
        //println!("set {} to obj {} by key {}", value, obj, key);
        let prop = self.upsert_prop(value);

        let mut node = match self.nodes.get_mut(&obj) {
            Some(node) => node,
            None => panic!("No such object ({})", obj)
        };
        //println!("{:?}",node);
        let ent = match node {
            Node::Entity(ent) => ent,
            Node::Value(val) => panic!("Id {} links to a value ({})", obj, val.val)
        };

        ent.props.insert(key.to_string(), prop);
    }

    pub fn link(&mut self, obj: usize, key: &str, other_node: usize) {
        //println!("link {} to obj {} by key {}", other_node, obj, key);
        let mut node = match self.nodes.get_mut(&obj) {
            Some(node) => node,
            None => panic!("No such object ({})", obj)
        };

        let ent = match node {
            Node::Entity(ent) => ent,
            Node::Value(val) => panic!("Id {} links to a value ({})", obj, val.val)
        };

        ent.props.insert(key.to_string(), other_node);
    }

    pub fn get(&self, obj: usize, key: &str) -> Option<(usize, &Node)> {
        match self.nodes.get(&obj) {
            Some(node) => {
                if key.len() == 0 {
                    Some((obj, node))
                }else {
                    match node {
                        Node::Entity(ent) => {
                            match ent.props.get(key) {
                                Some(id) => match self.nodes.get(id) {
                                    Some(n) => Some((id.to_owned(), n)),
                                    None => None
                                },
                                None => None
                            }
                        },
                        Node::Value(_) => None
                    }
                }
            },
            None => None
        }
    }
}