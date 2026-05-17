use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellError {
    message: String,
}

impl ShellError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ShellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ShellError {}

pub fn split_words(input: &str) -> Result<Vec<String>, ShellError> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut chars = input.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(character) = chars.next() {
        match quote {
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                } else {
                    current.push(character);
                    started = true;
                }
            }
            Some('"') => match character {
                '"' => quote = None,
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                        started = true;
                    } else {
                        current.push('\\');
                        started = true;
                    }
                }
                _ => {
                    current.push(character);
                    started = true;
                }
            },
            Some(_) => unreachable!("only single and double quotes are used"),
            None => match character {
                '\'' | '"' => {
                    quote = Some(character);
                    started = true;
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    } else {
                        current.push('\\');
                    }
                    started = true;
                }
                character if character.is_whitespace() => {
                    if started {
                        words.push(std::mem::take(&mut current));
                        started = false;
                    }
                }
                _ => {
                    current.push(character);
                    started = true;
                }
            },
        }
    }

    if let Some(open_quote) = quote {
        return Err(ShellError::new(format!(
            "Unterminated {open_quote} quote in task arguments"
        )));
    }

    if started {
        words.push(current);
    }
    Ok(words)
}

pub fn quote_arg(input: &str) -> String {
    if input.is_empty() {
        return "''".to_owned();
    }

    if input
        .chars()
        .all(|character| matches!(character, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.' | '/' | ':' | '@' | '%' | '+' | '=' | ','))
    {
        return input.to_owned();
    }

    let mut quoted = String::with_capacity(input.len() + 2);
    quoted.push('\'');
    for character in input.chars() {
        if character == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(character);
        }
    }
    quoted.push('\'');
    quoted
}

pub fn quote_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| quote_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{quote_arg, quote_args, split_words};

    #[test]
    fn splits_shell_like_words() {
        assert_eq!(
            split_words("build -- --name 'Jane Doe' \"x y\"").unwrap(),
            ["build", "--", "--name", "Jane Doe", "x y"]
        );
    }

    #[test]
    fn keeps_empty_quoted_arguments() {
        assert_eq!(split_words("task ''").unwrap(), ["task", ""]);
    }

    #[test]
    fn quotes_round_trip_arguments() {
        let args = vec!["hello world".to_owned(), "it's".to_owned()];
        let quoted = quote_args(&args);
        assert_eq!(split_words(&quoted).unwrap(), args);
        assert_eq!(quote_arg("abc-123"), "abc-123");
    }
}
