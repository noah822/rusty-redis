use std::collections::{HashMap, VecDeque, HashSet};
use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddr, ToSocketAddrs};
use std::sync::{mpsc, Mutex, Arc};

use rand::Rng;


use crate::{command, parser};
use crate::parser::{
    RESPObject, SimpleRESPObject, AggrRESPObject, AtomicItem
};
use crate::persistence::RedisStorage;

use self::master::nod_replica;



#[derive(Clone, Debug)]
pub enum ServerType{
    Master,
    Slave
}


#[derive(Clone, Debug)]
pub struct LaunchConfig{
    pub binding_addr: String,
    pub server_type: ServerType,
    pub replicaof: Option<(String, String)>,
    pub replica_id: Option<Vec<u8>>
} 



impl LaunchConfig {
    fn new() -> Self{
        Self{
          binding_addr: String::from("localhost:6379"),
          server_type: ServerType::Master,
          replicaof: None,
          replica_id: None 
        }
    }
}

// not only stores cmd but also their parameters

type CmdCallback<'a> = Box<dyn Fn(Vec<Vec<String>>) -> Option<Box<[u8]>> + 'a>;
pub struct CommandCache<'a>{
    // cache fixed sized command history and invoke callback accordingly
    size: usize,
    buf: VecDeque<Vec<String>>,
    callbacks: Vec<(VecDeque<Vec<String>>, CmdCallback<'a>)>
}

pub trait ToCmdCache{
    fn to_vec_string(&self) -> Vec<String>;
}

impl ToCmdCache for &str{
    fn to_vec_string(&self) -> Vec<String>{
        self.to_lowercase().split(' ')
               .filter(|chunk| {chunk.len() > 0})
               .map(|s| {String::from(s)})
               .collect::<Vec<_>>()
    }
}

impl ToCmdCache for &[u8]{
    fn to_vec_string(&self) -> Vec<String>{
        std::str::from_utf8(&self).unwrap().to_vec_string()
    }
}

impl<T: ToString> ToCmdCache for Vec<T>
{
   fn to_vec_string(&self) -> Vec<String>{
       self.iter().map(|item| {item.to_string().to_lowercase()}).collect::<Vec<_>>()
   } 
}


impl<'a> CommandCache<'a>{
    pub fn new(size: usize) -> Self{
        Self{size, buf: VecDeque::new(), callbacks: Vec::new()}
    }

    pub fn push<T: ToCmdCache>(&mut self, cmd: T) -> Option<Box<[u8]>>{
        if self.buf.len() >= self.size {self.buf.pop_front();}
        self.buf.push_back(cmd.to_vec_string());

        // linear search 
        for (target_seq, callback) in self.callbacks.iter(){
            let target_len = target_seq.len();
            // do token prefix matching for callbacks[prefix] && buf
            if self.buf.len() >= target_len{
               let token_window = self.buf.range(self.buf.len()-target_len..); 
               if token_window.zip(target_seq.iter())
                    .all(|(tk, ta)|{
                        println!("{:?}", ta);
                        if tk.len() < ta.len(){
                            false
                        }else{
                            &tk[..ta.len()] == ta
                        }
                    }){
                        let mut cmd_rollback = vec![Default::default(); target_len];
                        for i in 0..target_len {
                            cmd_rollback[target_len-1-i] = self.buf.pop_back().unwrap()
                        }
                        return callback(cmd_rollback)
                    }
            }
        }
        None
    }

    pub fn register_callback(&mut self, cmd_sequence: Vec<&str>, f: CmdCallback<'a>){
       let cmd_sequence = cmd_sequence.iter().map(|s|{s.to_vec_string()}).collect::<Vec<_>>();
       self.callbacks.push((VecDeque::from(cmd_sequence), f)); 
    }
}




// owned global state dispatched to each spawned worker thread
// ops on this struct is thread-safe but expensive
#[derive(Clone)]
pub struct SharedGlobalState{
    pub slave_hub: Arc<Mutex<HashSet<SocketAddr>>>,
    pub comm_channels: mpsc::Sender<Box<[u8]>>
}

