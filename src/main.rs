use std::io::{Read, Write};
use std::net::TcpListener;

fn main(){

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buf = [0u8; 512];
                let ping_response = "+PONG\r\n";
                
                let num_readin = stream.read(&mut buf).unwrap();
                if num_readin < 15{
                    stream.write_all(ping_response.clone().as_bytes())
                            .unwrap();
                }else{
                    stream.write_all(ping_response.repeat(2).as_bytes())
                            .unwrap();
                }
                // let _ = stream.write_all(ping_response);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
