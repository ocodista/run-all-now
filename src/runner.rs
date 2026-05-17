use crate::package::PackageInfo;
use crate::shell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResult {
    pub name: String,
    pub code: Option<i32>,
}

#[derive(Debug)]
pub struct RunError {
    pub message: String,
    pub results: Vec<TaskResult>,
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RunError {}

#[derive(Clone, Debug)]
pub struct RunOptions {
    pub parallel: bool,
    pub max_parallel: usize,
    pub continue_on_error: bool,
    pub print_label: bool,
    pub print_name: bool,
    pub race: bool,
    pub aggregate_output: bool,
    pub npm_path: String,
    pub prefix_options: Vec<String>,
    pub label_width: usize,
    pub package_info: Option<PackageInfo>,
}

pub fn run_tasks(tasks: &[String], options: &RunOptions) -> Result<Vec<TaskResult>, RunError> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }
    if !options.parallel {
        return run_tasks_sequentially(tasks, options);
    }

    let max_parallel = if options.parallel {
        if options.max_parallel == 0 {
            tasks.len()
        } else {
            options.max_parallel.min(tasks.len())
        }
    } else {
        1
    };

    let mut results = tasks
        .iter()
        .map(|task| TaskResult {
            name: task.clone(),
            code: None,
        })
        .collect::<Vec<_>>();
    let mut queue = tasks
        .iter()
        .cloned()
        .enumerate()
        .collect::<VecDeque<(usize, String)>>();
    let running_processes = Arc::new(Mutex::new(Vec::<u32>::new()));
    let aborted = Arc::new(AtomicBool::new(false));
    let output_lock = Arc::new(Mutex::new(()));
    let (sender, receiver) = mpsc::channel::<WorkerResult>();
    let mut active_count = 0_usize;
    let mut first_error: Option<String> = None;
    let mut race_won = false;

    while active_count < max_parallel && !queue.is_empty() {
        spawn_next(
            &mut queue,
            &mut active_count,
            sender.clone(),
            options,
            Arc::clone(&running_processes),
            Arc::clone(&aborted),
            Arc::clone(&output_lock),
        );
    }

    while active_count > 0 {
        let worker_result = match receiver.recv() {
            Ok(worker_result) => worker_result,
            Err(_) => break,
        };
        active_count -= 1;

        if aborted.load(Ordering::SeqCst) && worker_result.result.code != Some(0) {
            if worker_result.index < results.len() && results[worker_result.index].code.is_none() {
                results[worker_result.index].name = worker_result.result.name.clone();
            }
        } else if worker_result.index < results.len() {
            results[worker_result.index] = worker_result.result.clone();
        }

        if let Some(message) = worker_result.error_message {
            if first_error.is_none() {
                first_error = Some(message);
            }
            if !options.continue_on_error || options.race {
                abort_all(&running_processes, &aborted);
            }
        } else if let Some(code) = worker_result.result.code {
            if code != 0 {
                if first_error.is_none() {
                    first_error = Some(format!(
                        "{}: script exited with code {code}",
                        worker_result.result.name
                    ));
                }
                if !options.continue_on_error {
                    abort_all(&running_processes, &aborted);
                }
            } else if options.race && options.parallel {
                race_won = true;
                abort_all(&running_processes, &aborted);
            }
        }

        while active_count < max_parallel
            && !queue.is_empty()
            && (!aborted.load(Ordering::SeqCst) || options.continue_on_error)
        {
            if aborted.load(Ordering::SeqCst) {
                break;
            }
            spawn_next(
                &mut queue,
                &mut active_count,
                sender.clone(),
                options,
                Arc::clone(&running_processes),
                Arc::clone(&aborted),
                Arc::clone(&output_lock),
            );
        }
    }

    if race_won {
        return Ok(results);
    }

    if let Some(message) = first_error {
        Err(RunError { message, results })
    } else {
        Ok(results)
    }
}

