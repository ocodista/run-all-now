use crate::glob::{apply_arguments, match_tasks};
use crate::json::{parse as parse_json, quote_string, JsonValue};
use crate::package::{read_package_json, PackageInfo};
use crate::runner::{run_tasks, RunOptions, TaskResult};
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CliError {
    pub message: String,
    pub exit_code: u8,
    pub silent: bool,
}

impl CliError {
    fn new(message: impl Into<String>, silent: bool) -> Self {
        Self {
            message: message.into(),
            exit_code: 1,
            silent,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

pub fn run_from_env() -> Result<u8, CliError> {
    let raw_args = env::args().collect::<Vec<_>>();
    if raw_args
        .get(1)
        .is_some_and(|arg| arg == "--run-all-now-api")
    {
        let Some(path) = raw_args.get(2) else {
            return Err(CliError::new("Missing API options path", true));
        };
        return run_api(path);
    }

    let bin_name = env::var("RUN_ALL_NOW_BIN_NAME")
        .ok()
        .or_else(|| raw_args.first().and_then(|arg| executable_name(arg)))
        .unwrap_or_else(|| "npm-run-all".to_owned());
    let command_kind = CommandKind::from_bin_name(&bin_name);
    let args = raw_args.into_iter().skip(1).collect::<Vec<_>>();

    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        print_help(command_kind);
        return Ok(0);
    }
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--version" | "-v"))
    {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }

    let argument_set = parse_cli_args(&args, command_kind)?;
    execute_groups(&argument_set, None)
        .map(|_| 0)
        .map_err(|error| CliError {
            message: error.message,
            exit_code: 1,
            silent: error.silent,
        })
}

fn run_api(path: &str) -> Result<u8, CliError> {
    let contents = fs::read_to_string(path)
        .map_err(|error| CliError::new(format!("Cannot read API options {path}: {error}"), true))?;
    let value = parse_json(&contents).map_err(|error| {
        CliError::new(format!("Cannot parse API options {path}: {error}"), true)
    })?;
    let request = ApiRequest::from_json(&value)?;
    let mut argument_set = ArgumentSet::new(CommandKind::NpmRunAll);
    argument_set.continue_on_error = request.continue_on_error;
    argument_set.max_parallel = request.max_parallel;
    argument_set.npm_path = request.npm_path;
    argument_set.print_label = request.print_label;
    argument_set.print_name = request.print_name;
    argument_set.race = request.race;
    argument_set.silent = request.silent;
    argument_set.aggregate_output = request.aggregate_output;
    argument_set.rest = request.arguments;
    argument_set.config = request.config;
    argument_set.package_config = request.package_config;
    argument_set.groups = vec![Group {
        parallel: request.parallel,
        patterns: request.patterns,
    }];

    match execute_groups(&argument_set, request.task_list) {
        Ok(results) => {
            write_api_result(&results, None)?;
            Ok(0)
        }
        Err(error) => {
            write_api_result(&error.results, Some(&error.message))?;
            Err(CliError {
                message: error.message,
                exit_code: 1,
                silent: true,
            })
        }
    }
}

fn write_api_result(results: &[TaskResult], error: Option<&str>) -> Result<(), CliError> {
    let Some(path) = env::var("RUN_ALL_NOW_RESULT_FILE").ok() else {
        return Ok(());
    };
    let mut json = String::from("{\"results\":[");
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("{\"name\":");
        json.push_str(&quote_string(&result.name));
        json.push_str(",\"code\":");
        match result.code {
            Some(code) => json.push_str(&code.to_string()),
            None => json.push_str("null"),
        }
        json.push('}');
    }
    json.push_str("],\"error\":");
    if let Some(error) = error {
        json.push_str(&quote_string(error));
    } else {
        json.push_str("null");
    }
    json.push('}');
    fs::write(&path, json)
        .map_err(|error| CliError::new(format!("Cannot write API result {path}: {error}"), true))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    NpmRunAll,
    RunS,
    RunP,
}

impl CommandKind {
    fn from_bin_name(bin_name: &str) -> Self {
        match bin_name {
            "run-s" => Self::RunS,
            "run-p" => Self::RunP,
            _ => Self::NpmRunAll,
        }
    }

    fn is_single_mode(self) -> bool {
        matches!(self, Self::RunS | Self::RunP)
    }

