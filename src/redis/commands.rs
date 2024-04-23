use std::{cell::RefCell, collections::HashMap, io::Write};

use crate::redis::resp::RESPDataTypes;

macro_rules! redis_err {
    ($message:expr) => {
        Err(RESPDataTypes::BulkError($message.to_string()))
    };
}

thread_local! {
    pub static KV_STORE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

pub enum RedisCommand {
    PING { message: Option<String> },
    ECHO { message: String },
    SET { key: String, value: String },
    GET { key: String },
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
            if let Some(command) = command {
                command
            } else {
                return redis_err!("command must none null string");
            }
        } else {
            return redis_err!("command must be type of bulk string");
        };
        let mut args = Vec::new();

        if command_and_args.len() > 1 {
            if !command_and_args[1..].iter().all(|arg| match arg {
                RESPDataTypes::BulkString(arg) if arg.is_some() => true,
                _ => false,
            }) {
                return redis_err!("argument must be type of bulk string");
            }

            args.extend(command_and_args[1..].iter().map(|arg| {
                if let RESPDataTypes::BulkString(arg) = arg {
                    if let Some(arg) = arg {
                        arg.to_owned()
                    } else {
                        panic!("should never reach here")
                    }
                } else {
                    panic!("should never reach here either")
                }
            }));
        }

        match command.to_lowercase().as_str() {
            "ping" => {
                let mut message = None;

                if let Some(arg) = args.first() {
                    message = Some(arg.to_owned());
                }

                Ok(RedisCommand::PING { message })
            }
            "echo" => {
                if let Some(message) = args.first() {
                    Ok(RedisCommand::ECHO {
                        message: message.to_owned(),
                    })
                } else {
                    redis_err!("message not provided for echo command")
                }
            }
            "set" => {
                if args.len() < 2 {
                    redis_err!("both key and value should be provided")
                } else {
                    Ok(RedisCommand::SET {
                        key: args[0].to_owned(),
                        value: args[1].to_owned(),
                    })
                }
            }
            "get" => {
                if let Some(key) = args.first() {
                    Ok(RedisCommand::GET {
                        key: key.to_owned(),
                    })
                } else {
                    redis_err!("key not provided")
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

        let response = match self {
            PING { message } => {
                if let Some(message) = message {
                    RESPDataTypes::BulkString(Some(message.to_owned()))
                } else {
                    RESPDataTypes::SimpleString("PONG".to_string())
                }
            }
            ECHO { message } => RESPDataTypes::BulkString(Some(message.to_owned())),
            SET { key, value } => {
                let mut previous_value = None;

                KV_STORE.with_borrow_mut(|kv| {
                    if let Some(v) = kv.get(key) {
                        previous_value = Some(v.to_owned());
                    }

                    kv.insert(key.to_owned(), value.to_owned());
                });

                if let Some(previous_value) = previous_value {
                    RESPDataTypes::BulkString(Some(previous_value))
                } else {
                    RESPDataTypes::SimpleString("OK".to_string())
                }
            }
            GET { key } => {
                let mut value = None;

                KV_STORE.with_borrow(|kv| {
                    let found_value = kv.get(key);

                    if let Some(found_value) = found_value {
                        value = Some(found_value.to_owned())
                    }
                });

                if let Some(value) = value {
                    RESPDataTypes::BulkString(Some(value.to_owned()))
                } else {
                    RESPDataTypes::Null
                }
            }
        };

        stream.write(response.serialize().as_bytes()).unwrap();
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
                    RESPDataTypes::BulkString(Some($command.to_string())),
                    $(
                        RESPDataTypes::BulkString(Some($arg.to_string())),
                    )*
                ]
            )
        };
    }

    #[test]
    fn echo() {
        let redis_command = RedisCommand::try_from(raw_request!("echo", ["Hello World!"]));

        assert!(match redis_command {
            Ok(RedisCommand::ECHO { message: _ }) => true,
            _ => false,
        });

        if let Ok(RedisCommand::ECHO { message }) = redis_command {
            assert_eq!(message, String::from("Hello World!"));
        }
    }
}