fn run_tasks_sequentially(
    tasks: &[String],
    options: &RunOptions,
) -> Result<Vec<TaskResult>, RunError> {
    let mut results = tasks
        .iter()
        .map(|task| TaskResult {
            name: task.clone(),
            code: None,
        })
        .collect::<Vec<_>>();
    let running_processes = Arc::new(Mutex::new(Vec::<u32>::new()));
    let aborted = Arc::new(AtomicBool::new(false));
    let output_lock = Arc::new(Mutex::new(()));
    let mut first_error = None;

    for (index, task) in tasks.iter().cloned().enumerate() {
        let worker_result = run_task(
            index,
            task,
            options,
            Arc::clone(&running_processes),
            Arc::clone(&aborted),
            Arc::clone(&output_lock),
        );
        let code = worker_result.result.code;
        let name = worker_result.result.name.clone();
        if index < results.len() {
            results[index] = worker_result.result;
        }

        if let Some(message) = worker_result.error_message {
            first_error.get_or_insert(message);
            if !options.continue_on_error {
                break;
            }
        } else if let Some(code) = code {
            if code != 0 {
                first_error
                    .get_or_insert_with(|| format!("{name}: script exited with code {code}"));
                if !options.continue_on_error {
                    break;
                }
            }
        }
    }

    if let Some(message) = first_error {
        Err(RunError { message, results })
    } else {
        Ok(results)
    }
}

fn spawn_next(
    queue: &mut VecDeque<(usize, String)>,
    active_count: &mut usize,
    sender: mpsc::Sender<WorkerResult>,
    options: &RunOptions,
    running_processes: Arc<Mutex<Vec<u32>>>,
    aborted: Arc<AtomicBool>,
    output_lock: Arc<Mutex<()>>,
) {
    let Some((index, task)) = queue.pop_front() else {
        return;
    };
    let worker_options = options.clone();
    *active_count += 1;
    thread::spawn(move || {
        let result = run_task(
            index,
            task,
            &worker_options,
            running_processes,
            aborted,
            output_lock,
        );
        let _ = sender.send(result);
    });
}

fn abort_all(running_processes: &Arc<Mutex<Vec<u32>>>, aborted: &Arc<AtomicBool>) {
    if aborted.swap(true, Ordering::SeqCst) {
        return;
    }

    let process_ids = running_processes
        .lock()
        .map(|processes| processes.clone())
        .unwrap_or_default();
    for process_id in process_ids {
        terminate_process_tree(process_id);
    }
}

#[derive(Debug)]
struct WorkerResult {
    index: usize,
    result: TaskResult,
    error_message: Option<String>,
}

fn run_task(
    index: usize,
    task: String,
    options: &RunOptions,
    running_processes: Arc<Mutex<Vec<u32>>>,
    aborted: Arc<AtomicBool>,
    output_lock: Arc<Mutex<()>>,
) -> WorkerResult {
    if aborted.load(Ordering::SeqCst) {
        return WorkerResult {
            index,
            result: TaskResult {
                name: task,
                code: None,
            },
            error_message: None,
        };
    }

    let mut command = match build_command(&task, options) {
        Ok(command) => command,
        Err(error) => {
            return WorkerResult {
                index,
                result: TaskResult {
                    name: task,
                    code: Some(1),
                },
                error_message: Some(error.to_string()),
            };
        }
    };

    let pipe_stdout = options.print_label || options.aggregate_output;
    let pipe_stderr = options.print_label;

    command.stdin(Stdio::inherit());
    command.stdout(if pipe_stdout {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });
    command.stderr(if pipe_stderr {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });
    configure_process_group(&mut command);

    if options.print_name {
        write_stdout(
            create_header(&task, options.package_info.as_ref()),
            &output_lock,
        );
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return WorkerResult {
                index,
                result: TaskResult {
                    name: task.clone(),
                    code: Some(1),
                },
                error_message: Some(format!("Failed to start {task}: {error}")),
            };
        }
    };

    let process_id = child.id();
    if let Ok(mut processes) = running_processes.lock() {
        processes.push(process_id);
    }

    let stdout_handle = child.stdout.take().map(|stdout| {
        let label = options
            .print_label
            .then(|| Label::new(&task, options.label_width, StreamKind::Stdout));
        read_output(
            stdout,
            label,
            options.aggregate_output,
            StreamKind::Stdout,
            Arc::clone(&output_lock),
        )
    });
    let stderr_handle = child.stderr.take().map(|stderr| {
        let label = options
            .print_label
            .then(|| Label::new(&task, options.label_width, StreamKind::Stderr));
        read_output(
            stderr,
            label,
            false,
            StreamKind::Stderr,
            Arc::clone(&output_lock),
        )
    });

    let wait_result = wait_for_child(&mut child);

    if let Ok(mut processes) = running_processes.lock() {
        processes.retain(|candidate| *candidate != process_id);
    }

    if let Some(handle) = stdout_handle {
        if let Ok(buffer) = handle.join() {
            if options.aggregate_output && !buffer.is_empty() && !aborted.load(Ordering::SeqCst) {
                write_stdout_bytes(&buffer, &output_lock);
            }
        }
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }

    match wait_result {
        Ok(code) => WorkerResult {
            index,
            result: TaskResult {
                name: task,
                code: Some(code),
            },
            error_message: None,
        },
        Err(error) => WorkerResult {
            index,
            result: TaskResult {
                name: task.clone(),
                code: Some(1),
            },
            error_message: Some(format!("Failed to wait for {task}: {error}")),
        },
    }
}

