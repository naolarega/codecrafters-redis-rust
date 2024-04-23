use std::{
    collections::{BTreeSet, HashMap},
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
};

#[derive(Debug)]
pub enum RESPDataTypes {
    SimpleString(String),
    SimpleError(String),
    Integer(i64),
    BulkString(Option<String>),
    Array(Vec<RESPDataTypes>),
    Null,
    Boolean(bool),
    Double(f64),
    BigNumber(i128),
    BulkError(String),
    VerbatimString { encoding: String, string: String },
    Map(HashMap<RESPDataTypes, RESPDataTypes>),
    Set(BTreeSet<RESPDataTypes>),
    Push(Vec<RESPDataTypes>),
}

impl TryFrom<&TcpStream> for RESPDataTypes {
    type Error = RESPDataTypes;

    fn try_from(value: &TcpStream) -> Result<Self, Self::Error> {
        Self::deserialize(value)
    }
}

impl TryFrom<&[u8]> for RESPDataTypes {
    type Error = RESPDataTypes;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::deserialize(value)
    }
}

impl RESPDataTypes {
    fn deserialize<T>(value: T) -> Result<RESPDataTypes, RESPDataTypes>
    where
        T: Read,
    {
        let mut resp_buffer_reader = BufReader::new(value);
        let mut data_type_marker = [u8::default()];

        if resp_buffer_reader.read(&mut data_type_marker).unwrap() == 0 {
            return Err(RESPDataTypes::Null);
        }

        let mut resp_parser = RESPParser::new(resp_buffer_reader);

        match &data_type_marker[..] {
            b"+" => resp_parser.parse_simple_string(),
            b"$" => resp_parser.parse_bulk_string(),
            b"*" => resp_parser.parse_array(),
            _ => Ok(RESPDataTypes::Null),
        }
    }

    pub fn serialize(&self) -> String {
        use RESPDataTypes::*;

        match self {
            SimpleString(value) => format!("+{value}\r\n"),
            SimpleError(value) => format!("-{value}\r\n"),
            BulkString(value) => {
                if let Some(value) = value {
                    format!("${}\r\n{value}\r\n", value.len())
                } else {
                    "-1\r\n".to_string()
                }
            }
            Null => "_\r\n".to_string(),
            BulkError(value) => format!("!{}\r\n{value}\r\n", value.len()),
            _ => "-Error\r\n".to_string(),
        }
    }
}

struct RESPParser<T>
where
    T: Read,
{
    resp_buffer_reader: BufReader<T>,
}

impl<T> RESPParser<T>
where
    T: Read,
{
    fn new(resp_buffer_reader: BufReader<T>) -> Self {
        Self { resp_buffer_reader }
    }

    fn parse_simple_string(&mut self) -> Result<RESPDataTypes, RESPDataTypes> {
        let mut simple_string = String::new();

        self.resp_buffer_reader
            .read_line(&mut simple_string)
            .unwrap();

        Ok(RESPDataTypes::SimpleString(
            simple_string
                .strip_prefix("+")
                .unwrap()
                .strip_suffix("\r\n")
                .unwrap()
                .to_owned(),
        ))
    }

    fn parse_bulk_string(&mut self) -> Result<RESPDataTypes, RESPDataTypes> {
        let mut length = String::new();

        self.resp_buffer_reader.read_line(&mut length).unwrap();

        let length = if let Some(striped_string) = length.strip_prefix("$") {
            striped_string
        } else {
            &length
        };
        let length = if let Some(striped_string) = length.strip_suffix("\r\n") {
            striped_string
        } else {
            &length
        };
        let length = length.parse::<usize>().unwrap();
        let mut bulk_string = vec![u8::default(); length];

        self.resp_buffer_reader
            .read_exact(&mut bulk_string)
            .unwrap();

        Ok(RESPDataTypes::BulkString(Some(
            String::from_utf8(bulk_string).unwrap(),
        )))
    }

    fn parse_array(&mut self) -> Result<RESPDataTypes, RESPDataTypes> {
        let mut length = String::new();

        self.resp_buffer_reader.read_line(&mut length).unwrap();

        let length = if let Some(striped_string) = length.strip_prefix("*") {
            striped_string
        } else {
            &length
        };
        let length = if let Some(striped_string) = length.strip_suffix("\r\n") {
            striped_string
        } else {
            &length
        };
        let length = length.parse::<usize>().unwrap();
        let mut elements = Vec::<RESPDataTypes>::new();

        for _ in 0..length {
            let mut element_buffer = String::new();

            self.resp_buffer_reader
                .read_line(&mut element_buffer)
                .unwrap();
            self.resp_buffer_reader
                .read_line(&mut element_buffer)
                .unwrap();
            elements.push(RESPDataTypes::try_from(element_buffer.as_bytes())?);
        }

        Ok(RESPDataTypes::Array(elements))
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::{RESPDataTypes::*, RESPParser};

    #[test]
    fn simple_string() {
        assert!(match create_parser("+OK\r\n").parse_simple_string() {
            Ok(SimpleString(string)) if &string == "OK" => true,
            _ => false,
        });
    }

    #[test]
    fn bulk_string() {
        assert!(match create_parser("$5\r\nhello\r\n").parse_bulk_string() {
            Ok(BulkString(Some(string))) if &string == "hello" => true,
            _ => false,
        });
    }

    #[test]
    fn array() {
        let parsed_array = create_parser("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n").parse_array();

        assert!(if let Ok(Array(_)) = parsed_array {
            true
        } else {
            false
        });

        let mut elements = if let Ok(Array(mut elements)) = parsed_array {
            elements
        } else {
            panic!("");
        };

        assert_eq!(elements.len(), 2);

        assert!(if let Some(BulkString(ref string)) = elements.pop() {
            if let Some(string) = string {
                if string == "world" {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        });

        assert!(if let Some(BulkString(ref string)) = elements.pop() {
            if let Some(string) = string {
                if string == "hello" {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        });
    }

    fn create_parser(data: &str) -> RESPParser<&[u8]> {
        RESPParser::new(BufReader::new(data.as_bytes()))
    }
}
