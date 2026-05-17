use crate::json::{parse, JsonValue};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    pub name: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageInfo {
    pub path: PathBuf,
    pub name: Option<String>,
    pub version: Option<String>,
    pub scripts: Vec<Script>,
}

impl PackageInfo {
    pub fn task_names(&self) -> Vec<String> {
        self.scripts
            .iter()
            .map(|script| script.name.clone())
            .collect()
    }

    pub fn script_body(&self, name: &str) -> Option<&str> {
        self.scripts
            .iter()
            .find_map(|script| (script.name == name).then_some(script.body.as_str()))
    }
}

#[derive(Debug)]
pub struct PackageError {
    message: String,
}

impl PackageError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PackageError {}

pub fn read_package_json() -> Result<PackageInfo, PackageError> {
    let path = std::env::current_dir()
        .map_err(|error| PackageError::new(format!("Cannot read current directory: {error}")))?
        .join("package.json");
    let contents = fs::read_to_string(&path)
        .map_err(|error| PackageError::new(format!("Cannot read {}: {error}", path.display())))?;
    let root = parse(&contents)
        .map_err(|error| PackageError::new(format!("Cannot parse {}: {error}", path.display())))?;

    let scripts = root
        .get("scripts")
        .and_then(JsonValue::as_object)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(name, value)| {
                    value.as_str().map(|body| Script {
                        name: name.clone(),
                        body: body.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(PackageInfo {
        path,
        name: root
            .get("name")
            .and_then(JsonValue::as_str)
            .map(str::to_owned),
        version: root
            .get("version")
            .and_then(JsonValue::as_str)
            .map(str::to_owned),
        scripts,
    })
}
