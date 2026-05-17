use crate::shell;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchError {
    message: String,
}

impl MatchError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MatchError {}

pub fn apply_arguments(patterns: &[String], args: &[String]) -> Result<Vec<String>, MatchError> {
    if !patterns.iter().any(|pattern| pattern.contains('{')) {
        return Ok(patterns.to_vec());
    }

    let mut defaults = HashMap::new();
    patterns
        .iter()
        .map(|pattern| apply_arguments_to_pattern(pattern, args, &mut defaults))
        .collect()
}

pub fn match_tasks(task_list: &[String], patterns: &[String]) -> Result<Vec<String>, MatchError> {
    let filters = patterns
        .iter()
        .map(|pattern| Filter::new(pattern))
        .collect::<Vec<_>>();
    let mut task_set = TaskSet::default();
    let mut unknown_tasks = Vec::new();

    for filter in &filters {
        let mut found = false;
        for task_name in task_list {
            if glob_matches(&filter.task, task_name) {
                found = true;
                task_set.add(format!("{}{}", task_name, filter.args), &filter.task);
            }
        }

        if !found && matches!(filter.task.as_str(), "restart" | "env") {
            task_set.add(format!("{}{}", filter.task, filter.args), &filter.task);
            found = true;
        }

        if !found && !unknown_tasks.iter().any(|task| task == &filter.task) {
            unknown_tasks.push(filter.task.clone());
        }
    }

    if unknown_tasks.is_empty() {
        Ok(task_set.result)
    } else {
        Err(MatchError::new(format!(
            "Task not found: \"{}\"",
            unknown_tasks.join("\", \"")
        )))
    }
}

fn apply_arguments_to_pattern(
    pattern: &str,
    args: &[String],
    defaults: &mut HashMap<String, String>,
) -> Result<String, MatchError> {
    let mut output = String::with_capacity(pattern.len());
    let mut remainder = pattern;

    while let Some(open_position) = remainder.find('{') {
        output.push_str(&remainder[..open_position]);
        let after_open = &remainder[open_position + 1..];
        let Some(close_position) = after_open.find('}') else {
            output.push_str(&remainder[open_position..]);
            return Ok(output);
        };

        let body = &after_open[..close_position];
        match replace_placeholder(body, args, defaults)? {
            Some(replacement) => output.push_str(&replacement),
            None => {
                output.push('{');
                output.push_str(body);
                output.push('}');
            }
        }
        remainder = &after_open[close_position + 1..];
    }

    output.push_str(remainder);
    Ok(output)
}

fn replace_placeholder(
    body: &str,
    args: &[String],
    defaults: &mut HashMap<String, String>,
) -> Result<Option<String>, MatchError> {
    if let Some(stripped) = body.strip_prefix('!') {
        if is_placeholder_id(stripped) {
            return Err(MatchError::new(format!(
                "Invalid Placeholder: {{!{stripped}}}"
            )));
        }
        return Ok(None);
    }

    if body == "@" {
        return Ok(Some(shell::quote_args(args)));
    }
    if body == "*" {
        return Ok(Some(shell::quote_arg(&args.join(" "))));
    }

    let digit_count = body.bytes().take_while(u8::is_ascii_digit).count();
    if digit_count == 0 {
        return Ok(None);
    }

    let id = &body[..digit_count];
    let options = &body[digit_count..];
    let position = id
        .parse::<usize>()
        .map_err(|_| MatchError::new(format!("Invalid Placeholder: {{{body}}}")))?;

    if position >= 1 && position <= args.len() {
        return Ok(Some(shell::quote_arg(&args[position - 1])));
    }

    if let Some(value) = options.strip_prefix(":=") {
        let quoted = shell::quote_arg(value);
        defaults.insert(id.to_owned(), quoted.clone());
        return Ok(Some(quoted));
    }
    if let Some(value) = options.strip_prefix(":-") {
        return Ok(Some(shell::quote_arg(value)));
    }
    if !options.is_empty() {
        return Err(MatchError::new(format!("Invalid Placeholder: {{{body}}}")));
    }
    if let Some(value) = defaults.get(id) {
        return Ok(Some(value.clone()));
    }

    Ok(Some(String::new()))
}

fn is_placeholder_id(input: &str) -> bool {
    matches!(input, "@" | "*") || input.bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Debug)]
struct Filter {
    task: String,
    args: String,
}

impl Filter {
    fn new(pattern: &str) -> Self {
        let trimmed = pattern.trim();
        match trimmed.find(' ') {
            Some(space_position) => Self {
                task: trimmed[..space_position].to_owned(),
                args: trimmed[space_position..].to_owned(),
            },
            None => Self {
                task: trimmed.to_owned(),
                args: String::new(),
            },
        }
    }
}

#[derive(Default)]
struct TaskSet {
    result: Vec<String>,
    sources_by_command: HashMap<String, Vec<String>>,
}