fn build_command(task: &str, options: &RunOptions) -> Result<Command, String> {
    let npm_path = options.npm_path.as_str();
    let npm_path_is_js = Path::new(npm_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "js" | "mjs" | "cjs"));
    let executable = if npm_path_is_js {
        std::env::var_os("RUN_ALL_NOW_NODE_PATH").unwrap_or_else(|| OsString::from("node"))
    } else {
        OsString::from(npm_path)
    };

    let mut command = Command::new(executable);
    if npm_path_is_js {
        command.arg(npm_path);
    }

    command.arg("run");
    let is_yarn = Path::new(npm_path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| file_name.starts_with("yarn"));

    if is_yarn {
        if options
            .prefix_options
            .iter()
            .any(|option| option == "--silent")
        {
            command.arg("--silent");
        }
    } else {
        command.args(&options.prefix_options);
    }

    for word in shell::split_words(task).map_err(|error| error.to_string())? {
        command.arg(word);
    }

    Ok(command)
}

fn wait_for_child(child: &mut Child) -> io::Result<i32> {
    child.wait().map(|status| status.code().unwrap_or(1))
}

fn create_header(task: &str, package_info: Option<&PackageInfo>) -> String {
    let Some(package_info) = package_info else {
        return format!("\n> {task}\n\n");
    };

    let (script_name, args) = task
        .split_once(' ')
        .map_or((task, ""), |(name, args)| (name, args));
    let Some(script_body) = package_info.script_body(script_name) else {
        return format!("\n> {task}\n\n");
    };

    let package_name = package_info.name.as_deref().unwrap_or("");
    let package_version = package_info.version.as_deref().unwrap_or("");
    if args.is_empty() {
        format!(
            "\n> {package_name}@{package_version} {script_name} {}\n> {script_body}\n\n",
            package_info.path.display()
        )
    } else {
        format!(
            "\n> {package_name}@{package_version} {script_name} {}\n> {script_body} {args}\n\n",
            package_info.path.display()
        )
    }
}

#[derive(Clone, Copy, Debug)]
enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
struct Label {
    prefix: Vec<u8>,
}

impl Label {
    fn new(task: &str, width: usize, stream_kind: StreamKind) -> Self {
        let padded = format!("{task:<width$}");
        let prefix = if should_color(stream_kind) {
            format!("{}[{}]{} ", color_for_task(task), padded, "\x1b[0m").into_bytes()
        } else {
            format!("[{padded}] ").into_bytes()
        };
        Self { prefix }
    }
}

