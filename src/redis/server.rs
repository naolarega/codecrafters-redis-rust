use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
};

use crate::executor::ThreadPoolExecutor;
use crate::redis::resp::RESPDataTypes;

use super::commands::RedisCommand;

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
            let request = RESPDataTypes::from(&mut stream);
            let mut command = RedisCommand::from(request);

            command.respond(&mut stream);
        }
    }
}
