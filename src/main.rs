use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};


use std::thread;

fn serve_one_connection(mut stream: TcpStream){
    let mut buf = [0u8; 512];
    let ping_response = b"+PONG\r\n";
    loop{
        while stream.read(&mut buf).unwrap() == 0 {};
        stream.write_all(ping_response);
    }
}

fn main(){

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    
    for stream in listener.incoming() {
        match stream{
            Ok(mut stream) => {
                let thread_handler = thread::spawn(move || {serve_one_connection(stream)});
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
