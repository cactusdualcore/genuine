#[derive(Debug)]
pub struct Path {
    path: String,
    parts: Vec<Part>,
}

impl AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        &self.path
    }
}

#[allow(dead_code)]
impl Path {
    pub fn new(path: String) -> Result<Self, ParseError> {
        let parts = {
            let mut parser = parser::Parser::new(path.as_bytes());
            parser.parse()?
        };

        Ok(Self { path, parts })
    }

    pub fn matches(&self, path: &str) -> Option<Vec<Match>> {
        let mut matches = Vec::new();

        let mut bytes = path.as_bytes();
        for part in &self.parts {
            match part {
                Part::Literal(literal) => match bytes.strip_prefix(literal.as_slice()) {
                    Some(tail) => bytes = tail,
                    None => return None,
                },
                Part::Param { name } => {
                    let mut parser = parser::Parser::new(bytes);
                    let parameter = {
                        let segment = parser.segment().to_vec();
                        String::from_utf8(segment).unwrap()
                    };
                    bytes = &bytes[parameter.len()..];
                    matches.push(Match {
                        name: name.clone(),
                        value: parameter,
                    });
                }
            }
        }

        Some(matches)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub name: String,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub enum Part {
    Literal(Vec<u8>),
    Param { name: String },
}

pub use parser::ParseError;

/// Parse route paths with optional named parameters.
///
/// ```text
/// path           : '/'
///                | ( '/' segment-or-param )+
/// segmentOrParam : segment
///                | param
/// segment        : pchar*
/// pchar          : unreserved
///                | pct-encoded
///                | sub-delims
///                | ':'
///                | '@'
/// unreserved     : ALPHA
///                | DIGIT
///                | "-"
///                | "."
///                | "_"
///                | "~"
/// pct-encoded    : '%' HEXDIG HEXDIG
/// sub-delims     : '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
/// param          : '{' SPACE* name SPACE* '}'
/// name           : ALPHA ( ALPHA | DIGIT )*
/// ```
mod parser {
    use super::Part;

    #[derive(Debug, thiserror::Error, PartialEq)]
    pub enum ParseError {
        #[error("encountered byte 0x{actual:x} at position {pos}, but expected 0x{expected:x}")]
        ExpectedExact {
            expected: u8,
            actual: u8,
            pos: usize,
        },
        #[error("encountered byte 0x{actual:x} at position {pos}, but expected {expected}")]
        Expected {
            expected: &'static str,
            actual: u8,
            pos: usize,
        },
        #[error("encountered unexpected end of stream at position {pos}")]
        EndOfStream { pos: usize },
        #[error("route paths must start with a '/'")]
        IsNotAbsolute,
    }

    type Result<T> = std::result::Result<T, ParseError>;

    pub struct Parser<'b> {
        /// Route path to parse.
        bytes: &'b [u8],
        /// start position of current literal.
        anchor: usize,
        /// current parse position.
        cursor: usize,
    }

    impl<'b> Parser<'b> {
        /// Create a new [`Parser`].
        pub fn new(bytes: &'b [u8]) -> Self {
            Self {
                bytes,
                anchor: 0,
                cursor: 0,
            }
        }

        /// Parse a route path.
        pub fn parse(&'b mut self) -> Result<Vec<Part>> {
            self.consume(b'/').map_err(|_| ParseError::IsNotAbsolute)?;

            // According to the [specification][spec], the first path segment
            // must not exist or be of non-zero length.
            //
            // [spec]: https://www.rfc-editor.org/rfc/rfc3986#section-3.3
            {
                if self.consume(b'/').is_ok() {
                    return Err(ParseError::IsNotAbsolute);
                }
            }

            let mut parts = Vec::new();
            while self.consume(b'/').is_ok() {
                match self.peek() {
                    Some(b'{') => {
                        let literal = self.bytes[self.anchor..self.cursor].to_vec();
                        parts.push(Part::Literal(literal));

                        // consume the brace
                        self.cursor += 1;

                        self.ws();
                        let name = self.parameter_name()?;

                        self.ws();
                        self.consume(b'}')?;

                        self.anchor = self.cursor;
                        parts.push(Part::Param { name });
                    }
                    _ => {
                        let _ = self.segment();
                    }
                }
            }

            let tail = &self.bytes[self.anchor..self.cursor];
            if tail.is_empty() {
                debug_assert_eq!(self.cursor, self.bytes.len());
            } else {
                parts.push(Part::Literal(tail.to_vec()));
            }

            Ok(parts)
        }

        /// Parses a parameter name.
        fn parameter_name(&mut self) -> Result<String> {
            let name = {
                let (name_bytes, ()) = self.capture(|parser| {
                    assert!(parser.any()?.is_ascii_alphabetic());
                    parser.skip_while(move |x| x.is_ascii_alphanumeric());
                    Ok(())
                })?;

                std::str::from_utf8(name_bytes)
                    .unwrap_or_else(|_| unreachable!())
                    .to_string()
            };

            Ok(name)
        }

        /// Parses a `segment`. This is defined by [RFC 3986][rfc].
        ///
        /// [rfc]: https://www.rfc-editor.org/rfc/rfc3986#section-3.3
        pub fn segment(&mut self) -> &[u8] {
            self.capture(|parser| {
                loop {
                    match parser.pchar() {
                        Ok(_) => (),
                        Err(ParseError::EndOfStream { .. }) => break,
                        Err(_) => {
                            parser.cursor -= 1;
                            break;
                        }
                    }
                }

                Ok(())
            })
            .unwrap()
            .0
        }

        /// Parses a `pchar`. This is defined by [RFC 3986][rfc].
        ///
        /// [rfc]: https://www.rfc-editor.org/rfc/rfc3986#section-3.3
        fn pchar(&mut self) -> Result<u8> {
            if let Some(x) = self.unreserved() {
                Ok(x)
            } else {
                self.percent_encoded()
                    .or_else(|_| self.sub_delimiter())
                    .or_else(|_| self.consume(b'@'))
            }
        }

        /// Parses a unreserved character. This is defined by [RFC 3986][rfc].
        ///
        /// [rfc]: https://www.rfc-editor.org/rfc/rfc3986#section-2.3
        fn unreserved(&mut self) -> Option<u8> {
            self.peek()
                .filter(|x| x.is_ascii_alphanumeric() || matches!(x, b'-' | b'.' | b'_' | b'~'))
                .inspect(|_| self.cursor += 1)
        }

        /// Parses a percent-encoded byte. This is defined by [RFC 3986][rfc].
        ///
        /// [rfc]: https://www.rfc-editor.org/rfc/rfc3986#section-2.1
        fn percent_encoded(&mut self) -> Result<u8> {
            self.consume(b'%')?;

            let upper = self.hex_digit()?;
            let lower = self.hex_digit()?;

            Ok((upper << 4) | lower)
        }

        /// Parses a single hexadecimal digit and returns its numeric value, or
        /// a [`ParseError`] if the byte is no valid hexadecimal digit.
        fn hex_digit(&mut self) -> Result<u8> {
            Ok(match self.any()? {
                x @ b'0'..=b'9' => x - b'0',
                x @ b'A'..=b'F' => x - b'A' + 0xA,
                x @ b'a'..=b'f' => x - b'a' + 0xA,
                x => return Err(self.expected("a hex digit", x)),
            })
        }

        /// Parses a sub-delimiter. This is defined by [RFC 3986][rfc].
        ///
        /// [rfc]: https://www.rfc-editor.org/rfc/rfc3986#section-2.2
        fn sub_delimiter(&mut self) -> Result<u8> {
            match self.any() {
                ok @ Ok(
                    b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=',
                ) => ok,
                Ok(x) => Err(self.expected("a sub delimiter", x)),
                Err(err) => Err(err),
            }
        }

        fn ws(&mut self) {
            while let Some(b' ' | b'\t') = self.peek() {
                self.cursor += 1;
            }
        }

        fn expected(&self, what: &'static str, actual: u8) -> ParseError {
            ParseError::Expected {
                expected: what,
                actual,
                pos: self.cursor,
            }
        }

        fn skip_while<P>(&mut self, predicate: P)
        where
            P: Fn(u8) -> bool,
        {
            while self.peek().is_some_and(&predicate) {
                self.cursor += 1;
            }
        }

        fn capture<F, T>(&mut self, parse: F) -> Result<(&[u8], T)>
        where
            F: FnOnce(&mut Self) -> Result<T>,
        {
            let start = self.cursor;
            let x = parse(self)?;
            let slice = &self.bytes[start..self.cursor];
            Ok((slice, x))
        }

        /// Consumes the next byte if it matches the expected value. Advances
        /// the cursor if successful, or returns a [`ParseError`] if the byte
        /// does not match or if the input ends unexpectedly.
        fn consume(&mut self, expected: u8) -> Result<u8> {
            match self.peek() {
                Some(x) if x == expected => {
                    self.cursor += 1;
                    Ok(x)
                }
                Some(x) => Err(ParseError::ExpectedExact {
                    expected,
                    actual: x,
                    pos: self.cursor,
                }),
                None => Err(ParseError::EndOfStream { pos: self.cursor }),
            }
        }

        /// Consumes and returns the next byte, advancing the cursor or returns
        /// a [`ParseError`] if the end of input is reached.
        fn any(&mut self) -> Result<u8> {
            match self.bytes.get(self.cursor).copied() {
                Some(x) => {
                    self.cursor += 1;
                    Ok(x)
                }
                None => Err(ParseError::EndOfStream { pos: self.cursor }),
            }
        }

        /// Peeks at the next byte without consuming it or returns [`None`] if
        /// the end of input is reached.
        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.cursor).copied()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        #[should_panic]
        fn consume_rejects_unexpected_byte() {
            Parser::new(b"BBB")
                .consume(b'A')
                .expect("could not consume b'A'");
        }

        #[test]
        fn consume_takes_first_byte() {
            let mut parser = Parser::new(b"ABB");
            assert_eq!(parser.consume(b'A'), Ok(b'A'));
            assert_eq!(parser.cursor, 1);
        }

        #[test]
        fn skip_while_skips_while_predicate_is_true() {
            let mut parser = Parser::new(b"ABC123");
            parser.skip_while(|x| x.is_ascii_alphabetic());
            assert_eq!(parser.cursor, 3);
        }

        #[test]
        fn segment_does_not_consume_slash() {
            let mut parser = Parser::new(b"abc/def/");
            parser.segment();
            assert_eq!(parser.cursor, 3);
        }

        fn parse_path_and_compare(path: &str, expected: &[Part]) {
            let parts = Parser::new(path.as_bytes()).parse().unwrap();
            assert_eq!(expected, &parts);
        }

        #[test]
        fn parse_literal_url_path() {
            const PATH: &str = "/url/path/to/parse";
            let expected = &[Part::Literal(PATH.into())];
            parse_path_and_compare(PATH, expected);
        }

        #[test]
        fn parse_literal_url_path_with_trailing_slash() {
            const PATH: &str = "/url/path/to/parse/";
            let expected = &[Part::Literal(PATH.into())];
            parse_path_and_compare(PATH, expected);
        }

        #[test]
        fn parse_literal_url_path_without_leading_slash() {
            let mut parser = Parser::new(b"url/path/to/parse/");
            assert_eq!(parser.parse(), Err(ParseError::IsNotAbsolute));
        }

        #[test]
        fn path_url_path_with_trailing_parameter() {
            const PREFIX: &str = "/url/with/";
            const PARAM: &str = "parameters";

            let expected = &[
                Part::Literal(PREFIX.into()),
                Part::Param { name: PARAM.into() },
            ];

            parse_path_and_compare(&format!("{}{{{}}}", PREFIX, PARAM), expected);
        }

        #[test]
        fn root_path_is_valid() {
            let mut parser = Parser::new(b"/");
            assert_eq!(parser.parse(), Err(ParseError::IsNotAbsolute));
        }
    }
}