fn color_for_task(task: &str) -> &'static str {
    const COLORS: [&str; 5] = ["\x1b[36m", "\x1b[32m", "\x1b[35m", "\x1b[33m", "\x1b[31m"];
    let hash = task.bytes().fold(0_usize, |accumulator, byte| {
        accumulator.wrapping_add(usize::from(byte))
    });
    COLORS[hash % COLORS.len()]
}

fn should_color(stream_kind: StreamKind) -> bool {
    match stream_kind {
        StreamKind::Stdout => io::stdout().is_terminal(),
        StreamKind::Stderr => io::stderr().is_terminal(),
    }
}

fn read_output<R: Read + Send + 'static>(
    mut reader: R,
    label: Option<Label>,
    aggregate: bool,
    stream_kind: StreamKind,
    output_lock: Arc<Mutex<()>>,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut source = [0_u8; 8192];
        let mut at_line_start = true;
        let mut aggregate_buffer = Vec::new();

        loop {
            let bytes_read = match reader.read(&mut source) {
                Ok(0) => break,
                Ok(bytes_read) => bytes_read,
                Err(_) => break,
            };
            let transformed =
                apply_label(&source[..bytes_read], label.as_ref(), &mut at_line_start);
            if aggregate {
                aggregate_buffer.extend_from_slice(&transformed);
            } else {
                write_stream_bytes(&transformed, stream_kind, &output_lock);
            }
        }

        aggregate_buffer
    })
}

fn apply_label(chunk: &[u8], label: Option<&Label>, at_line_start: &mut bool) -> Vec<u8> {
    let Some(label) = label else {
        return chunk.to_vec();
    };

    let mut output = Vec::with_capacity(chunk.len() + label.prefix.len());
    for byte in chunk {
        if *at_line_start {
            output.extend_from_slice(&label.prefix);
            *at_line_start = false;
        }
        output.push(*byte);
        if *byte == b'\n' {
            *at_line_start = true;
        }
    }
    output
}

fn write_stdout(text: String, output_lock: &Arc<Mutex<()>>) {
    write_stdout_bytes(text.as_bytes(), output_lock);
}

fn write_stdout_bytes(bytes: &[u8], output_lock: &Arc<Mutex<()>>) {
    write_stream_bytes(bytes, StreamKind::Stdout, output_lock);
}

fn write_stream_bytes(bytes: &[u8], stream_kind: StreamKind, output_lock: &Arc<Mutex<()>>) {
    let _guard = output_lock.lock().ok();
    match stream_kind {
        StreamKind::Stdout => {
            let mut stdout = io::stdout().lock();
            let _ = stdout.write_all(bytes);
            let _ = stdout.flush();
        }
        StreamKind::Stderr => {
            let mut stderr = io::stderr().lock();
            let _ = stderr.write_all(bytes);
            let _ = stderr.flush();
        }
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_process_tree(process_id: u32) {
    let group = format!("-{}", process_id);
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(&group)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(windows)]
fn terminate_process_tree(process_id: u32) {
    let _ = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &process_id.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(any(unix, windows)))]
fn terminate_process_tree(_process_id: u32) {}

#[cfg(test)]
mod tests {
    use super::{create_header, TaskResult};
    use crate::package::{PackageInfo, Script};
    use std::path::PathBuf;

    #[test]
    fn creates_npm_like_header() {
        let package = PackageInfo {
            path: PathBuf::from("/repo/package.json"),
            name: Some("demo".to_owned()),
            version: Some("1.0.0".to_owned()),
            scripts: vec![Script {
                name: "build".to_owned(),
                body: "node build.js".to_owned(),
            }],
        };
        assert_eq!(
            create_header("build -- --watch", Some(&package)),
            "\n> demo@1.0.0 build /repo/package.json\n> node build.js -- --watch\n\n"
        );
    }

    #[test]
    fn task_result_can_hold_unstarted_task() {
        let result = TaskResult {
            name: "build".to_owned(),
            code: None,
        };
        assert_eq!(result.code, None);
    }
}