impl TaskSet {
    fn add(&mut self, command: String, source: &str) {
        let sources = self.sources_by_command.entry(command.clone()).or_default();
        if sources.is_empty() || sources.iter().any(|candidate| candidate == source) {
            self.result.push(command);
        }
        sources.push(source.to_owned());
    }
}

fn glob_matches(pattern: &str, task_name: &str) -> bool {
    let pattern_segments = pattern.split(':').collect::<Vec<_>>();
    let task_segments = task_name.split(':').collect::<Vec<_>>();
    match_segments(&pattern_segments, &task_segments)
}

fn match_segments(pattern_segments: &[&str], task_segments: &[&str]) -> bool {
    if pattern_segments.is_empty() {
        return task_segments.is_empty();
    }

    if pattern_segments[0] == "**" {
        for consumed in 0..=task_segments.len() {
            if match_segments(&pattern_segments[1..], &task_segments[consumed..]) {
                return true;
            }
        }
        return false;
    }

    if task_segments.is_empty() {
        return false;
    }

    match_segment(pattern_segments[0], task_segments[0])
        && match_segments(&pattern_segments[1..], &task_segments[1..])
}

fn match_segment(pattern: &str, text: &str) -> bool {
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    let text_chars = text.chars().collect::<Vec<_>>();
    match_segment_chars(&pattern_chars, &text_chars)
}

fn match_segment_chars(pattern: &[char], text: &[char]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    match pattern[0] {
        '*' => {
            let mut rest = &pattern[1..];
            while matches!(rest.first(), Some('*')) {
                rest = &rest[1..];
            }
            if rest.is_empty() {
                return true;
            }
            (0..=text.len()).any(|index| match_segment_chars(rest, &text[index..]))
        }
        '?' => !text.is_empty() && match_segment_chars(&pattern[1..], &text[1..]),
        '[' => match_char_class(pattern, text),
        '\\' => {
            if pattern.len() >= 2 {
                !text.is_empty()
                    && pattern[1] == text[0]
                    && match_segment_chars(&pattern[2..], &text[1..])
            } else {
                !text.is_empty()
                    && text[0] == '\\'
                    && match_segment_chars(&pattern[1..], &text[1..])
            }
        }
        character => {
            !text.is_empty()
                && character == text[0]
                && match_segment_chars(&pattern[1..], &text[1..])
        }
    }
}

fn match_char_class(pattern: &[char], text: &[char]) -> bool {
    if text.is_empty() {
        return false;
    }

    let Some(close_index) = pattern.iter().position(|character| *character == ']') else {
        return text[0] == '[' && match_segment_chars(&pattern[1..], &text[1..]);
    };
    if close_index == 0 {
        return text[0] == '[' && match_segment_chars(&pattern[1..], &text[1..]);
    }

    let class = &pattern[1..close_index];
    let (negated, class) = match class.first() {
        Some('!' | '^') => (true, &class[1..]),
        _ => (false, class),
    };
    let matched = class_matches(class, text[0]);
    if matched == negated {
        return false;
    }

    match_segment_chars(&pattern[close_index + 1..], &text[1..])
}

fn class_matches(class: &[char], target: char) -> bool {
    let mut index = 0;
    while index < class.len() {
        if index + 2 < class.len() && class[index + 1] == '-' {
            if class[index] <= target && target <= class[index + 2] {
                return true;
            }
            index += 3;
        } else if class[index] == target {
            return true;
        } else {
            index += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{apply_arguments, match_tasks};

    #[test]
    fn matches_glob_segments() {
        let tasks = vec![
            "build:css".to_owned(),
            "build:js:index".to_owned(),
            "lint".to_owned(),
        ];
        assert_eq!(
            match_tasks(&tasks, &["build:*".to_owned()]).unwrap(),
            ["build:css"]
        );
        assert_eq!(
            match_tasks(&tasks, &["build:**".to_owned()]).unwrap(),
            ["build:css", "build:js:index"]
        );
        assert_eq!(
            match_tasks(&tasks, &["build:[cj]*".to_owned()]).unwrap(),
            ["build:css"]
        );
    }

    #[test]
    fn keeps_pattern_order_and_removes_cross_source_duplicates() {
        let tasks = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(
            match_tasks(&tasks, &["*".to_owned(), "a".to_owned(), "*".to_owned()]).unwrap(),
            ["a", "b", "a", "b"]
        );
    }

    #[test]
    fn applies_argument_placeholders() {
        let patterns = vec!["serve -- --port {1} {@} {*} {2:-fallback}".to_owned()];
        let args = vec!["3000".to_owned(), "hello world".to_owned()];
        let applied = apply_arguments(&patterns, &args).unwrap();
        assert_eq!(
            applied,
            ["serve -- --port 3000 3000 'hello world' '3000 hello world' 'hello world'"]
        );
    }

    #[test]
    fn reports_unknown_tasks() {
        let error = match_tasks(&["build".to_owned()], &["missing".to_owned()]).unwrap_err();
        assert_eq!(error.to_string(), "Task not found: \"missing\"");
    }
}