    fn initial_parallel(self) -> bool {
        matches!(self, Self::RunP)
    }
}

#[derive(Clone, Debug)]
struct ArgumentSet {
    config: BTreeMap<String, String>,
    continue_on_error: bool,
    groups: Vec<Group>,
    max_parallel: usize,
    npm_path: Option<String>,
    package_config: BTreeMap<String, BTreeMap<String, String>>,
    print_label: bool,
    print_name: bool,
    race: bool,
    rest: Vec<String>,
    silent: bool,
    aggregate_output: bool,
}

impl ArgumentSet {
    fn new(command_kind: CommandKind) -> Self {
        Self {
            config: BTreeMap::new(),
            continue_on_error: false,
            groups: vec![Group {
                parallel: command_kind.initial_parallel(),
                patterns: Vec::new(),
            }],
            max_parallel: 0,
            npm_path: None,
            package_config: create_package_config(),
            print_label: false,
            print_name: false,
            race: false,
            rest: Vec::new(),
            silent: env::var("npm_config_loglevel").is_ok_and(|level| level == "silent"),
            aggregate_output: false,
        }
    }

    fn last_group_mut(&mut self) -> &mut Group {
        self.groups
            .last_mut()
            .expect("argument set always contains a group")
    }

    fn has_parallel_group(&self) -> bool {
        self.groups.iter().any(|group| group.parallel)
    }
}

#[derive(Clone, Debug)]
struct Group {
    parallel: bool,
    patterns: Vec<String>,
}

fn parse_cli_args(args: &[String], command_kind: CommandKind) -> Result<ArgumentSet, CliError> {
    let mut set = ArgumentSet::new(command_kind);
    parse_cli_args_core(&mut set, args, command_kind)?;

    if !set.has_parallel_group() && set.aggregate_output {
        return Err(CliError::new(
            "Invalid Option: --aggregate-output (without parallel)",
            set.silent,
        ));
    }
    if !set.has_parallel_group() && set.race {
        return Err(CliError::new(
            "Invalid Option: --race (without parallel)",
            set.silent,
        ));
    }
    if !set.has_parallel_group() && set.max_parallel != 0 {
        return Err(CliError::new(
            "Invalid Option: --max-parallel (without parallel)",
            set.silent,
        ));
    }

    Ok(set)
}

fn parse_cli_args_core(
    set: &mut ArgumentSet,
    args: &[String],
    command_kind: CommandKind,
) -> Result<(), CliError> {
    let mut index = 0_usize;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--" => {
                set.rest = args[index + 1..].to_vec();
                break;
            }
            "--color" | "--no-color" => {}
            "-c" | "--continue-on-error" => set.continue_on_error = true,
            "-l" | "--print-label" => set.print_label = true,
            "-n" | "--print-name" => set.print_name = true,
            "-r" | "--race" => set.race = true,
            "--silent" => set.silent = true,
            "--max-parallel" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(CliError::new("Invalid Option: --max-parallel", set.silent));
                };
                set.max_parallel = value.parse::<usize>().map_err(|_| {
                    CliError::new(
                        format!("Invalid Option: --max-parallel {value}"),
                        set.silent,
                    )
                })?;
                if set.max_parallel == 0 {
                    return Err(CliError::new(
                        format!("Invalid Option: --max-parallel {value}"),
                        set.silent,
                    ));
                }
            }
            "-s" | "--sequential" | "--serial" => {
                if command_kind.is_single_mode() && arg == "-s" {
                    set.silent = true;
                } else if command_kind.is_single_mode() {
                    return Err(CliError::new(format!("Invalid Option: {arg}"), set.silent));
                } else {
                    set.groups.push(Group {
                        parallel: false,
                        patterns: Vec::new(),
                    });
                }
            }
            "--aggregate-output" => set.aggregate_output = true,
            "-p" | "--parallel" => {
                if command_kind.is_single_mode() {
                    return Err(CliError::new(format!("Invalid Option: {arg}"), set.silent));
                }
                set.groups.push(Group {
                    parallel: true,
                    patterns: Vec::new(),
                });
            }
            "--npm-path" => {
                index += 1;
                set.npm_path = args.get(index).cloned();
            }
            _ if is_concat_options(arg) => {
                let expanded = arg
                    .chars()
                    .skip(1)
                    .map(|option| format!("-{option}"))
                    .collect::<Vec<_>>();
                parse_cli_args_core(set, &expanded, command_kind)?;
            }
            _ if arg.starts_with('-') => parse_unknown_option(set, args, &mut index)?,
            _ => set.last_group_mut().patterns.push(arg.clone()),
        }
        index += 1;
    }

    Ok(())
}

