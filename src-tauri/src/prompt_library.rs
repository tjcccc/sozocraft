use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet, HashSet},
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptListItem {
    pub id: String,
    pub path: String,
    pub name: String,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDocument {
    pub item: PromptListItem,
    pub source: String,
    pub rendered_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePromptRequest {
    pub id: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePromptMetadataRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePromptRequest {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPromptResult {
    pub rendered_prompt: String,
}

#[derive(Debug, Clone, Default)]
struct Frontmatter {
    name: Option<String>,
    tags: Vec<String>,
    description: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    schema_version: Option<i64>,
}

pub fn rescan_prompt_directory(prompt_directory: &str) -> Result<Vec<PromptListItem>, String> {
    let root = prompt_root(prompt_directory);
    fs::create_dir_all(&root).map_err(|err| format!("Failed to create prompt directory: {err}"))?;
    let conn = open_db()?;
    init_db(&conn)?;

    let mut seen = BTreeSet::new();
    for path in collect_markdown_files(&root).map_err(|err| err.to_string())? {
        let item = index_file(&conn, &root, &path)?;
        seen.insert(item.id);
    }

    let mut stmt = conn
        .prepare("SELECT id FROM prompts WHERE root = ?1")
        .map_err(|err| err.to_string())?;
    let ids = stmt
        .query_map(params![root.to_string_lossy().to_string()], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;
    for id in ids {
        if !seen.contains(&id) {
            conn.execute("UPDATE prompts SET missing = 1 WHERE id = ?1", params![id])
                .map_err(|err| err.to_string())?;
        }
    }

    list_prompts(prompt_directory, None)
}

pub fn list_prompts(
    prompt_directory: &str,
    query: Option<String>,
) -> Result<Vec<PromptListItem>, String> {
    let root = prompt_root(prompt_directory);
    let conn = open_db()?;
    init_db(&conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, path, name, tags, description, created_at, updated_at
             FROM prompts
             WHERE root = ?1 AND missing = 0
             ORDER BY updated_at DESC, name ASC",
        )
        .map_err(|err| err.to_string())?;
    let mut items = stmt
        .query_map(params![root.to_string_lossy().to_string()], row_to_item)
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    let needle = query.unwrap_or_default().trim().to_ascii_lowercase();
    if !needle.is_empty() {
        items.retain(|item| {
            item.name.to_ascii_lowercase().contains(&needle)
                || item.description.to_ascii_lowercase().contains(&needle)
                || item
                    .tags
                    .iter()
                    .any(|tag| tag.to_ascii_lowercase().contains(&needle))
        });
    }
    Ok(items)
}

pub fn create_prompt(
    prompt_directory: &str,
    request: CreatePromptRequest,
) -> Result<PromptDocument, String> {
    let root = prompt_root(prompt_directory);
    fs::create_dir_all(&root).map_err(|err| format!("Failed to create prompt directory: {err}"))?;
    let conn = open_db()?;
    init_db(&conn)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let name = normalize_prompt_name(&request.name);
    let tags = normalize_tags(request.tags);
    let source = String::new();
    let path = root.join(format!("{id}.md"));
    fs::write(&path, &source).map_err(|err| format!("Failed to create prompt file: {err}"))?;

    conn.execute(
        "INSERT INTO prompts (
            id, root, path, name, tags, description, created_at, updated_at,
            content_hash, file_mtime, schema_version, last_indexed_at, missing
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)",
        params![
            id,
            root.to_string_lossy().to_string(),
            path.to_string_lossy().to_string(),
            name,
            serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
            normalize_optional_text(&request.description).unwrap_or_default(),
            now.clone(),
            now,
            content_hash(&source),
            file_mtime(&path),
            SCHEMA_VERSION,
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|err| err.to_string())?;

    let item = index_file(&conn, &root, &path)?;
    Ok(PromptDocument {
        rendered_prompt: render_prompt_source_with_library(
            &source,
            prompt_directory,
            Some(&item.id),
        )
        .rendered_prompt,
        source,
        item,
    })
}

pub fn read_prompt(prompt_directory: &str, id: &str) -> Result<PromptDocument, String> {
    let root = prompt_root(prompt_directory);
    let conn = open_db()?;
    init_db(&conn)?;
    let path = path_for_id(&conn, &root, id)?;
    let raw_source =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read prompt: {err}"))?;
    let source = prompt_body(&raw_source).to_string();
    let item = index_file(&conn, &root, &path)?;
    Ok(PromptDocument {
        rendered_prompt: render_prompt_source_with_library(&source, prompt_directory, Some(id))
            .rendered_prompt,
        source,
        item,
    })
}

pub fn save_prompt(
    prompt_directory: &str,
    request: SavePromptRequest,
) -> Result<PromptDocument, String> {
    let root = prompt_root(prompt_directory);
    let conn = open_db()?;
    init_db(&conn)?;
    let path = path_for_id(&conn, &root, &request.id)?;
    let source = prompt_body(&request.source).to_string();
    fs::write(&path, &source).map_err(|err| format!("Failed to save prompt: {err}"))?;

    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE prompts
         SET content_hash = ?1, file_mtime = ?2, updated_at = ?3, last_indexed_at = ?4, missing = 0
         WHERE id = ?5 AND root = ?6",
        params![
            content_hash(&source),
            file_mtime(&path),
            now.clone(),
            now,
            request.id,
            root.to_string_lossy().to_string(),
        ],
    )
    .map_err(|err| err.to_string())?;
    let item = get_item(&conn, &root, &request.id)?;
    Ok(PromptDocument {
        rendered_prompt: render_prompt_source_with_library(
            &source,
            prompt_directory,
            Some(&request.id),
        )
        .rendered_prompt,
        source,
        item,
    })
}

pub fn update_prompt_metadata(
    prompt_directory: &str,
    request: UpdatePromptMetadataRequest,
) -> Result<PromptListItem, String> {
    let root = prompt_root(prompt_directory);
    let conn = open_db()?;
    init_db(&conn)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE prompts
         SET name = ?1, tags = ?2, updated_at = ?3, last_indexed_at = ?4
         WHERE id = ?5 AND root = ?6 AND missing = 0",
        params![
            normalize_prompt_name(&request.name),
            serde_json::to_string(&normalize_tags(request.tags))
                .unwrap_or_else(|_| "[]".to_string()),
            now.clone(),
            now,
            request.id,
            root.to_string_lossy().to_string(),
        ],
    )
    .map_err(|err| err.to_string())?;
    get_item(&conn, &root, &request.id)
}

pub fn delete_prompt(prompt_directory: &str, id: &str) -> Result<(), String> {
    let root = prompt_root(prompt_directory);
    let conn = open_db()?;
    init_db(&conn)?;
    let path = path_for_id(&conn, &root, id)?;
    fs::remove_file(&path).map_err(|err| format!("Failed to delete prompt file: {err}"))?;
    conn.execute("DELETE FROM prompts WHERE id = ?1", params![id])
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub fn render_prompt_source(source: &str) -> RenderPromptResult {
    RenderPromptResult {
        rendered_prompt: render_source_body(source),
    }
}

pub fn render_prompt_source_with_library(
    source: &str,
    prompt_directory: &str,
    current_prompt_id: Option<&str>,
) -> RenderPromptResult {
    let root = prompt_root(prompt_directory);
    let mut visited = current_prompt_id
        .map(|id| HashSet::from([id.to_string()]))
        .unwrap_or_default();
    let rendered = match open_db().and_then(|conn| {
        init_db(&conn)?;
        resolve_prompt_includes(
            &conn,
            &root,
            &render_source_body(source),
            current_prompt_id,
            &mut visited,
            0,
        )
    }) {
        Ok(value) => value,
        Err(_) => render_source_body(source),
    };
    RenderPromptResult {
        rendered_prompt: rendered,
    }
}

fn render_source_body(source: &str) -> String {
    let body = prompt_body(source);
    let cleaned = remove_comments(body);
    let variables = get_variables(&cleaned);
    match find_raw_prompt(&cleaned) {
        Some(raw_prompt) => replace_placeholders(&raw_prompt, &variables),
        None => body.trim().to_string(),
    }
}

fn open_db() -> Result<Connection, String> {
    let path = crate::local_config::config_dir().join("prompts.sqlite");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    Connection::open(path).map_err(|err| err.to_string())
}

fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS prompts (
            id TEXT PRIMARY KEY,
            root TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            tags TEXT NOT NULL,
            description TEXT NOT NULL,
            created_at TEXT,
            updated_at TEXT,
            content_hash TEXT NOT NULL,
            file_mtime INTEGER NOT NULL,
            schema_version INTEGER NOT NULL,
            last_indexed_at TEXT NOT NULL,
            missing INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_prompts_root_missing_updated
            ON prompts(root, missing, updated_at);
        CREATE INDEX IF NOT EXISTS idx_prompts_name
            ON prompts(name);",
    )
    .map_err(|err| err.to_string())
}

fn index_file(conn: &Connection, root: &Path, path: &Path) -> Result<PromptListItem, String> {
    let raw_source =
        fs::read_to_string(path).map_err(|err| format!("Failed to read prompt file: {err}"))?;
    let (frontmatter, body) = split_frontmatter(&raw_source);
    let source = body.to_string();
    let id = id_for_path(root, path);
    let existing = get_item(conn, root, &id)
        .or_else(|_| get_item_by_id(conn, &id))
        .ok();
    let filename = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled Prompt");
    let name = existing
        .as_ref()
        .map(|item| item.name.clone())
        .filter(|value| !value.trim().is_empty())
        .or(frontmatter.name)
        .unwrap_or_else(|| filename.to_string());
    let tags = existing
        .as_ref()
        .map(|item| item.tags.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| normalize_tags(frontmatter.tags));
    let description = existing
        .as_ref()
        .map(|item| item.description.clone())
        .or(frontmatter.description)
        .unwrap_or_default();
    let created_at = existing
        .as_ref()
        .and_then(|item| item.created_at.clone())
        .or(frontmatter.created_at)
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let updated_at = existing
        .as_ref()
        .and_then(|item| item.updated_at.clone())
        .or(frontmatter.updated_at)
        .unwrap_or_else(|| created_at.clone());

    conn.execute(
        "INSERT INTO prompts (
            id, root, path, name, tags, description, created_at, updated_at,
            content_hash, file_mtime, schema_version, last_indexed_at, missing
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)
        ON CONFLICT(id) DO UPDATE SET
            root = excluded.root,
            path = excluded.path,
            name = excluded.name,
            tags = excluded.tags,
            description = excluded.description,
            created_at = excluded.created_at,
            updated_at = excluded.updated_at,
            content_hash = excluded.content_hash,
            file_mtime = excluded.file_mtime,
            schema_version = excluded.schema_version,
            last_indexed_at = excluded.last_indexed_at,
            missing = 0",
        params![
            id,
            root.to_string_lossy().to_string(),
            path.to_string_lossy().to_string(),
            name,
            serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
            description,
            created_at,
            updated_at,
            content_hash(&source),
            file_mtime(path),
            frontmatter.schema_version.unwrap_or(SCHEMA_VERSION),
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|err| err.to_string())?;

    get_item(conn, root, &id_for_path(root, path))
}

fn get_item(conn: &Connection, root: &Path, id: &str) -> Result<PromptListItem, String> {
    conn.query_row(
        "SELECT id, path, name, tags, description, created_at, updated_at
         FROM prompts
         WHERE id = ?1 AND root = ?2 AND missing = 0",
        params![id, root.to_string_lossy().to_string()],
        row_to_item,
    )
    .optional()
    .map_err(|err| err.to_string())?
    .ok_or_else(|| "Prompt not found.".to_string())
}

fn get_item_by_id(conn: &Connection, id: &str) -> Result<PromptListItem, String> {
    conn.query_row(
        "SELECT id, path, name, tags, description, created_at, updated_at
         FROM prompts
         WHERE id = ?1 AND missing = 0",
        params![id],
        row_to_item,
    )
    .optional()
    .map_err(|err| err.to_string())?
    .ok_or_else(|| "Prompt not found.".to_string())
}

fn path_for_id(conn: &Connection, root: &Path, id: &str) -> Result<PathBuf, String> {
    let path = conn
        .query_row(
            "SELECT path FROM prompts WHERE id = ?1 AND root = ?2 AND missing = 0",
            params![id, root.to_string_lossy().to_string()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Prompt not found.".to_string())?;
    Ok(PathBuf::from(path))
}

fn find_prompt_by_title(
    conn: &Connection,
    root: &Path,
    title: &str,
    current_prompt_id: Option<&str>,
) -> Result<Option<PromptListItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, path, name, tags, description, created_at, updated_at
             FROM prompts
             WHERE root = ?1 AND missing = 0 AND lower(name) = lower(?2)
             ORDER BY updated_at DESC, name ASC",
        )
        .map_err(|err| err.to_string())?;
    let items = stmt
        .query_map(
            params![root.to_string_lossy().to_string(), title],
            row_to_item,
        )
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;
    Ok(items
        .into_iter()
        .find(|item| current_prompt_id.map(|id| id != item.id).unwrap_or(true)))
}

fn resolve_prompt_includes(
    conn: &Connection,
    root: &Path,
    source: &str,
    current_prompt_id: Option<&str>,
    visited: &mut HashSet<String>,
    depth: usize,
) -> Result<String, String> {
    if depth >= 8 {
        return Ok(source.to_string());
    }

    let mut output = String::new();
    let mut index = 0;
    while let Some(start) = source[index..].find("{#") {
        let absolute_start = index + start;
        output.push_str(&source[index..absolute_start]);
        let Some(end) = source[absolute_start + 2..].find('}') else {
            output.push_str(&source[absolute_start..]);
            return Ok(output);
        };
        let absolute_end = absolute_start + 2 + end;
        let title = source[absolute_start + 2..absolute_end].trim();
        if title.is_empty() {
            output.push_str(&source[absolute_start..=absolute_end]);
            index = absolute_end + 1;
            continue;
        }

        let Some(item) = find_prompt_by_title(conn, root, title, current_prompt_id)? else {
            output.push_str(&source[absolute_start..=absolute_end]);
            index = absolute_end + 1;
            continue;
        };
        if visited.contains(&item.id) {
            output.push_str(&source[absolute_start..=absolute_end]);
            index = absolute_end + 1;
            continue;
        }
        visited.insert(item.id.clone());
        let included_source = fs::read_to_string(&item.path)
            .map_err(|err| format!("Failed to read included prompt: {err}"))?;
        let rendered = render_source_body(prompt_body(&included_source));
        let resolved =
            resolve_prompt_includes(conn, root, &rendered, Some(&item.id), visited, depth + 1)?;
        visited.remove(&item.id);
        output.push_str(&resolved);
        index = absolute_end + 1;
    }
    output.push_str(&source[index..]);
    Ok(output)
}

fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptListItem> {
    let tags_json: String = row.get(3)?;
    let tags = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(PromptListItem {
        id: row.get(0)?,
        path: row.get(1)?,
        name: row.get(2)?,
        tags,
        description: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn collect_markdown_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("md"))
                .unwrap_or(false)
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn split_frontmatter(source: &str) -> (Frontmatter, &str) {
    let Some(rest) = source.strip_prefix("---") else {
        return (Frontmatter::default(), source);
    };
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim() == "---" {
            let body_start = offset + line.len();
            return (parse_frontmatter(&rest[..offset]), &rest[body_start..]);
        }
        offset += line.len();
    }
    (Frontmatter::default(), source)
}

fn prompt_body(source: &str) -> &str {
    split_frontmatter(source).1
}

fn parse_frontmatter(raw: &str) -> Frontmatter {
    let mut frontmatter = Frontmatter::default();
    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "name" => frontmatter.name = normalize_optional_text(value),
            "tags" => frontmatter.tags = parse_tags(value),
            "description" => frontmatter.description = normalize_optional_text(value),
            "createdAt" => frontmatter.created_at = normalize_optional_text(value),
            "updatedAt" => frontmatter.updated_at = normalize_optional_text(value),
            "schemaVersion" => frontmatter.schema_version = value.parse::<i64>().ok(),
            _ => {}
        }
    }
    frontmatter
}

fn parse_tags(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return normalize_tags(
            trimmed[1..trimmed.len() - 1]
                .split(',')
                .map(|item| trim_quotes(item.trim()).to_string())
                .collect(),
        );
    }
    normalize_tags(
        trimmed
            .split(',')
            .map(|item| trim_quotes(item.trim()).to_string())
            .collect(),
    )
}

fn remove_comments(text: &str) -> String {
    let mut result_lines = Vec::new();
    let mut in_block = false;
    for raw_line in text.lines() {
        let mut i = 0;
        let mut out = String::new();
        while i < raw_line.len() {
            if !in_block {
                if let Some(start) = raw_line[i..].find("/*") {
                    out.push_str(&raw_line[i..i + start]);
                    i += start + 2;
                    in_block = true;
                } else {
                    out.push_str(&raw_line[i..]);
                    break;
                }
            } else if let Some(end) = raw_line[i..].find("*/") {
                i += end + 2;
                in_block = false;
            } else {
                break;
            }
        }
        let trimmed = out.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        result_lines.push(out.trim_end().to_string());
    }
    result_lines.join("\n")
}

fn get_variables(source: &str) -> Vec<(String, String)> {
    let mut variables = Vec::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if is_prompt_start(line) {
            break;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key == "prompt" || !is_variable_name(key) {
            continue;
        }
        variables.push((key.to_string(), trim_quotes(value.trim()).to_string()));
    }
    variables
}

fn find_raw_prompt(source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let mut search_from = 0;
    while let Some(found) = source[search_from..].find("prompt") {
        let prompt_index = search_from + found;
        if prompt_index > 0 {
            let previous = bytes[prompt_index - 1] as char;
            if previous.is_ascii_alphanumeric() || previous == '_' {
                search_from = prompt_index + "prompt".len();
                continue;
            }
        }
        let mut index = prompt_index + "prompt".len();
        if index < bytes.len() {
            let next = bytes[index] as char;
            if next.is_ascii_alphanumeric() || next == '_' {
                search_from = index;
                continue;
            }
        }
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'=' {
            index += 1;
        }
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'{' {
            search_from = prompt_index + "prompt".len();
            continue;
        }
        index += 1;
        let content_start = index;
        let mut depth = 1;
        while index < bytes.len() {
            match bytes[index] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(source[content_start..index].trim().to_string());
                    }
                }
                _ => {}
            }
            index += 1;
        }
        return Some(String::new());
    }
    None
}

