use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;

use crate::runtime::error::{GraphError, GraphResult};
use crate::runtime::tool::{ToolCall, ToolDefinition, ToolOutput, ToolRegistry};

#[derive(Clone, Debug)]
pub struct FileToolKit {
    root: Arc<PathBuf>,
}

impl FileToolKit {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Arc::new(root.into()),
        }
    }

    pub fn register(&self, registry: &mut ToolRegistry) {
        register_file_tools_with_root(registry, Arc::clone(&self.root));
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

pub fn register_file_tools(registry: &mut ToolRegistry, root: impl Into<PathBuf>) {
    register_file_tools_with_root(registry, Arc::new(root.into()));
}

fn register_file_tools_with_root(registry: &mut ToolRegistry, root: Arc<PathBuf>) {
    let read_root = Arc::clone(&root);
    registry.register_with_definition(
        ToolDefinition::new("read", "Read file contents")
            .with_input_schema(schema_read())
            .with_output_schema(schema_read_output()),
        Arc::new(move |call, _ctx| {
            let root = Arc::clone(&read_root);
            Box::pin(async move { handle_read(&root, call) })
        }),
    );

    let write_root = Arc::clone(&root);
    registry.register_with_definition(
        ToolDefinition::new("write", "Write file contents")
            .with_input_schema(schema_write())
            .with_output_schema(schema_write_output())
            .mark_sensitive(),
        Arc::new(move |call, _ctx| {
            let root = Arc::clone(&write_root);
            Box::pin(async move { handle_write(&root, call) })
        }),
    );

    let edit_root = Arc::clone(&root);
    registry.register_with_definition(
        ToolDefinition::new("edit", "Edit file contents")
            .with_input_schema(schema_edit())
            .with_output_schema(schema_edit_output())
            .mark_sensitive(),
        Arc::new(move |call, _ctx| {
            let root = Arc::clone(&edit_root);
            Box::pin(async move { handle_edit(&root, call) })
        }),
    );

    let search_root = Arc::clone(&root);
    registry.register_with_definition(
        ToolDefinition::new("search", "Search text across files")
            .with_input_schema(schema_search())
            .with_output_schema(schema_search_output()),
        Arc::new(move |call, _ctx| {
            let root = Arc::clone(&search_root);
            Box::pin(async move { handle_search(&root, call) })
        }),
    );

    let list_root = Arc::clone(&root);
    registry.register_with_definition(
        ToolDefinition::new("list", "List files and directories")
            .with_input_schema(schema_list())
            .with_output_schema(schema_list_output()),
        Arc::new(move |call, _ctx| {
            let root = Arc::clone(&list_root);
            Box::pin(async move { handle_list(&root, call) })
        }),
    );
}

#[derive(Deserialize)]
struct ReadInput {
    path: String,
}

#[derive(Deserialize)]
struct WriteInput {
    path: String,
    content: String,
    #[serde(default = "default_true")]
    create_dirs: bool,
}

#[derive(Deserialize)]
struct EditInput {
    path: String,
    find: String,
    replace: String,
    #[serde(default)]
    all: bool,
}

#[derive(Deserialize)]
struct SearchInput {
    query: String,
    path: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct ListInput {
    path: Option<String>,
    #[serde(default)]
    recursive: bool,
}

fn default_true() -> bool {
    true
}

fn handle_read(root: &Path, call: ToolCall) -> GraphResult<ToolOutput> {
    let input: ReadInput = parse_input("read", call.input)?;
    let path = resolve_path("read", root, &input.path)?;
    let contents = std::fs::read_to_string(&path)
        .map_err(|err| tool_error("read", format!("read failed: {}", err)))?;
    Ok(ToolOutput::new(serde_json::json!({
        "path": input.path,
        "content": contents,
    })))
}

fn handle_write(root: &Path, call: ToolCall) -> GraphResult<ToolOutput> {
    let input: WriteInput = parse_input("write", call.input)?;
    let path = resolve_path("write", root, &input.path)?;
    if let Some(parent) = path.parent() {
        if input.create_dirs {
            std::fs::create_dir_all(parent)
                .map_err(|err| tool_error("write", format!("create dir failed: {}", err)))?;
        } else if !parent.exists() {
            return Err(tool_error("write", "parent directory missing"));
        }
    }
    std::fs::write(&path, input.content.as_bytes())
        .map_err(|err| tool_error("write", format!("write failed: {}", err)))?;
    Ok(ToolOutput::new(serde_json::json!({
        "path": input.path,
        "bytes_written": input.content.len(),
    })))
}

fn handle_edit(root: &Path, call: ToolCall) -> GraphResult<ToolOutput> {
    let input: EditInput = parse_input("edit", call.input)?;
    let path = resolve_path("edit", root, &input.path)?;
    let contents = std::fs::read_to_string(&path)
        .map_err(|err| tool_error("edit", format!("read failed: {}", err)))?;

    let (updated, replaced) = if input.all {
        let replaced = contents.matches(&input.find).count();
        let updated = contents.replace(&input.find, &input.replace);
        (updated, replaced)
    } else if let Some(index) = contents.find(&input.find) {
        let mut updated = contents.clone();
        updated.replace_range(index..index + input.find.len(), &input.replace);
        (updated, 1)
    } else {
        (contents, 0)
    };

    if replaced == 0 {
        return Err(tool_error("edit", "pattern not found"));
    }

    std::fs::write(&path, updated.as_bytes())
        .map_err(|err| tool_error("edit", format!("write failed: {}", err)))?;

    Ok(ToolOutput::new(serde_json::json!({
        "path": input.path,
        "replaced": replaced,
    })))
}

fn handle_search(root: &Path, call: ToolCall) -> GraphResult<ToolOutput> {
    let input: SearchInput = parse_input("search", call.input)?;
    let base = match &input.path {
        Some(rel) => resolve_path("search", root, rel)?,
        None => root.to_path_buf(),
    };

    let mut files = Vec::new();
    collect_files(&base, &mut files)
        .map_err(|err| tool_error("search", format!("scan failed: {}", err)))?;
    files.sort();

    let mut matches = Vec::new();
    let limit = input.limit.unwrap_or(100);
    for file in files {
        let content = match std::fs::read_to_string(&file) {
            Ok(content) => content,
            Err(_) => continue,
        };
        for (index, line) in content.lines().enumerate() {
            if line.contains(&input.query) {
                matches.push(serde_json::json!({
                    "path": display_rel(root, &file),
                    "line": index + 1,
                    "text": line,
                }));
                if matches.len() >= limit {
                    break;
                }
            }
        }
        if matches.len() >= limit {
            break;
        }
    }

    Ok(ToolOutput::new(serde_json::json!({
        "matches": matches,
    })))
}

fn handle_list(root: &Path, call: ToolCall) -> GraphResult<ToolOutput> {
    let input: ListInput = parse_input("list", call.input)?;
    let base = match &input.path {
        Some(rel) => resolve_path("list", root, rel)?,
        None => root.to_path_buf(),
    };

    let mut entries = Vec::new();
    if input.recursive {
        collect_entries_recursive(root, &base, &mut entries)
            .map_err(|err| tool_error("list", format!("list failed: {}", err)))?;
    } else {
        collect_entries(root, &base, &mut entries)
            .map_err(|err| tool_error("list", format!("list failed: {}", err)))?;
    }

    entries.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));

    Ok(ToolOutput::new(serde_json::json!({
        "entries": entries,
    })))
}

