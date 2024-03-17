use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;


use crate::{command, parser};
use crate::parser::{
    RESPObject, SimpleRESPObject, AggrRESPObject, AtomicItem
};


#[derive(Clone, Debug)]
pub struct LaunchConfig{
    pub binding_addr: String,
    pub replicaof: Option<(String, String)> 
} 

impl LaunchConfig {
    fn new() -> Self{
        Self{
          binding_addr: String::from("localhost:6379"),
          replicaof: None
        }
    }
}


pub struct Client{
    pub storage: HashMap<Box<[u8]>, Box<[u8]>>
}




pub fn serve_one_connection(
    mut stream: TcpStream,
    mut client_state: Client,
    server_state: LaunchConfig
){
    let mut buf = vec!(0u8; 2048).into_boxed_slice(); 
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
                   },
                   
                   RESPObject::Aggregate(aggr_object) => {
                       match aggr_object {
                         AggrRESPObject::BulkStr(client_msg) => {
                           (cmd, params) = (client_msg, vec!());
                         },
                         AggrRESPObject::Array(objects_arr) => {
                             let unpacked_arr = unpack_resp_array(objects_arr);
                             (cmd, params) = (unpacked_arr[0], (&unpacked_arr[1..]).to_vec());
                         }
                       }
                   }
                }
                let server_response = command_router(cmd, params, &mut client_state, &server_state);
                stream.write_all(&server_response);
            }
            Err(_) => {}
        }
    }
}

fn command_router(
    cmd: &[u8], params: Vec<&[u8]>,
    client_state: &mut Client,
    server_state: &LaunchConfig
) -> Box<[u8]>{
   let lowercase_cmd = std::str::from_utf8(cmd).unwrap().to_lowercase();
   match lowercase_cmd.as_str() {
       "ping" => command::ping(),
       "echo" => command::echo(params[0]),
       "set" => command::set(params[0], params[1], &mut client_state.storage),
       "get" => command::get(params[0], &client_state.storage),
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
                },
                _ => panic!("unacceptable cmd line key arg")
            }
        }else{
            panic!("incorrect keyword argument spec");
        }
    }
    config
}



pub mod slave{
    use crate::parser::encrypt::{as_bulk_str, as_array};

    use std::net::TcpStream;
    use std::io::{Write, Read};
    use super::LaunchConfig;
    
    pub fn initiate_replica(config: &LaunchConfig) -> Result<(), std::io::Error>{
       println!("try to connect to master");
       let (master_ip, master_port) = &config.replicaof.as_ref().unwrap();
       println!("{}:{}", master_ip, master_port);
       let mut stream = TcpStream::connect(format!("{}:{}", master_ip, master_port))?;
       
       let mut buf = vec![0u8; 512].into_boxed_slice();
       // three way handshake

       // first: sending ping -> expecting pong
       stream.write(&as_bulk_str(Some(b"PING")))?;
       stream.read(&mut buf)?;

       // second: sending $replconf listening-port <port_id>
       let mut msg = vec!["REPLCONF", "listening-port", master_port];
       stream.write(&as_array(msg))?;
       stream.read(&mut buf)?;

       // third stage: sending $replconfg capa eof capa psync2
       msg = vec!["REPLCONF", "capa", "eof", "capa", "psync2"];
       stream.write(&as_array(msg))?;
       stream.read(&mut buf)?;
       Ok(())
    }
}