fn replace_placeholders(raw_prompt: &str, variables: &[(String, String)]) -> String {
    let mut output = String::new();
    let mut index = 0;
    while let Some(start) = raw_prompt[index..].find('{') {
        let absolute_start = index + start;
        output.push_str(&raw_prompt[index..absolute_start]);
        let Some(end) = raw_prompt[absolute_start + 1..].find('}') else {
            output.push_str(&raw_prompt[absolute_start..]);
            return output;
        };
        let absolute_end = absolute_start + 1 + end;
        let name = &raw_prompt[absolute_start + 1..absolute_end];
        if is_variable_name(name) {
            if let Some((_, value)) = variables.iter().find(|(key, _)| key == name) {
                output.push_str(value);
            } else {
                output.push_str(&raw_prompt[absolute_start..=absolute_end]);
            }
        } else {
            output.push_str(&raw_prompt[absolute_start..=absolute_end]);
        }
        index = absolute_end + 1;
    }
    output.push_str(&raw_prompt[index..]);
    output
}

fn is_prompt_start(line: &str) -> bool {
    if !line.starts_with("prompt") {
        return false;
    }
    if line
        .chars()
        .nth("prompt".len())
        .map(|value| value.is_ascii_alphanumeric() || value == '_')
        .unwrap_or(false)
    {
        return false;
    }
    line.contains('{')
}

