use std::io::Write;

use crate::redis::resp::RESPDataTypes;

macro_rules! redis_err {
    ($message:expr) => {
        Err(RESPDataTypes::BulkError($message.to_string()))
    };
}

pub enum RedisCommand {
    Echo { message: String },
}

impl TryFrom<RESPDataTypes> for RedisCommand {
    type Error = RESPDataTypes;

    fn try_from(request: RESPDataTypes) -> Result<Self, Self::Error> {
        let command_and_args = if let RESPDataTypes::Array(elements) = request {
            elements
        } else {
            return redis_err!("resp datatype other than array of bulk string not supported");
        };

        if command_and_args.is_empty() {
            return redis_err!("no command provided");
        }

        let command = if let Some(RESPDataTypes::BulkString(command)) = command_and_args.first() {
            command
        } else {
            return redis_err!("command must be type of bulk string");
        };
        let mut args = Vec::new();

        if command_and_args.len() > 1 {
            if !command_and_args[1..].iter().all(|arg| match arg {
                RESPDataTypes::BulkString(_) => true,
                _ => false,
            }) {
                return redis_err!("argument must be type of bulk string");
            }

            args.extend(command_and_args[1..].iter().map(|arg| {
                if let RESPDataTypes::BulkString(arg) = arg {
                    arg.to_owned()
                } else {
                    panic!("should never reach here")
                }
            }));
        }

        match command.to_lowercase().as_str() {
            "echo" => {
                if let Some(message) = args.first() {
                    Ok(RedisCommand::Echo {
                        message: message.to_owned(),
                    })
                } else {
                    redis_err!("message not provided for echo command")
                }
            }
            _ => redis_err!("unknown command"),
        }
    }
}

impl RedisCommand {
    pub fn respond<T>(&mut self, stream: &mut T)
    where
        T: Write,
    {
        use RedisCommand::*;

        match self {
            Echo { message } => stream
                .write(format!("${}\r\n{message}\r\n", message.len()).as_bytes())
                .unwrap(),
            _ => stream.write(b"!5\r\nerror\r\n").unwrap(),
        };

        stream.flush().unwrap();
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
        let redis_command = RedisCommand::try_from(raw_request!("echo", ["Hello World!"]));

        assert!(match redis_command {
            Ok(RedisCommand::Echo { message: _ }) => true,
            _ => false,
        });

        if let Ok(RedisCommand::Echo { message }) = redis_command {
            assert_eq!(message, String::from("Hello World!"));
        }
    }
}
