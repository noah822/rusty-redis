use std::io::Write;
use std::net::TcpListener;

fn main(){

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let ping_response = b"+PONG\r\n";
                let _ = stream.write_all(ping_response);
                let _ = stream.write_all(ping_response);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