fn is_variable_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|item| item.is_ascii_alphanumeric() || item == '_')
}

fn id_for_path(root: &Path, path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if Uuid::parse_str(stem).is_ok() {
        stem.to_string()
    } else {
        let relative = path.strip_prefix(root).unwrap_or(path);
        content_hash(&relative.to_string_lossy())
    }
}

fn content_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn prompt_root(prompt_directory: &str) -> PathBuf {
    if prompt_directory.trim().is_empty() {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".sozocraft")
            .join("prompts")
    } else {
        PathBuf::from(prompt_directory)
    }
}

fn normalize_prompt_name(name: &str) -> String {
    let value = name.trim();
    if value.is_empty() {
        "Untitled Prompt".to_string()
    } else {
        value.to_string()
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let value = tag.trim().trim_start_matches('#').to_string();
        if !value.is_empty() && !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    normalized
}

fn normalize_optional_text(value: &str) -> Option<String> {
    let value = trim_quotes(value.trim()).to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn trim_quotes(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn file_mtime(path: &Path) -> i64 {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_comfy_style_prompt_blocks() {
        let source = r#"character = cinematic portrait
location = neon street

prompt = {
{character} in a {location}.
{unknown}
}
"#;
        let rendered = render_prompt_source(source);
        assert_eq!(
            rendered.rendered_prompt,
            "cinematic portrait in a neon street.\n{unknown}"
        );
    }

    #[test]
    fn strips_legacy_frontmatter_before_rendering() {
        let source = "---\nname: Plain\n---\n\nA plain markdown prompt.";
        let rendered = render_prompt_source(source);
        assert_eq!(rendered.rendered_prompt, "A plain markdown prompt.");
    }

    #[test]
    fn skips_prompt_like_variable_names_before_prompt_block() {
        let source = "prompt_text = not the block\nsubject = cat\nprompt = {\nA {subject}.\n}";
        let rendered = render_prompt_source(source);
        assert_eq!(rendered.rendered_prompt, "A cat.");
    }

    #[test]
    fn empty_prompt_block_renders_empty() {
        let source = "prompt = {\n\n}";
        let rendered = render_prompt_source(source);
        assert_eq!(rendered.rendered_prompt, "");
    }

    #[test]
    fn preserves_blank_lines_inside_prompt_block() {
        let source = "subject = cat\nprompt = {\nLine one.\n\nLine two: {subject}.\n}";
        let rendered = render_prompt_source(source);
        assert_eq!(rendered.rendered_prompt, "Line one.\n\nLine two: cat.");
    }

    #[test]
    fn preserves_synced_metadata_when_prompt_root_changes() {
        let temp_root =
            std::env::temp_dir().join(format!("sozocraft-prompt-root-change-{}", Uuid::new_v4()));
        let old_root = temp_root.join("old").join("prompts");
        let new_root = temp_root.join("new").join("prompts");
        fs::create_dir_all(&old_root).unwrap();
        fs::create_dir_all(&new_root).unwrap();

        let id = Uuid::new_v4().to_string();
        let old_path = old_root.join(format!("{id}.md"));
        let new_path = new_root.join(format!("{id}.md"));
        fs::write(&old_path, "prompt = {\nOld body.\n}").unwrap();
        fs::write(&new_path, "prompt = {\nNew body.\n}").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO prompts (
                id, root, path, name, tags, description, created_at, updated_at,
                content_hash, file_mtime, schema_version, last_indexed_at, missing
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)",
            params![
                id,
                old_root.to_string_lossy().to_string(),
                old_path.to_string_lossy().to_string(),
                "Identify Reference",
                r#"["nano-banana/identity"]"#,
                "portable metadata",
                "2026-01-01T00:00:00Z",
                "2026-01-02T00:00:00Z",
                "old-hash",
                0,
                SCHEMA_VERSION,
                "2026-01-02T00:00:00Z",
            ],
        )
        .unwrap();

        let item = index_file(&conn, &new_root, &new_path).unwrap();

        assert_eq!(item.name, "Identify Reference");
        assert_eq!(item.tags, vec!["nano-banana/identity"]);
        assert_eq!(item.description, "portable metadata");

        fs::remove_dir_all(temp_root).unwrap();
    }
}