pub fn serve_one_connection(
    mut stream: TcpStream,
    mut client_state: RedisStorage,
    server_state: LaunchConfig,
    shared_state: SharedGlobalState
){
    let mut buf = vec!(0u8; 2048).into_boxed_slice(); 
    let mut cmd_cache = CommandCache::new(6);

    
    cmd_cache.register_callback(
        vec!["ping", "replconf listening-port", "replconf capa", "psync"],
        Box::new(
            |cmd_rollback|{
               println!("callback here!");
               for i in &cmd_rollback{
                   println!("{:?}", i);
               };
               let slave_port = &cmd_rollback[1].last().unwrap();
               let slave_socket_addrs = format!("localhost:{}", slave_port)
                                            .to_socket_addrs().unwrap()
                                            .next().unwrap();
               let mut locked_slave_hub = shared_state.slave_hub.lock().unwrap();
               locked_slave_hub.insert(slave_socket_addrs);
               Some(nod_replica(&server_state))
            }
        )
    );
    
    loop{
        let num_readin = stream.read(&mut buf).unwrap();
        let str_wo_escape = std::str::from_utf8(&buf[..num_readin]).unwrap()
                                .chars().collect::<Vec<_>>(); 
        
        println!("#bytes read: {}", num_readin);
        println!("{:?}", str_wo_escape);
        while num_readin == 0 {};
        let client_msg = parser::decrypt::parse_resp(&buf[..num_readin]);


        match client_msg {
            Ok(resp_object) => {
                let (mut cmd, mut params) = ("none".as_bytes(), vec!());
                let client_raw_bytes: Vec<&[u8]>;
                match resp_object {
                   RESPObject::Simple(simple_object) => {
                      match simple_object {
                        SimpleRESPObject::Str(client_msg) => {
                            (cmd, params) = (client_msg.as_bytes(), vec!());
                        },
                        _ => {
                            (cmd, params) = ("none".as_bytes(), vec!());
                        }
                      } 
                      client_raw_bytes = vec!(cmd);
                      
                   },
                   
                   RESPObject::Aggregate(aggr_object) => {
                       match aggr_object {
                         AggrRESPObject::BulkStr(client_msg) => {
                           (cmd, params) = (client_msg, vec!());
                           client_raw_bytes = vec!(cmd);
                         },
                         AggrRESPObject::Array(objects_arr) => {
                             let unpacked_arr = unpack_resp_array(objects_arr);
                             (cmd, params) = (unpacked_arr[0], (&unpacked_arr[1..]).to_vec());
                             client_raw_bytes = unpacked_arr;
                         }
                       }
                   }
                }
                // considering push both cmd & params
                let client_raw_bytes = client_raw_bytes.iter()
                                            .map(|s|{std::str::from_utf8(s).unwrap()})
                                            .collect::<Vec<_>>();
                let callback_ret = cmd_cache.push(client_raw_bytes);
                if let Some(callback_msg) = callback_ret{
                    stream.write_all(&callback_msg);
                }else {
                    let server_response = command_router(cmd, params, &mut client_state, &server_state);

                    // propagate modification on master to possible slaves by send it to
                    // dispatching thread
                    if let ServerType::Master = server_state.server_type{
                        match cmd {
                            b"set" => {
                                let client_msg = buf[..num_readin].to_vec().into_boxed_slice();
                                shared_state.comm_channels.send(client_msg);
                            },
                            _ => {}
                        }
                    }
                    stream.write_all(&server_response);
                }
            }
            Err(_) => {}
        }
    }
}

fn command_router(
    cmd: &[u8], params: Vec<&[u8]>,
    client_state: &mut RedisStorage,
    server_state: &LaunchConfig
) -> Box<[u8]>{
   let lowercase_cmd = std::str::from_utf8(cmd).unwrap().to_lowercase();
   match lowercase_cmd.as_str() {
       "ping" => command::ping(),
       "echo" => command::echo(params[0]),
       "set" => {
            let mut data = client_state.data.lock().unwrap();
            command::set(params[0], params[1], &mut data)
        },
       "get" => {
            let data = client_state.data.lock().unwrap();
            command::get(params[0], &data)
        },
       "info" => command::info(params[0], &server_state),
       "replconf" => command::replconf(params),
       _ => unreachable!() 
   } 
}

fn unpack_resp_array<'a>(object_arr: Vec<AtomicItem<'a>>) -> Vec<&'a [u8]>{
    object_arr.into_iter().take_while(
        |item| {
            match item{
                AtomicItem::SimpleItem(SimpleRESPObject::Str(_)) => true,
                AtomicItem::AggrItem(AggrRESPObject::BulkStr(_)) => true,
                _ => false
            }
        }
    ).map(move |item| {item.as_bytes().unwrap()}).collect()
}