fn parse_unknown_option(
    set: &mut ArgumentSet,
    args: &[String],
    index: &mut usize,
) -> Result<(), CliError> {
    let arg = &args[*index];
    if let Some(option) = arg.strip_prefix("--") {
        if let Some((scope, rest)) = option.split_once(':') {
            if !scope.is_empty() {
                let (variable, value) = if let Some((variable, value)) = rest.split_once('=') {
                    (variable, value.to_owned())
                } else {
                    *index += 1;
                    let value = args.get(*index).cloned().ok_or_else(|| {
                        CliError::new(format!("Invalid Option: {arg}"), set.silent)
                    })?;
                    (rest, value)
                };
                set.package_config
                    .entry(scope.to_owned())
                    .or_default()
                    .insert(variable.to_owned(), value);
                return Ok(());
            }
        }

        if let Some((key, value)) = option.split_once('=') {
            if !key.is_empty() {
                set.config.insert(key.to_owned(), value.to_owned());
                return Ok(());
            }
        }
    }

    Err(CliError::new(format!("Invalid Option: {arg}"), set.silent))
}

fn is_concat_options(arg: &str) -> bool {
    let Some(rest) = arg.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty()
        && rest
            .bytes()
            .all(|byte| matches!(byte, b'c' | b'l' | b'n' | b'p' | b'r' | b's'))
}

#[derive(Debug)]
struct ExecutionError {
    message: String,
    silent: bool,
    results: Vec<TaskResult>,
}

fn execute_groups(
    argument_set: &ArgumentSet,
    task_list_override: Option<Vec<String>>,
) -> Result<Vec<TaskResult>, ExecutionError> {
    let mut all_results = Vec::new();
    let package_info = if task_list_override.is_none() {
        Some(read_package_json().map_err(|error| ExecutionError {
            message: error.to_string(),
            silent: argument_set.silent,
            results: all_results.clone(),
        })?)
    } else {
        None
    };
    let task_list = task_list_override.unwrap_or_else(|| {
        package_info
            .as_ref()
            .map(PackageInfo::task_names)
            .unwrap_or_default()
    });
    let prefix_options = prefix_options(argument_set);
    let npm_path = resolve_npm_path(argument_set.npm_path.as_deref());

    for group in &argument_set.groups {
        if group.patterns.is_empty() {
            continue;
        }
        let patterns = apply_arguments(&group.patterns, &argument_set.rest).map_err(|error| {
            ExecutionError {
                message: error.to_string(),
                silent: argument_set.silent,
                results: all_results.clone(),
            }
        })?;
        let tasks = match_tasks(&task_list, &patterns).map_err(|error| ExecutionError {
            message: error.to_string(),
            silent: argument_set.silent,
            results: all_results.clone(),
        })?;
        let label_width = tasks.iter().map(String::len).max().unwrap_or(0);
        let run_options = RunOptions {
            parallel: group.parallel,
            max_parallel: if group.parallel {
                argument_set.max_parallel
            } else {
                1
            },
            continue_on_error: argument_set.continue_on_error,
            print_label: argument_set.print_label,
            print_name: argument_set.print_name,
            race: group.parallel && argument_set.race,
            aggregate_output: group.parallel && argument_set.aggregate_output,
            npm_path: npm_path.clone(),
            prefix_options: prefix_options.clone(),
            label_width,
            package_info: package_info.clone(),
        };

        match run_tasks(&tasks, &run_options) {
            Ok(results) => all_results.extend(results),
            Err(error) => {
                all_results.extend(error.results.clone());
                return Err(ExecutionError {
                    message: error.message,
                    silent: argument_set.silent,
                    results: all_results,
                });
            }
        }
    }

    Ok(all_results)
}

fn resolve_npm_path(configured_npm_path: Option<&str>) -> String {
    configured_npm_path
        .map(str::to_owned)
        .or_else(|| std::env::var("npm_execpath").ok())
        .or_else(find_npm_cli_from_node)
        .unwrap_or_else(|| "npm".to_owned())
}

fn find_npm_cli_from_node() -> Option<String> {
    let node_path = std::env::var_os("RUN_ALL_NOW_NODE_PATH")
        .map(PathBuf::from)
        .or_else(|| find_program_in_path("node"))?;
    let node_dir = node_path.parent()?;
    let candidates = [
        node_dir.join("node_modules/npm/bin/npm-cli.js"),
        node_dir.join("../lib/node_modules/npm/bin/npm-cli.js"),
    ];

    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
}

