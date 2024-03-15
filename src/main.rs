use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};


use std::thread;


use redislib::{command, parser};
use redislib::parser::{
    RESPObject, SimpleRESPObject, AggrRESPObject, AtomicItem
};


struct Client{
    storage: HashMap<Box<[u8]>, Box<[u8]>>
}


fn serve_one_connection(mut stream: TcpStream, mut client_state: Client){
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
                let server_response = command_router(cmd, params, &mut client_state);
                stream.write_all(&server_response);
            }
            Err(_) => {}
        }
    }
}

fn command_router(cmd: &[u8], params: Vec<&[u8]>, client_state: &mut Client) -> Box<[u8]>{
   let lowercase_cmd = std::str::from_utf8(cmd).unwrap().to_lowercase();
   match lowercase_cmd.as_str() {
       "ping" => command::ping(),
       "echo" => command::echo(params[0]),
       "set" => command::set(params[0], params[1], &mut client_state.storage),
       "get" => command::get(params[0], &client_state.storage),
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

fn main(){

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    
    for stream in listener.incoming() {
        match stream{
            Ok(stream) => {
                let init_client_state = Client {storage: HashMap::new()};
                let _ = thread::spawn(move || {serve_one_connection(stream, init_client_state)});
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
        // match stream {
        //     Ok(mut stream) => {
        //         let mut buf = [0u8; 512];
        //         let ping_response = b"+PONG\r\n";
        //         
        //         loop {
        //             while stream.read(&mut buf).unwrap() == 0 {}
        //             stream.write_all(ping_response).unwrap();
        //         }
        //         
        //     }
        //     Err(e) => {
        //         println!("error: {}", e);
        //     }
        // }
    }
}
