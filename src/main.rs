use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread
};

fn handle(mut stream: TcpStream) {
    loop {
        let mut buf = [u8::default(); 512];
        let n = stream.read(&mut buf).unwrap();

        if n == 0 {
            break;
        }

        stream.write(b"+PONG\r\n").unwrap();
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle(stream));
    }
}