fn find_program_in_path(program: &str) -> Option<PathBuf> {
    let program_path = Path::new(program);
    if program_path.components().count() > 1 && program_path.is_file() {
        return Some(program_path.to_path_buf());
    }

    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }

        #[cfg(windows)]
        {
            for extension in ["exe", "cmd", "bat"] {
                let candidate = directory.join(format!("{program}.{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn prefix_options(argument_set: &ArgumentSet) -> Vec<String> {
    let mut options = Vec::new();
    if argument_set.silent {
        options.push("--silent".to_owned());
    }
    for (package_name, variables) in &argument_set.package_config {
        for (variable_name, value) in variables {
            options.push(format!("--{package_name}:{variable_name}={value}"));
        }
    }
    for (key, value) in &argument_set.config {
        options.push(format!("--{key}={value}"));
    }
    options
}

fn create_package_config() -> BTreeMap<String, BTreeMap<String, String>> {
    let mut config = BTreeMap::<String, BTreeMap<String, String>>::new();
    let Ok(package_name) = env::var("npm_package_name") else {
        return config;
    };

    for (key, value) in env::vars() {
        if let Some(variable) = key.strip_prefix("npm_package_config_") {
            config
                .entry(package_name.clone())
                .or_default()
                .insert(variable.to_owned(), value);
        }
    }

    config
}

#[derive(Debug)]
struct ApiRequest {
    patterns: Vec<String>,
    arguments: Vec<String>,
    parallel: bool,
    max_parallel: usize,
    continue_on_error: bool,
    print_label: bool,
    print_name: bool,
    race: bool,
    silent: bool,
    aggregate_output: bool,
    npm_path: Option<String>,
    task_list: Option<Vec<String>>,
    config: BTreeMap<String, String>,
    package_config: BTreeMap<String, BTreeMap<String, String>>,
}

impl ApiRequest {
    fn from_json(value: &JsonValue) -> Result<Self, CliError> {
        Ok(Self {
            patterns: string_array(field(value, "patterns")?)?,
            arguments: optional_string_array(value.get("arguments"))?.unwrap_or_default(),
            parallel: optional_bool(value.get("parallel")).unwrap_or(false),
            max_parallel: optional_usize(value.get("maxParallel"))?.unwrap_or(0),
            continue_on_error: optional_bool(value.get("continueOnError")).unwrap_or(false),
            print_label: optional_bool(value.get("printLabel")).unwrap_or(false),
            print_name: optional_bool(value.get("printName")).unwrap_or(false),
            race: optional_bool(value.get("race")).unwrap_or(false),
            silent: optional_bool(value.get("silent")).unwrap_or(false),
            aggregate_output: optional_bool(value.get("aggregateOutput")).unwrap_or(false),
            npm_path: value
                .get("npmPath")
                .and_then(JsonValue::as_str)
                .map(str::to_owned),
            task_list: optional_string_array(value.get("taskList"))?,
            config: object_to_string_map(value.get("config"))?,
            package_config: package_config_from_json(value.get("packageConfig"))?,
        })
    }
}

fn field<'a>(value: &'a JsonValue, key: &str) -> Result<&'a JsonValue, CliError> {
    value
        .get(key)
        .ok_or_else(|| CliError::new(format!("Missing API field: {key}"), true))
}

fn string_array(value: &JsonValue) -> Result<Vec<String>, CliError> {
    value
        .as_array()
        .ok_or_else(|| CliError::new("Expected a string array", true))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| CliError::new("Expected a string array", true))
        })
        .collect()
}

fn optional_string_array(value: Option<&JsonValue>) -> Result<Option<Vec<String>>, CliError> {
    match value {
        Some(JsonValue::Null) | None => Ok(None),
        Some(value) => string_array(value).map(Some),
    }
}

fn optional_bool(value: Option<&JsonValue>) -> Option<bool> {
    match value {
        Some(JsonValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn optional_usize(value: Option<&JsonValue>) -> Result<Option<usize>, CliError> {
    match value {
        Some(JsonValue::Number(value)) => value
            .parse::<usize>()
            .map(Some)
            .map_err(|_| CliError::new("Expected a positive integer", true)),
        Some(JsonValue::Null) | None => Ok(None),
        _ => Err(CliError::new("Expected a positive integer", true)),
    }
}

fn object_to_string_map(value: Option<&JsonValue>) -> Result<BTreeMap<String, String>, CliError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(BTreeMap::new());
    }

    value
        .as_object()
        .ok_or_else(|| CliError::new("Expected an object", true))?
        .iter()
        .map(|(key, value)| Ok((key.clone(), json_to_config_value(value))))
        .collect()
}

fn package_config_from_json(
    value: Option<&JsonValue>,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, CliError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    if matches!(value, JsonValue::Null) {
        return Ok(BTreeMap::new());
    }

    value
        .as_object()
        .ok_or_else(|| CliError::new("Expected packageConfig to be an object", true))?
        .iter()
        .map(|(package_name, value)| {
            let variables = value
                .as_object()
                .ok_or_else(|| CliError::new("Expected packageConfig values to be objects", true))?
                .iter()
                .map(|(key, value)| (key.clone(), json_to_config_value(value)))
                .collect();
            Ok((package_name.clone(), variables))
        })
        .collect()
}

fn json_to_config_value(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_owned(),
        JsonValue::Bool(value) => value.to_string(),
        JsonValue::Number(value) | JsonValue::String(value) => value.clone(),
        JsonValue::Array(_) | JsonValue::Object(_) => "[object Object]".to_owned(),
    }
}

