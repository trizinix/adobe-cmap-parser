use pom;
use std::io;
use std::result;
use std::str;
use std::string;
pub type Result<T> = result::Result<T, CMapError>;

#[derive(Fail, Debug)]
pub enum CMapError {
    #[fail(display = "Encountered unknown operator {}", _0)]
    UnknownOperator(String),

    #[fail(display = "Encountered the type {}, but expected {}", found, expected)]
    CMapType { expected: &'static str, found: &'static str },

    #[fail(display = "Encountered an array of size {}, but expected {}", found, expected)]
    InvalidArrayLength { expected: usize, found: usize },

    #[fail(display = "No unicode mapping found for codepoint {}", _0)]
    NoUnicodeMappingFound(u32),

    #[fail(display = "{}", _0)]
    Utf16(#[cause] string::FromUtf16Error),

    #[fail(display = "{}", _0)]
    Utf8(#[cause] str::Utf8Error),

    #[fail(display = "{}", _0)]
    Lexer(#[cause] pom::Error),

    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
}