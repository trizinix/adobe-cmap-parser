use std::str;

use ::*;
use error::*;
use lexer::{Value, lexer};

pub fn parse_cmap(input: &[u8]) -> Result<CMap> {
    let lexems = lexer(input).unwrap();

    let mut cmap: CMap = Default::default();

    let mut i = 0;
    while i < lexems.len() {
        match lexems[i] {
            Value::Operator(ref op) => {
                match op.as_ref() {
                    "beginbfchar" => {
                        let size = lexems[i-1].as_integer()?;
                        for k in 0..(size as usize) {
                            let idx = i + 1 + 2*k;
                            let char_code = lexems[idx].as_literal_string()?;

                            match lexems[idx+1] {
                                Value::LiteralString(ref output_code) => {
                                    let unicode = str::from_utf8(output_code).map_err(CMapError::Utf8)?;
                                    cmap.add_unicode_mapping(char_code, unicode.to_owned())
                                },
                                Value::Name(ref output_name) => {
                                    let name = str::from_utf8(output_name).map_err(CMapError::Utf8)?;
                                    cmap.add_unicode_mapping(char_code, name.to_owned());
                                },
                                _ => {
                                    return lexems[idx+1].expect_type(&"Literal String or Name");
                                }
                            }

                        }
                        i += (1 + 2*(size + 1)) as usize;
                    },
                    "beginbfrange" => {
                        let size = lexems[i-1].as_integer()?;
                        for k in 0..(size as usize) {
                            let idx = i + 1 + 3*k;
                            let upper_code = lexems[idx].as_literal_string()?;
                            let lower_code = lexems[idx+1].as_literal_string()?;
                            match lexems[idx+2] {
                                Value::LiteralString(ref start) => {
                                    //let mut idx: Vec<u8> = lower_code.to_owned();
                                    //let mut uni = start.clone();
                                    // inclusive ranges would be nice
                                    let range = CMapRange {
                                        from: as_code(lower_code),
                                        to: as_code(upper_code),
                                        start: as_code(start)
                                    };
                                    cmap.add_unicode_range(range);
                                    /*
                                    for _ in as_code(lower_code)..as_code(upper_code)+1 {
                                        let mut unicode = String::from_utf8(uni.to_owned())
                                            .map_err(|e| CMapError::Utf8(e.utf8_error()))?;
                                        cmap.add_unicode_mapping(&idx, unicode);

                                        increment_code(&mut idx);
                                        increment_code(&mut uni);
                                    }*/
                                }
                                Value::Array(ref codes) => {
                                    let expected_len = (as_code(upper_code) - as_code(lower_code) + 1) as usize;
                                    if expected_len as usize != codes.len() {
                                        return Err(CMapError::InvalidArrayLength { expected: expected_len as usize, found: codes.len()});
                                    }
                                    let mut idx = lower_code.to_owned();
                                    for c in 0..expected_len {
                                        let uni = codes[c as usize].as_literal_string()?;
                                        let mut unicode = String::from_utf8(uni.to_owned())
                                            .map_err(|e| CMapError::Utf8(e.utf8_error()))?;


                                        cmap.add_unicode_mapping(&idx, unicode);

                                        increment_code(&mut idx)
                                    }
                                }
                                _ => { return lexems[idx+2].expect_type(&"Literal String or Array"); }
                            }
                        }
                        i += (1 + 3*(size + 1)) as usize;

                    },
                    "begincodespacerange" => {
                        let size = lexems[i-1].as_integer()?;
                        for k in 0..(size as usize) {
                            let idx = i + 1 + 2 * k;
                            let upper_code = lexems[idx].as_literal_string()?;
                            let lower_code = lexems[idx+1].as_literal_string()?;

                            let range = CodespaceRange {
                                from: as_code(lower_code),
                                to: as_code(upper_code),
                                len: upper_code.len()
                            };

                            cmap.add_codespace_range(range);
                        }
                        i += (1 + 2*(size + 1)) as usize;
                    },
                    "begincidchar" => {
                        let size = lexems[i-1].as_integer()?;
                        for k in 0..(size as usize) {
                            let idx = i + 1 + 2 * k;
                            let char_code = lexems[idx].as_literal_string()?;
                            let cid = lexems[idx+1].as_integer()?;
                            cmap.add_cid_mapping(char_code, cid as u32);
                        }
                        i += (1 + 2*(size + 1)) as usize;
                    },
                    "begincidrange" => {
                        let size = lexems[i-1].as_integer()?;
                        for k in 0..(size as usize) {
                            let idx = i + 1 + 3 * k;
                            let upper_code = lexems[idx].as_literal_string()?;
                            let lower_code = lexems[idx+1].as_literal_string()?;

                            let start = lexems[idx+2].as_integer()?;

                            let range = CMapRange {
                                from: as_code(upper_code),
                                to: as_code(lower_code),
                                start: start as u32
                            };

                            cmap.add_cid_range(range);
                        }
                        i += (1 + 3*(size + 1)) as usize;
                    },
                    "usecmap" => {
                        let other_cmap = lexems[i-1].as_literal_string()?;
                        let referenced_cmap = str::from_utf8(other_cmap).map_err(CMapError::Utf8)?;
                        i += 2;
                    },
                    "endcmap" => { break; },
                    _ => {
                        //return Err(CMapError::UnknownOperator(op.to_owned()));
                        i += 1;
                    }
                }
            },
            Value::Name(ref s) => {
                match str::from_utf8(&s).unwrap() {
                    "WMode" => {
                        if let Value::Integer(mode) = lexems[i+1] {
                            cmap.writing_mode = WritingMode::from(mode != 0);
                        }
                    },
                    "CMapName" => {
                        if let Value::Name(ref name) = lexems[i+1] {
                            cmap.name = String::from(str::from_utf8(name).unwrap());
                        }
                    },
                    "CMapVersion" => {
                        if let Value::Integer(version) = lexems[i+1] {
                            cmap.version = version.to_string();
                        }
                        if let Value::LiteralString(ref version) = lexems[i+1] {
                            cmap.version = String::from(str::from_utf8(version).unwrap());
                        }
                    },
                    "CMapType" => {
                        if let Value::Integer(cmap_type) = lexems[i+1] {
                            cmap.cmap_type = cmap_type;
                        }

                    },
                    "Registry" => {
                        if let Value::LiteralString(ref registry) = lexems[i+1] {
                            cmap.registry = String::from(str::from_utf8(registry).unwrap());
                        }
                    },
                    "Ordering" => {
                        if let Value::LiteralString(ref ordering) = lexems[i+1] {
                            cmap.ordering = String::from(str::from_utf8(ordering).unwrap());
                        }
                    },
                    "Supplement" => {
                        if let Value::Integer(supplement) = lexems[i+1] {
                            cmap.supplement = supplement as u32;
                        }
                    },
                    _ => { if i > 0 { i -= 1; } /* Since we didn't consume an argument */}
                }
                i += 2;
            }
            _ => { i += 1; }
        }
    }

    Ok(cmap)
}

