use std::collections::HashMap;
use std::net::TcpListener;
use std::thread;


use redislib::server::*;

fn main() {
    let launch_config = parse_cmd_args();

    let listener = TcpListener::bind(launch_config.binding_addr.to_owned()).unwrap();

    if launch_config.replicaof.is_some(){
        slave::initiate_replica(&launch_config).unwrap();
    }
    
    for stream in listener.incoming() {
        match stream{
            Ok(stream) => {
                let init_client_state = Client {storage: HashMap::new()};
                let server_state = launch_config.to_owned();
                let _ = thread::spawn(
                    move || {serve_one_connection(stream, init_client_state, server_state)}
                );
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