pub fn parse_cmd_args() -> LaunchConfig{
    let mut config = LaunchConfig::new();
    let args = std::env::args().collect::<Vec<String>>();
    let mut args_iter = args.iter().skip(1);

    while let Some(arg) = args_iter.next(){
        if &arg[..2] == "--" {
            match &arg[2..]{
                "port" => {
                    let port_id = args_iter.next().unwrap();
                    config.binding_addr = format!("localhost:{port_id}");
                },
                "replicaof" => {
                    let master_ip = args_iter.next().unwrap();
                    let master_port = args_iter.next().unwrap();
                    config.replicaof = Some((format!("{master_ip}"), format!("{master_port}")));
                    config.server_type = ServerType::Slave;
                },
                _ => panic!("unacceptable cmd line key arg")
            }
        }else{
            panic!("incorrect keyword argument spec");
        }
    }

    // generate random char vector with size 40
    if let ServerType::Master = config.server_type{
       config.replica_id = Some(
           rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric)
                .take(40).collect::<Vec<u8>>()
       )
    }
    config
}





pub mod master{
   use crate::parser::encrypt::as_array;
   use super::LaunchConfig;

   pub fn nod_replica(config: &LaunchConfig) -> Box<[u8]>{
       println!("{:?}", config.replica_id.as_ref().unwrap());
       let replica_id = std::str::from_utf8(config.replica_id.as_ref().unwrap()).unwrap();
       let msg = vec!["+FULLRESYNC", replica_id, "0"];
       as_array(msg)
   }

   pub mod service{
       
       use std::net::TcpStream;
       use std::sync::{Arc, Mutex, mpsc};
       use std::collections::HashSet;
       use std::io::Write;
       
       type SlaveHub = HashSet<std::net::SocketAddr>;
       // bring up another thread for propagate modifications made on master to its slaves
       pub fn push_down_ops(slave_hub: Arc<Mutex<SlaveHub>>, rc: mpsc::Receiver<Box<[u8]>>){
           loop {
               let msg = rc.recv().unwrap();
               for slave_addr in &*slave_hub.lock().unwrap(){
                   let mut stream = TcpStream::connect(slave_addr).expect("slave server is not up");
                   stream.write_all(&msg);
               }
           }
       }
   }
}



pub mod slave{
    use crate::parser::encrypt::{as_bulk_str, as_array};
    use crate::parser::decrypt::parse_resp;

    use std::net::TcpStream;
    use std::io::{Write, Read};
    use super::LaunchConfig;
    
    pub fn initiate_replica(config: &mut LaunchConfig) -> Result<(), std::io::Error>{
       println!("try to connect to master");
       let (master_ip, master_port) = &config.replicaof.as_ref().unwrap();
       let (_, slave_port) = config.binding_addr.rsplit_once(':').unwrap();
       println!("{}:{}", master_ip, master_port);
       let mut stream = TcpStream::connect(format!("{}:{}", master_ip, master_port))?;
       
       let mut buf = vec![0u8; 512].into_boxed_slice();
       // three way handshake

       // first: sending ping -> expecting pong
       stream.write(&as_bulk_str(Some(b"PING")))?;
       stream.read(&mut buf)?;

       // second: sending $replconf listening-port <port_id>
       let mut msg = vec!["REPLCONF", "listening-port", slave_port];
       stream.write(&as_array(msg))?;
       stream.read(&mut buf)?;

       // sending $replconfg capa eof capa psync2
       msg = vec!["REPLCONF", "capa", "eof", "capa", "psync2"];
       stream.write(&as_array(msg))?;
       stream.read(&mut buf)?;

       // third stage
       msg = vec!["PSYNC", "-1", "?"];
       stream.write(&as_array(msg))?;
       let num_readin = stream.read(&mut buf)?;
       let decoded_replica_config = parse_resp(&buf[..num_readin]).unwrap();
       config.replica_id = Some(decoded_replica_config.to_vec().unwrap()[1].to_vec()); 
       println!("id: {:?}", std::str::from_utf8(config.replica_id.as_ref().unwrap()).unwrap());
       Ok(())
    }
}

