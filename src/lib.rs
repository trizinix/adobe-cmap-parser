extern crate failure;
extern crate pom;

#[macro_use] extern crate failure_derive;

use error::Result;
use std::collections::HashMap;
use std::cmp::min;

mod error;
mod lexer;
mod parser;

pub use parser::parse_cmap;
pub use error::CMapError;

#[derive(Debug)]
pub enum WritingMode {
    Horizontally,
    Vertically
}

impl From<bool> for WritingMode {
    fn from(u: bool) -> WritingMode {
        if u { WritingMode::Vertically }
        else { WritingMode::Horizontally }
    }
}

impl Default for WritingMode {
    fn default() ->  WritingMode { WritingMode::Horizontally }
}

#[derive(Clone, Debug)]
pub struct CodespaceRange {
    from: u32,
    to: u32,
    len: usize
}

impl CodespaceRange {
    pub fn in_range(&self, bytes: &[u8]) -> bool {
        if bytes.len() != self.len { return false; }
        let b = as_code(bytes);
        self.from <= b && b <= self.to
    }
}

#[derive(Clone, Debug)]
pub struct CMapRange {
    from: u32,
    to: u32,
    start: u32
}

impl CMapRange {
    pub fn mapped_value(&self, codepoint: u32) -> Option<u32> {
        if self.from <= codepoint && codepoint <= self.to {
            Some(self.start + (codepoint - self.from))
        } else {
            None
        }
    }
}

#[derive(Default, Debug)]
pub struct CMap {
    pub name: String,
    pub version: String,
    pub cmap_type: i64,
    pub writing_mode: WritingMode,
    pub registry: String,
    pub ordering: String,
    pub supplement: u32,
    codespace_ranges: Vec<CodespaceRange>,
    unicode_mapping: HashMap<u32, String>,
    unicode_range_mapping: Vec<CMapRange>,
    cid_mapping: HashMap<u32, u32>,
    cid_range_mapping: Vec<CMapRange>
}

impl CMap {
    pub fn extract_codepoint(&self, codepoints: &[u8]) -> Option<usize> {
        let max_len = self.max_len_codespace();
        for i in 0..min(max_len+1, codepoints.len()) {
            let substr = &codepoints[0..(i+1)];
            for range in &self.codespace_ranges {
                if range.in_range(substr) {
                    return Some(i);
                }
            }
        }
        None
    }

    pub fn codepoint_to_cid(&self, codepoint: u32) -> u32 {
        if let Some(cid) = self.cid_mapping.get(&codepoint) {
            return *cid;
        }
        for range in &self.cid_range_mapping {
            if let Some(cid) = range.mapped_value(codepoint) {
                return cid;
            }
        }
        0 // If no mapping is found we have to return 0
    }

    pub fn codepoint_to_unicode(&self, codepoint: u32) -> Result<String> {
        if let Some(unicode) = self.unicode_mapping.get(&codepoint) {
            return Ok(unicode.to_owned());
        }
        for range in &self.unicode_range_mapping {
            if let Some(unicode) = range.mapped_value(codepoint) {
                return as_string(&from_code(unicode));
            }
        }
        Err(CMapError::NoUnicodeMappingFound(codepoint))
    }

    pub fn add_codespace_range(&mut self, range: CodespaceRange) {
        self.codespace_ranges.push(range);
    }

    pub fn add_unicode_mapping(&mut self, codepoints: &[u8], unicode: String) {
        self.unicode_mapping.insert(as_code(codepoints), unicode);
    }

    fn add_unicode_range(&mut self, range: CMapRange) {
        self.unicode_range_mapping.push(range);
    }

    pub fn add_cid_mapping(&mut self, codepoints: &[u8], cid: u32) {
        self.cid_mapping.insert(as_code(codepoints), cid);
    }

    fn add_cid_range(&mut self, range: CMapRange) {
        self.cid_range_mapping.push(range);
    }


    pub fn merge(&mut self, other: &CMap) {
        self.codespace_ranges.extend_from_slice(&other.codespace_ranges);
        self.unicode_mapping.extend(other.unicode_mapping.iter().map(|(k,v)| (k.clone(), v.clone())));
        self.cid_mapping.extend(other.cid_mapping.iter());
        self.cid_range_mapping.extend_from_slice(&other.cid_range_mapping);
    }

    fn max_len_codespace(&self) -> usize {
        let max_len = self.codespace_ranges.iter().max_by_key(|r| r.len);
        max_len.map(|r| r.len).unwrap_or(1)
    }

}

fn as_code(str: &[u8]) -> u32 {
    let mut code: u32 = 0;
    for c in str {
        code = (code << 8) | (*c as u32);
    }
    code
}

fn from_code(code: u32) -> [u8; 4]{
    let mut str = [0u8; 4];
    for (i, byte) in str.iter_mut().enumerate() {
        *byte = ((code >> (i*8)) & 0xFF) as u8;
    }
    str
}

fn as_string(str: &[u8]) -> Result<String> {
    match str.len() {
        0 => Ok(String::new()),
        1 => {
          // Luckily ISO 8859 maps 1:1 to unicode points
            Ok((str[0] as char).to_string())
        },
        _ => {
            let utf16: Result<Vec<String>> = str.chunks(2).map(|x| {
                if x.len() == 2 {
                    let code = x[0] as u16 | (x[1] as u16) << 8;
                    String::from_utf16(&[code]).map_err(CMapError::Utf16)
                } else {
                    Ok((x[0] as char).to_string())
                }
            }).collect();
            utf16.map(|v| v.concat())
        }
    }
}

fn increment_code(str: &mut [u8]) {
    let mut carry_bit = 1;
    for x in str.iter_mut().rev() {
        if carry_bit > 0 && *x == 255 {
            *x = 0;
        }
        else if carry_bit > 0 {
            *x += 1;
            carry_bit = 0;
        }
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use super::*;
    /*fn do_parse(input: &[u8]) {
        let result = parse(input);
        if let Ok(lines) = result  {
            for l in lines {
                println!("{:?}", l)
            }
        } else {
            println!("{:?}", result)
        }
    }*/
    #[test]
    fn it_works() {
        let mut f = File::open("assets/example").unwrap();
        let mut contents = Vec::new();
        f.read_to_end(&mut contents).unwrap();

        let cmap = parse_cmap(&contents).unwrap();
        println!("{:?}", cmap);
    }
}