/*pub fn get_unicode_map(input: &[u8]) -> Result<HashMap<u32, u32>, &'static str> {
    let lexed = parse(&input).expect("failed to parse");

    let mut i = 0;
    let mut map = HashMap::new();
    while i < lexed.len() {
        match lexed[i] {
            Value::Operator(ref o) => {
                match o.as_ref() {
                    "beginbfchar" => {
                        let count = if let &Value::Integer(ref c) = &lexed[i-1] { Ok(*c) } else { Err("beginbfchar exected int") }?;
                        i += 1;
                        for _ in 0..count {
                            let char_code = if let &Value::LiteralString(ref s) = &lexed[i] { Ok(s) } else { Err("beginbfchar exected hexstring") }?;
                            let uni_code = if let &Value::LiteralString(ref s) = &lexed[i+1] { Ok(s) } else { Err("beginbfchar exected hexstring") }?;
                            //let char_code =
                            map.insert(as_code(char_code), as_code(uni_code));
                            i += 2;
                        }
                        i += 1;
                    }
                    "beginbfrange" => {
                        let count = if let &Value::Integer(ref c) = &lexed[i-1] { Ok(*c) } else { Err("beginbfrange exected int") }?;
                        i += 1;
                        for _ in 0..count {
                            let lower_code = if let &Value::LiteralString(ref s) = &lexed[i] { Ok(as_code(s)) } else { Err("beginbfrange exected hexstring") }?;
                            let upper_code = if let &Value::LiteralString(ref s) = &lexed[i+1] { Ok(as_code(s)) } else { Err("beginbfrange exected hexstring") }?;
                            match &lexed[i+2] {
                                &Value::LiteralString(ref start) => {
                                    let mut unicode = as_code(start);
                                    // inclusive ranges would be nice
                                    for c in lower_code..upper_code+1 {
                                        map.insert(c, unicode);
                                        unicode += 1;
                                    }
                                }
                                &Value::Array(ref codes) => {
                                    // inclusive ranges would be nice
                                    let mut i = 0;
                                    if (upper_code - lower_code + 1) as usize != codes.len() {
                                        return Err("bad length of array");
                                    }
                                    for c in lower_code..upper_code+1 {
                                        map.insert(c, if let &Value::LiteralString(ref s) = &codes[i] { Ok(as_code(s)) } else { Err("beginbfrange exected hexstring") }?);
                                        i += 1;
                                    }
                                }
                                _ => { return Err("beginbfrange exected array or literal") }
                            }
                            i += 3;
                        }
                        i += 1;
                    }
                    _ => { i += 1; }
                }

            }
            _ => { i += 1; }
        }
    }
    Ok(map)

}
*/