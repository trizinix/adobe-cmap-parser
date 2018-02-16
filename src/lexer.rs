use pom::char_class::{alpha, hex_digit, oct_digit, multispace};
use pom::{Parser, DataInput};
use pom::parser::*;
use std::collections::HashMap;

use std::str::FromStr;

use error::{Result, CMapError};

#[derive(Debug)]
pub enum Value {
    LiteralString(Vec<u8>),
    Name(Vec<u8>),
    Number(String),
    Integer(i64),
    Array(Vec<Value>),
    Operator(String),
    Boolean(bool),
    Dictionary(HashMap<String, Value>),
}

impl Value {
    pub fn as_literal_string(&self) -> Result<&[u8]> {
        match *self {
            Value::LiteralString(ref s) => Ok(s),
            _ => self.expect_type(&"LiteralString")
        }
    }

    pub fn as_name(&self) -> Result<&[u8]> {
        match *self {
            Value::Name(ref s) => Ok(s),
            _ => self.expect_type(&"Name")         }
    }

    pub fn as_integer(&self) -> Result<i64> {
        match *self {
            Value::Integer(ref i) => Ok(*i),
            _ => self.expect_type(&"Integer")
        }
    }

    pub fn expect_type<T>(&self, expected: &'static str) -> Result<T> {
        Err(CMapError::CMapType {expected, found: self.get_type()})
    }

    fn get_type(&self) -> &'static str {
        match *self {
            Value::LiteralString(_) => "LiteralString",
            Value::Name(_) => "Name",
            Value::Number(_) => "Number",
            Value::Integer(_) => "Integer",
            Value::Array(_) => "Array",
            Value::Operator(_) => "Operator",
            Value::Boolean(_) => "Boolean",
            Value::Dictionary(_) => "Dictionary"
        }
    }
}

fn hex_char() -> Parser<u8, u8> {
    let number = is_a(hex_digit).repeat(2);
    number.collect().convert(|v|u8::from_str_radix(&String::from_utf8(v).unwrap(), 16))
}

fn comment() -> Parser<u8, ()> {
    sym(b'%') * none_of(b"\r\n").repeat(0..) * eol().discard()
}

fn content_space() -> Parser<u8, ()> {
    is_a(multispace).repeat(0..).discard()
}

fn operator() -> Parser<u8, String> {
    (is_a(alpha) | one_of(b"*'\"")).repeat(1..).convert(|v|String::from_utf8(v))
}

fn oct_char() -> Parser<u8, u8> {
    let number = is_a(oct_digit).repeat(1..4);
    number.collect().convert(|v|u8::from_str_radix(&String::from_utf8(v).unwrap(), 8))
}

fn escape_sequence() -> Parser<u8, Vec<u8>> {
    sym(b'\\') *
        ( sym(b'\\').map(|_| vec![b'\\'])
            | sym(b'(').map(|_| vec![b'('])
            | sym(b')').map(|_| vec![b')'])
            | sym(b'n').map(|_| vec![b'\n'])
            | sym(b'r').map(|_| vec![b'\r'])
            | sym(b't').map(|_| vec![b'\t'])
            | sym(b'b').map(|_| vec![b'\x08'])
            | sym(b'f').map(|_| vec![b'\x0C'])
            | oct_char().map(|c| vec![c])
            | eol()     .map(|_| vec![])
            | empty()   .map(|_| vec![])
        )
}

fn nested_literal_string() -> Parser<u8, Vec<u8>> {
    sym(b'(') *
        ( none_of(b"\\()").repeat(1..)
            | escape_sequence()
            | call(nested_literal_string)
        ).repeat(0..).map(|segments| {
            let mut bytes = segments.into_iter().fold(
                vec![b'('],
                |mut bytes, mut segment| {
                    bytes.append(&mut segment);
                    bytes
                });
            bytes.push(b')');
            bytes
        })
        - sym(b')')
}

fn literal_string() -> Parser<u8, Vec<u8>> {
    sym(b'(') *
        ( none_of(b"\\()").repeat(1..)
            | escape_sequence()
            | nested_literal_string()
        ).repeat(0..).map(|segments|segments.concat())
        - sym(b')')
}

fn name() -> Parser<u8, Vec<u8>> {
    sym(b'/') * (none_of(b" \t\n\r\x0C()<>[]{}/%#") | sym(b'#') * hex_char()).repeat(0..)
}

fn integer() -> Parser<u8, i64> {
    let number = one_of(b"+-").opt() + one_of(b"0123456789").repeat(1..);
    number.collect().convert(|v|String::from_utf8(v)).convert(|s|i64::from_str(&s))
}

fn number() -> Parser<u8, String> {
    let number = one_of(b"+-").opt() +
        ( (one_of(b"0123456789") - one_of(b"0123456789").repeat(0..).discard())
            | (one_of(b"0123456789").repeat(1..) * sym(b'.') - one_of(b"0123456789").repeat(0..))
            | sym(b'.') - one_of(b"0123456789").repeat(1..)
        );
    number.collect().convert(|v|String::from_utf8(v))
}

fn space() -> Parser<u8, ()> {
    ( one_of(b" \t\n\r\0\x0C").repeat(1..).discard()
    ).repeat(0..).discard()
}

fn hexadecimal_string() -> Parser<u8, Vec<u8>> {
    sym(b'<') * hex_char().repeat(0..) - sym(b'>')
}

fn eol() -> Parser<u8, u8> {
    sym(b'\r') * sym(b'\n') | sym(b'\n') | sym(b'\r')
}

// Dictionaries are not mentioned in the CMap spec but are produced by software like Cairo and Skia and supported other by readers
fn dictionary() -> Parser<u8, HashMap<String, Value>> {
    let entry = name() - space() + call(value);
    let entries = seq(b"<<") * space() * entry.repeat(0..) - seq(b">>");
    entries.map(|entries| entries.into_iter().fold(
        HashMap::new(),
        |mut dict: HashMap<String, Value>, (key, value)| { dict.insert(String::from_utf8(key).unwrap(), value); dict }
    ))
}

fn array() -> Parser<u8, Vec<Value>> {
    sym(b'[') * space() * call(value).repeat(0..) - sym(b']')
}


fn value() -> Parser<u8, Value> {
    ( seq(b"true").map(|_| Value::Boolean(true))
    | seq(b"false").map(|_| Value::Boolean(false))
    | integer().map(|v| Value::Integer(v))
    | number().map(|v| Value::Number(v))
    | name().map(|v| Value::Name(v))
    | operator().map(|v| Value::Operator(v))
    | literal_string().map(|v| Value::LiteralString(v))
    | dictionary().map(|v| Value::Dictionary(v))
    | hexadecimal_string().map(|v| Value::LiteralString(v))
    | array().map(|v| Value::Array(v))
    ) - content_space()
}


fn lexems() -> Parser<u8,Vec<Value>>
{
    ( comment().repeat(0..) * content_space() * value()).repeat(1..)
}


pub fn lexer(input: &[u8]) -> Result<Vec<Value>> {
    lexems().parse(&mut DataInput::new(input)).map_err(CMapError::Lexer)
}
