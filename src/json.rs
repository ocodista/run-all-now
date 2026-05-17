use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object()?
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    message: String,
}

impl JsonError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for JsonError {}

pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.is_finished() {
        Ok(value)
    } else {
        Err(parser.error("unexpected trailing characters"))
    }
}

pub fn quote_string(input: &str) -> String {
    let mut output = String::with_capacity(input.len() + 2);
    output.push('"');
    for character in input.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'n') => self.parse_literal(b"null", JsonValue::Null),
            Some(b't') => self.parse_literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", JsonValue::Bool(false)),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(_) => Err(self.error("unexpected token")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn parse_literal(&mut self, literal: &[u8], value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.bytes[self.position..].starts_with(literal) {
            self.position += literal.len();
            Ok(value)
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect_byte(b'"')?;
        let mut output = String::new();

        loop {
            let Some(byte) = self.next_byte() else {
                return Err(self.error("unterminated string"));
            };

            match byte {
                b'"' => return Ok(output),
                b'\\' => self.parse_escape(&mut output)?,
                0x00..=0x1f => return Err(self.error("control character in string")),
                _ => {
                    let start = self.position - 1;
                    let character = self.input[start..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("invalid utf-8"))?;
                    self.position = start + character.len_utf8();
                    output.push(character);
                }
            }
        }
    }

    fn parse_escape(&mut self, output: &mut String) -> Result<(), JsonError> {
        let Some(byte) = self.next_byte() else {
            return Err(self.error("unterminated escape"));
        };

        match byte {
            b'"' => output.push('"'),
            b'\\' => output.push('\\'),
            b'/' => output.push('/'),
            b'b' => output.push('\u{08}'),
            b'f' => output.push('\u{0c}'),
            b'n' => output.push('\n'),
            b'r' => output.push('\r'),
            b't' => output.push('\t'),
            b'u' => {
                let code = self.parse_hex_quad()?;
                let character = if (0xd800..=0xdbff).contains(&code) {
                    self.expect_byte(b'\\')?;
                    self.expect_byte(b'u')?;
                    let low = self.parse_hex_quad()?;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err(self.error("invalid unicode surrogate pair"));
                    }
                    let scalar = 0x1_0000 + (((code - 0xd800) << 10) | (low - 0xdc00));
                    char::from_u32(scalar).ok_or_else(|| self.error("invalid unicode scalar"))?
                } else {
                    char::from_u32(code).ok_or_else(|| self.error("invalid unicode scalar"))?
                };
                output.push(character);
            }
            _ => return Err(self.error("invalid escape")),
        }

        Ok(())
    }

    fn parse_hex_quad(&mut self) -> Result<u32, JsonError> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let Some(byte) = self.next_byte() else {
                return Err(self.error("unterminated unicode escape"));
            };
            value = value
                .checked_mul(16)
                .and_then(|current| current.checked_add(hex_value(byte)?))
                .ok_or_else(|| self.error("invalid unicode escape"))?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.position;
        if self.peek_byte() == Some(b'-') {
            self.position += 1;
        }

        match self.peek_byte() {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => {
                self.position += 1;
                while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(self.error("invalid number")),
        }

        if self.peek_byte() == Some(b'.') {
            self.position += 1;
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.error("invalid number"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.error("invalid number"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        Ok(JsonValue::Number(
            self.input[start..self.position].to_owned(),
        ))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_byte(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.peek_byte() == Some(b']') {
            self.position += 1;
            return Ok(JsonValue::Array(values));
        }

        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            match self.next_byte() {
                Some(b',') => self.skip_whitespace(),
                Some(b']') => return Ok(JsonValue::Array(values)),
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_byte(b'{')?;
        self.skip_whitespace();
        let mut entries = Vec::new();
        if self.peek_byte() == Some(b'}') {
            self.position += 1;
            return Ok(JsonValue::Object(entries));
        }

        loop {
            self.skip_whitespace();
            if self.peek_byte() != Some(b'"') {
                return Err(self.error("expected object key"));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.next_byte() {
                Some(b',') => self.skip_whitespace(),
                Some(b'}') => return Ok(JsonValue::Object(entries)),
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), JsonError> {
        match self.next_byte() {
            Some(actual) if actual == expected => Ok(()),
            _ => Err(self.error(format!("expected '{}'", expected as char))),
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.position += 1;
        Some(byte)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn is_finished(&self) -> bool {
        self.position >= self.bytes.len()
    }

    fn error(&self, message: impl Into<String>) -> JsonError {
        JsonError::new(format!("{} at byte {}", message.into(), self.position))
    }
}

fn hex_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a' + 10)),
        b'A'..=b'F' => Some(u32::from(byte - b'A' + 10)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, quote_string, JsonValue};

    #[test]
    fn parses_object_order_and_strings() {
        let value =
            parse(r#"{"scripts":{"build":"node build.js","test":"cargo test"},"name":"demo"}"#)
                .expect("valid json");
        let scripts = value.get("scripts").and_then(JsonValue::as_object).unwrap();
        assert_eq!(scripts[0].0, "build");
        assert_eq!(scripts[1].0, "test");
        assert_eq!(value.get("name").and_then(JsonValue::as_str), Some("demo"));
    }

    #[test]
    fn parses_unicode_escape() {
        let value = parse(r#""hello \u263a""#).expect("valid json");
        assert_eq!(value.as_str(), Some("hello ☺"));
    }

    #[test]
    fn quotes_json_strings() {
        assert_eq!(quote_string("a\nb"), r#""a\nb""#);
        assert_eq!(quote_string("a\"b"), r#""a\"b""#);
    }
}
