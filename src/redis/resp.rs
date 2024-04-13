use std::{
    collections::{BTreeSet, HashMap},
    io::{BufRead, BufReader, Read},
};

pub enum RESPDataTypes {
    SimpleString(String),
    SimpleError(String),
    Integer(i64),
    BulkString(String),
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

impl<T> From<T> for RESPDataTypes
where
    T: Read,
{
    fn from(value: T) -> Self {
        let mut resp_buffer_reader = BufReader::new(value);
        let mut data_type_marker = [u8::default()];

        resp_buffer_reader
            .read_exact(&mut data_type_marker)
            .unwrap();

        let mut resp_parser = RESPParser::new(resp_buffer_reader);

        match &data_type_marker[..] {
            b"+" => resp_parser.parse_simple_string(),
            b"$" => resp_parser.parse_bulk_string(),
            b"*" => resp_parser.parse_array(),
            _ => RESPDataTypes::Null,
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

    fn parse_simple_string(&mut self) -> RESPDataTypes {
        let mut simple_string = String::new();

        self.resp_buffer_reader
            .read_line(&mut simple_string)
            .unwrap();

        RESPDataTypes::SimpleString(
            simple_string
                .strip_prefix("+")
                .unwrap()
                .strip_suffix("\r\n")
                .unwrap()
                .to_owned(),
        )
    }

    fn parse_bulk_string(&mut self) -> RESPDataTypes {
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

        RESPDataTypes::BulkString(String::from_utf8(bulk_string).unwrap())
    }

    fn parse_array(&mut self) -> RESPDataTypes {
        let mut length = String::new();

        self.resp_buffer_reader.read_line(&mut length).unwrap();

        let length = length
            .strip_prefix("*")
            .unwrap()
            .strip_suffix("\r\n")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let mut elements = Vec::<RESPDataTypes>::new();

        for _ in 0..length {
            let mut element_buffer = String::new();

            self.resp_buffer_reader
                .read_line(&mut element_buffer)
                .unwrap();
            self.resp_buffer_reader
                .read_line(&mut element_buffer)
                .unwrap();
            elements.push(RESPDataTypes::from(element_buffer.as_bytes()));
        }

        RESPDataTypes::Array(elements)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::{RESPDataTypes::*, RESPParser};

    #[test]
    fn simple_string() {
        assert!(match create_parser("+OK\r\n").parse_simple_string() {
            SimpleString(string) if &string == "OK" => true,
            _ => false,
        });
    }

    #[test]
    fn bulk_string() {
        assert!(match create_parser("$5\r\nhello\r\n").parse_bulk_string() {
            BulkString(string) if &string == "hello" => true,
            _ => false,
        });
    }

    #[test]
    fn array() {
        let parsed_array = create_parser("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n").parse_array();

        assert!(if let Array(_) = parsed_array {
            true
        } else {
            false
        });

        if let Array(mut elements) = parsed_array {
            assert_eq!(elements.len(), 2);

            assert!(if let Some(BulkString(ref string)) = elements.pop() {
                if string == "world" {
                    true
                } else {
                    false
                }
            } else {
                false
            });

            assert!(if let Some(BulkString(ref string)) = elements.pop() {
                if string == "hello" {
                    true
                } else {
                    false
                }
            } else {
                false
            });
        }
    }

    fn create_parser(data: &str) -> RESPParser<&[u8]> {
        RESPParser::new(BufReader::new(data.as_bytes()))
    }
}