use std::io::{Read, Write};
use std::net::TcpListener;

fn main(){

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buf = [0u8; 512];
                let ping_response = b"+PONG\r\n";
                
                loop {
                    while stream.read(&mut buf).unwrap() == 0 {}
                    stream.write_all(ping_response).unwrap();
                }
                
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
