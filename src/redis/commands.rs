use std::io::Write;

use crate::redis::resp::RESPDataTypes;

pub enum RedisCommand {
    Echo(Vec<String>),
}

impl From<RESPDataTypes> for RedisCommand {
    fn from(request: RESPDataTypes) -> Self {
        let command_and_args = if let RESPDataTypes::Array(elements) = request {
            elements
        } else {
            panic!("resp datatype other than array of bulk string not supported");
        };

        if command_and_args.is_empty() {
            panic!("no command provided");
        }

        let command = if let Some(RESPDataTypes::BulkString(command)) = command_and_args.first() {
            command
        } else {
            panic!("command must be type of bulk string");
        };

        let mut args = Vec::new();

        if command_and_args.len() > 1 {
            args.extend(command_and_args[1..].iter().map(|arg| {
                if let RESPDataTypes::BulkString(arg) = arg {
                    arg.to_owned()
                } else {
                    panic!("argument must be type of bulk string")
                }
            }));
        }

        match command.to_lowercase().as_str() {
            "echo" => RedisCommand::Echo(args),
            _ => panic!("unknown command"),
        }
    }
}

impl RedisCommand {
    pub fn respond<T>(&mut self, stream: &mut T)
    where
        T: Write,
    {
    }
}

#[cfg(test)]
mod tests {
    use super::{RESPDataTypes, RedisCommand};

    macro_rules! raw_request {
        ($command:literal) => {
            RESPDataTypes::Array(
                vec![
                    RESPDataTypes::BulkString($command.to_string())
                ]
            )
        };
        ($command:literal, [$($arg:literal),*]) => {
            RESPDataTypes::Array(
                vec![
                    RESPDataTypes::BulkString($command.to_string()),
                    $(
                        RESPDataTypes::BulkString($arg.to_string()),
                    )*
                ]
            )
        };
    }

    #[test]
    fn echo() {
        let redis_command = RedisCommand::from(raw_request!("echo", ["Hello World!"]));

        assert!(match redis_command {
            RedisCommand::Echo(_) => true,
            _ => false,
        });

        if let RedisCommand::Echo(mut args) = redis_command {
            assert_eq!(args.len(), 1);

            assert_eq!(args.pop(), Some(String::from("Hello World!")));
        }
    }
}
