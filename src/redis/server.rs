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
            match RESPDataTypes::try_from(&stream) {
                Ok(request) => {
                    let mut command = match RedisCommand::try_from(request) {
                        Ok(command) => command,
                        Err(_) => panic!("something went wrong"),
                    };
                    command.respond(&mut stream);
                }
                Err(error) => {
                    if let RESPDataTypes::Null = error {
                        return;
                    } else if let RESPDataTypes::BulkError(_) = error {
                        error.write(&mut stream);
                    }
                }
            }
        }
    }
}
