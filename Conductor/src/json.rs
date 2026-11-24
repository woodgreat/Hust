//! Minimal JSON reader for the core `info` protocol.
//!
//! The conductor deliberately avoids third-party crates so it can be built
//! offline. Only the subset needed by the protocol is implemented, but the
//! parser is complete enough to accept any valid JSON object.

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries
                .iter()
                .rev()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Json::Number(value)
                if value.is_finite() && *value >= 0.0 && *value <= u32::MAX as f64 =>
            {
                let rounded = value.trunc();
                if (rounded - value).abs() < f64::EPSILON {
                    Some(rounded as u32)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub fn parse(input: &str) -> Result<Json, String> {
    let mut parser = Parser {
        text: input,
        pos: 0,
    };
    let value = parser.value()?;
    parser.skip_whitespace();
    if parser.pos != parser.text.len() {
        return Err(parser.error("trailing characters"));
    }
    Ok(value)
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, message: &str) -> String {
        format!("invalid JSON at byte {}: {}", self.pos, message)
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.text[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), String> {
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(&format!("expected `{}`", expected as char)))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.text[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.error(&format!("expected `{}`", word)))
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while let Some(byte) = self.peek() {
            match byte {
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9' => self.pos += 1,
                _ => break,
            }
        }
        let raw = &self.text[start..self.pos];
        if raw.is_empty() {
            return Err(self.error("expected number"));
        }
        raw.parse::<f64>()
            .map(Json::Number)
            .map_err(|_| self.error("invalid number"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(out),
                Some('\\') => {
                    let escaped = self
                        .bump()
                        .ok_or_else(|| self.error("unterminated escape"))?;
                    match escaped {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let first = self.hex4()?;
                            let code = if (0xD800..=0xDBFF).contains(&first) {
                                if self.bump() != Some('\\') || self.bump() != Some('u') {
                                    return Err(self.error("unpaired high surrogate"));
                                }
                                let second = self.hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(self.error("invalid low surrogate"));
                                }
                                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                            } else {
                                first
                            };
                            out.push(
                                char::from_u32(code)
                                    .ok_or_else(|| self.error("invalid unicode escape"))?,
                            );
                        }
                        _ => return Err(self.error("invalid escape sequence")),
                    }
                }
                Some(ch) if (ch as u32) < 0x20 => {
                    return Err(self.error("control character in string"))
                }
                Some(ch) => out.push(ch),
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let start = self.pos;
        let end = start + 4;
        if end > self.text.len() || !self.text.is_char_boundary(end) {
            return Err(self.error("truncated unicode escape"));
        }
        let raw = &self.text[start..end];
        let value =
            u32::from_str_radix(raw, 16).map_err(|_| self.error("invalid unicode escape"))?;
        self.pos = end;
        Ok(value)
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("expected `,` or `]`")),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(entries));
                }
                _ => return Err(self.error("expected `,` or `}`")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_object() {
        let value = parse(r#"{"a":"b","n":3,"ok":true,"no":false,"nil":null}"#).unwrap();
        assert_eq!(value.get("a").and_then(Json::as_str), Some("b"));
        assert_eq!(value.get("n").and_then(Json::as_u32), Some(3));
        assert_eq!(value.get("ok").and_then(Json::as_bool), Some(true));
        assert_eq!(value.get("no").and_then(Json::as_bool), Some(false));
        assert_eq!(value.get("nil"), Some(&Json::Null));
    }

    #[test]
    fn parses_nested_values_and_escapes() {
        let value = parse(r#"{"a":[1,{"b":"x\ny"}],"u":"\u4f60\u597d"}"#).unwrap();
        assert_eq!(value.get("u").and_then(Json::as_str), Some("你好"));
        match value.get("a") {
            Some(Json::Array(items)) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[1].get("b").and_then(Json::as_str), Some("x\ny"));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse(r#"{"a":1} trailing"#).is_err());
    }

    #[test]
    fn rejects_malformed_objects() {
        assert!(parse(r#"{"a":}"#).is_err());
        assert!(parse(r#"{"a" 1}"#).is_err());
        assert!(parse(r#"{"a":1"#).is_err());
    }

    #[test]
    fn u32_requires_an_integer() {
        let value = parse(r#"{"a":1.5,"b":-1,"c":7.0}"#).unwrap();
        assert_eq!(value.get("a").and_then(Json::as_u32), None);
        assert_eq!(value.get("b").and_then(Json::as_u32), None);
        assert_eq!(value.get("c").and_then(Json::as_u32), Some(7));
    }
}
