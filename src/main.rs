use std::collections::{HashMap, HashSet};
use std::net::{TcpListener, SocketAddr};
use std::thread;

use std::sync::{mpsc, Arc, Mutex};


use redislib::server::*;

fn main() {
    let mut launch_config = parse_cmd_args();

    let listener = TcpListener::bind(launch_config.binding_addr.to_owned()).unwrap();

    if launch_config.replicaof.is_some(){
        slave::initiate_replica(&mut launch_config).unwrap();
    }

    let shared_slave_hub = Arc::new(Mutex::new(HashSet::new()));
    let (tx, rx) = mpsc::channel();
    let shared_global_state = SharedGlobalState{
        slave_hub: shared_slave_hub, 
        comm_channels: tx
    };


    // launch a dispatch thread to handle modification if the server is launched in master mode
    if let ServerType::Master = launch_config.server_type {
        let dup_shared_slave_hub = Arc::clone(&shared_global_state.slave_hub);
        thread::spawn(
            move || master::service::push_down_ops(dup_shared_slave_hub, rx)
        );
    }
        
    
    for stream in listener.incoming() {
        match stream{
            Ok(stream) => {
                let init_client_state = Client {storage: HashMap::new()};
                let server_state = launch_config.to_owned();
                let shared_global_state = shared_global_state.clone();
                let _ = thread::spawn(
                    move || {serve_one_connection(stream, init_client_state, server_state, shared_global_state)}
                );
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
