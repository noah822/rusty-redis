use std::sync::{Arc, Mutex};
use std::collections::HashMap;

type ThreadSafeHmap<K, V> = Arc<Mutex<HashMap<K, V>>>;



#[derive(Clone)]
pub struct RedisStorage{
    pub data: ThreadSafeHmap<Box<[u8]>, Box<[u8]>>
}


impl RedisStorage{
    pub fn new() -> Self{
        let data = Arc::new(Mutex::new(HashMap::new()));
        Self {data}
    }


    pub fn from_hashmap(data: HashMap<Box<[u8]>, Box<[u8]>>) -> Self{
        let data = Arc::new(Mutex::new(data));
        Self {data}
    }
}
