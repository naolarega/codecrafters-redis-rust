use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
};

use crate::executor::ThreadPoolExecutor;

pub struct Redis {
    host: &'static str,
    port: u16,
    executor: ThreadPoolExecutor,
}

impl Default for Redis {
    fn default() -> Self {
        Self {
            host: "127.0.0.1",
            port: 6379,
            executor: ThreadPoolExecutor::new(),
        }
    }
}

impl Redis {
    pub fn set_host(&mut self, host: &'static str) {
        self.host = host;
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn listen(&mut self) -> io::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port))?;

        for stream in listener.incoming().flatten() {
            self.executor.submit(|| Self::handle(stream));
        }

        Ok(())
    }

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
}