fn parse_input<T: for<'de> Deserialize<'de>>(
    tool: &str,
    input: serde_json::Value,
) -> GraphResult<T> {
    serde_json::from_value(input).map_err(|err| tool_error(tool, format!("invalid input: {}", err)))
}

fn resolve_path(tool: &str, root: &Path, rel: &str) -> GraphResult<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.as_os_str().is_empty() {
        return Err(tool_error(tool, "path required"));
    }
    for component in rel_path.components() {
        match component {
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err(tool_error(tool, "path escapes workspace"));
            }
            _ => {}
        }
    }
    Ok(root.join(rel_path))
}

fn tool_error(tool: &str, message: impl Into<String>) -> GraphError {
    GraphError::ExecutionError {
        node: format!("tool:{}", tool),
        message: message.into(),
    }
}

fn display_rel(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_file() {
        files.push(dir.to_path_buf());
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_entries(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<serde_json::Value>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = if path.is_dir() { "dir" } else { "file" };
        entries.push(serde_json::json!({
            "path": display_rel(root, &path),
            "kind": kind,
        }));
    }
    Ok(())
}

fn collect_entries_recursive(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<serde_json::Value>,
) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let kind = if path.is_dir() { "dir" } else { "file" };
            entries.push(serde_json::json!({
                "path": display_rel(root, &path),
                "kind": kind,
            }));
            if path.is_dir() {
                collect_entries_recursive(root, &path, entries)?;
            }
        }
    }
    Ok(())
}

fn schema_read() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": { "path": { "type": "string" } },
        "required": ["path"]
    })
}

fn schema_read_output() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "content": { "type": "string" }
        },
        "required": ["path", "content"]
    })
}

fn schema_write() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "content": { "type": "string" },
            "create_dirs": { "type": "boolean" }
        },
        "required": ["path", "content"]
    })
}

fn schema_write_output() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "bytes_written": { "type": "number" }
        },
        "required": ["path", "bytes_written"]
    })
}

fn schema_edit() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "find": { "type": "string" },
            "replace": { "type": "string" },
            "all": { "type": "boolean" }
        },
        "required": ["path", "find", "replace"]
    })
}

fn schema_edit_output() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "replaced": { "type": "number" }
        },
        "required": ["path", "replaced"]
    })
}

fn schema_search() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" },
            "path": { "type": "string" },
            "limit": { "type": "number" }
        },
        "required": ["query"]
    })
}

fn schema_search_output() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "matches": { "type": "array" }
        },
        "required": ["matches"]
    })
}

fn schema_list() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "recursive": { "type": "boolean" }
        }
    })
}

fn schema_list_output() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "entries": { "type": "array" }
        },
        "required": ["entries"]
    })
}