fn executable_name(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

fn print_help(command_kind: CommandKind) {
    match command_kind {
        CommandKind::RunS => println!("{}", RUN_S_HELP),
        CommandKind::RunP => println!("{}", RUN_P_HELP),
        CommandKind::NpmRunAll => println!("{}", NPM_RUN_ALL_HELP),
    }
}

const NPM_RUN_ALL_HELP: &str = r"Usage:
    $ npm-run-all [--help | -h | --version | -v]
    $ npm-run-all [tasks] [OPTIONS]

    Run given npm-scripts in parallel or sequential.

Options:
    --aggregate-output       Avoid interleaving parallel stdout.
    -c, --continue-on-error  Continue after task failures, then exit non-zero.
    --max-parallel <number>  Limit parallel tasks. Default is unlimited.
    --npm-path <string>      Set npm/yarn path. Defaults to npm_execpath or npm.
    -l, --print-label        Prefix each output line with the task name.
    -n, --print-name         Print each task header before running it.
    -p, --parallel <tasks>   Run a group of tasks in parallel.
    -r, --race               Stop when one parallel task succeeds.
    -s, --sequential <tasks> Run a group of tasks sequentially.
        --serial <tasks>     Alias of --sequential.
    --silent                 Set npm loglevel to silent.
";

const RUN_S_HELP: &str = r"Usage:
    $ run-s [--help | -h | --version | -v]
    $ run-s [OPTIONS] <tasks>

    Run given npm-scripts sequentially.

Options:
    -c, --continue-on-error  Continue after task failures, then exit non-zero.
    --npm-path <string>      Set npm/yarn path. Defaults to npm_execpath or npm.
    -l, --print-label        Prefix each output line with the task name.
    -n, --print-name         Print each task header before running it.
    -s, --silent             Set npm loglevel to silent.
";

const RUN_P_HELP: &str = r"Usage:
    $ run-p [--help | -h | --version | -v]
    $ run-p [OPTIONS] <tasks>

    Run given npm-scripts in parallel.

Options:
    --aggregate-output       Avoid interleaving stdout until a task completes.
    -c, --continue-on-error  Continue after task failures, then exit non-zero.
    --max-parallel <number>  Limit parallel tasks. Default is unlimited.
    --npm-path <string>      Set npm/yarn path. Defaults to npm_execpath or npm.
    -l, --print-label        Prefix each output line with the task name.
    -n, --print-name         Print each task header before running it.
    -r, --race               Stop when one parallel task succeeds.
    -s, --silent             Set npm loglevel to silent.
";

#[cfg(test)]
mod tests {
    use super::{parse_cli_args, prefix_options, CommandKind};

    #[test]
    fn parses_mixed_groups() {
        let args = vec![
            "clean".to_owned(),
            "-p".to_owned(),
            "build:*".to_owned(),
            "--".to_owned(),
            "3000".to_owned(),
        ];
        let parsed = parse_cli_args(&args, CommandKind::NpmRunAll).unwrap();
        assert_eq!(parsed.groups.len(), 2);
        assert_eq!(parsed.groups[0].patterns, ["clean"]);
        assert!(parsed.groups[1].parallel);
        assert_eq!(parsed.rest, ["3000"]);
    }

    #[test]
    fn parses_run_s_s_as_silent() {
        let args = vec!["-s".to_owned(), "build".to_owned()];
        let parsed = parse_cli_args(&args, CommandKind::RunS).unwrap();
        assert!(parsed.silent);
        assert_eq!(parsed.groups[0].patterns, ["build"]);
    }

    #[test]
    fn parses_package_config_and_config_options() {
        let args = vec![
            "--pkg:port=3000".to_owned(),
            "--loglevel=warn".to_owned(),
            "build".to_owned(),
        ];
        let parsed = parse_cli_args(&args, CommandKind::NpmRunAll).unwrap();
        let options = prefix_options(&parsed);
        assert!(options.contains(&"--pkg:port=3000".to_owned()));
        assert!(options.contains(&"--loglevel=warn".to_owned()));
    }
}
