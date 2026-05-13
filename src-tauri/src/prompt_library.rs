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
pub struct RenamePromptTagRequest {
    pub old_tag_path: String,
    pub new_tag_path: String,
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
            conn.execute(
                "UPDATE prompts SET missing = 1 WHERE id = ?1 AND missing != 1",
                params![id],
            )
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

pub fn rename_prompt_tag(
    prompt_directory: &str,
    request: RenamePromptTagRequest,
) -> Result<Vec<PromptListItem>, String> {
    let root = prompt_root(prompt_directory);
    let old_tag_path = normalize_tag_path(&request.old_tag_path);
    let new_tag_path = normalize_tag_path(&request.new_tag_path);
    if old_tag_path.is_empty() || new_tag_path.is_empty() {
        return Err("Tag name cannot be empty.".to_string());
    }
    if old_tag_path == new_tag_path {
        return list_prompts(prompt_directory, None);
    }

    let conn = open_db()?;
    init_db(&conn)?;
    let now = Utc::now().to_rfc3339();
    let mut stmt = conn
        .prepare(
            "SELECT id, path, name, tags, description, created_at, updated_at
             FROM prompts
             WHERE root = ?1 AND missing = 0",
        )
        .map_err(|err| err.to_string())?;
    let items = stmt
        .query_map(params![root.to_string_lossy().to_string()], row_to_item)
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;
    drop(stmt);

    for item in items {
        let next_tags = rename_tags(item.tags.clone(), &old_tag_path, &new_tag_path);
        if next_tags == item.tags {
            continue;
        }
        let tags_json = serde_json::to_string(&next_tags).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "UPDATE prompts
             SET tags = ?1, updated_at = ?2, last_indexed_at = ?3
             WHERE id = ?4 AND root = ?5 AND missing = 0",
            params![
                tags_json,
                now.clone(),
                now,
                item.id,
                root.to_string_lossy().to_string(),
            ],
        )
        .map_err(|err| err.to_string())?;
        let item_path = ensure_existing_prompt_path(&root, Path::new(&item.path))?;
        update_legacy_frontmatter_tags(&item_path, &next_tags)?;
    }

    list_prompts(prompt_directory, None)
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
        None => cleaned.trim().to_string(),
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
            missing = 0
        WHERE prompts.root IS NOT excluded.root
            OR prompts.path IS NOT excluded.path
            OR prompts.name IS NOT excluded.name
            OR prompts.tags IS NOT excluded.tags
            OR prompts.description IS NOT excluded.description
            OR prompts.created_at IS NOT excluded.created_at
            OR prompts.updated_at IS NOT excluded.updated_at
            OR prompts.content_hash IS NOT excluded.content_hash
            OR prompts.file_mtime IS NOT excluded.file_mtime
            OR prompts.schema_version IS NOT excluded.schema_version
            OR prompts.missing IS NOT 0",
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
    ensure_existing_prompt_path(root, Path::new(&path))
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

fn find_prompt_by_reference(
    conn: &Connection,
    root: &Path,
    reference: &str,
    current_prompt_id: Option<&str>,
) -> Result<Option<PromptListItem>, String> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Ok(None);
    }
    let plain_reference = normalize_include_part(reference);

    if let Some((tag_scope, title)) = parse_scoped_include_reference(reference) {
        return find_prompt_by_scoped_reference(conn, root, &tag_scope, &title, current_prompt_id);
    }

    if reference.contains('/') {
        if let Some(item) = find_prompt_by_exact_tag(conn, root, reference, current_prompt_id)? {
            return Ok(Some(item));
        }
        if let Some(item) = find_prompt_by_deprecated_slash_scoped_reference(
            conn,
            root,
            reference,
            current_prompt_id,
        )? {
            return Ok(Some(item));
        }
        return find_prompt_by_search(conn, root, &plain_reference, current_prompt_id);
    }

    if let Some(item) = find_prompt_by_title(conn, root, &plain_reference, current_prompt_id)? {
        return Ok(Some(item));
    }
    if let Some(item) = find_prompt_by_tag_leaf(conn, root, &plain_reference, current_prompt_id)? {
        return Ok(Some(item));
    }
    find_prompt_by_search(conn, root, &plain_reference, current_prompt_id)
}

fn find_prompt_by_exact_tag(
    conn: &Connection,
    root: &Path,
    tag: &str,
    current_prompt_id: Option<&str>,
) -> Result<Option<PromptListItem>, String> {
    let tag = parse_tag_path_reference(tag).unwrap_or_else(|| normalize_tag_path(tag));
    find_prompt_by_tags(conn, root, current_prompt_id, |item_tag| {
        item_tag.eq_ignore_ascii_case(&tag)
    })
}

fn find_prompt_by_tag_leaf(
    conn: &Connection,
    root: &Path,
    leaf: &str,
    current_prompt_id: Option<&str>,
) -> Result<Option<PromptListItem>, String> {
    find_prompt_by_tags(conn, root, current_prompt_id, |item_tag| {
        item_tag
            .rsplit('/')
            .next()
            .map(|value| value.eq_ignore_ascii_case(leaf))
            .unwrap_or(false)
    })
}

fn find_prompt_by_scoped_reference(
    conn: &Connection,
    root: &Path,
    tag_scope: &str,
    title: &str,
    current_prompt_id: Option<&str>,
) -> Result<Option<PromptListItem>, String> {
    let tag_scope = normalize_tag_path(tag_scope);
    let title = normalize_include_part(title);
    if tag_scope.is_empty() || title.is_empty() {
        return Ok(None);
    }

    let items = all_items_for_root(conn, root)?;
    Ok(items.into_iter().find(|item| {
        current_prompt_id.map(|id| id != item.id).unwrap_or(true)
            && item.name.eq_ignore_ascii_case(&title)
            && item.tags.iter().any(|tag| {
                let tag_lower = tag.to_ascii_lowercase();
                let scope_lower = tag_scope.to_ascii_lowercase();
                tag_lower == scope_lower || tag_lower.starts_with(&format!("{scope_lower}/"))
            })
    }))
}

fn find_prompt_by_deprecated_slash_scoped_reference(
    conn: &Connection,
    root: &Path,
    reference: &str,
    current_prompt_id: Option<&str>,
) -> Result<Option<PromptListItem>, String> {
    let Some((tag_scope, title)) = reference.rsplit_once('/') else {
        return Ok(None);
    };
    let tag_scope =
        parse_tag_path_reference(tag_scope).unwrap_or_else(|| normalize_tag_path(tag_scope));
    let title = normalize_include_part(title);
    find_prompt_by_scoped_reference(conn, root, &tag_scope, &title, current_prompt_id)
}

fn find_prompt_by_tags<F>(
    conn: &Connection,
    root: &Path,
    current_prompt_id: Option<&str>,
    matches: F,
) -> Result<Option<PromptListItem>, String>
where
    F: Fn(&str) -> bool,
{
    let items = all_items_for_root(conn, root)?;
    Ok(items.into_iter().find(|item| {
        current_prompt_id.map(|id| id != item.id).unwrap_or(true)
            && item.tags.iter().any(|tag| matches(tag))
    }))
}

fn find_prompt_by_search(
    conn: &Connection,
    root: &Path,
    reference: &str,
    current_prompt_id: Option<&str>,
) -> Result<Option<PromptListItem>, String> {
    let needle = reference.to_ascii_lowercase();
    let items = all_items_for_root(conn, root)?;
    Ok(items.into_iter().find(|item| {
        current_prompt_id.map(|id| id != item.id).unwrap_or(true)
            && (item.name.to_ascii_lowercase().contains(&needle)
                || item.description.to_ascii_lowercase().contains(&needle)
                || item
                    .tags
                    .iter()
                    .any(|tag| tag.to_ascii_lowercase().contains(&needle)))
    }))
}

fn all_items_for_root(conn: &Connection, root: &Path) -> Result<Vec<PromptListItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, path, name, tags, description, created_at, updated_at
             FROM prompts
             WHERE root = ?1 AND missing = 0
             ORDER BY updated_at DESC, name ASC",
        )
        .map_err(|err| err.to_string())?;
    let items = stmt
        .query_map(params![root.to_string_lossy().to_string()], row_to_item)
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;
    Ok(items)
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
        let reference = source[absolute_start + 2..absolute_end].trim();
        if reference.is_empty() {
            output.push_str(&source[absolute_start..=absolute_end]);
            index = absolute_end + 1;
            continue;
        }

        let Some(item) = find_prompt_by_reference(conn, root, reference, current_prompt_id)? else {
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
        let include_path = ensure_existing_prompt_path(root, Path::new(&item.path))?;
        let included_source = fs::read_to_string(&include_path)
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
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file()
                && path
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

fn ensure_existing_prompt_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|err| format!("Failed to inspect prompt root: {err}"))?;
    let path = path
        .canonicalize()
        .map_err(|err| format!("Failed to inspect prompt file: {err}"))?;
    if !path.starts_with(&root) {
        return Err("Prompt file is outside the configured prompt directory.".to_string());
    }
    Ok(path)
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

fn parse_scoped_include_reference(reference: &str) -> Option<(String, String)> {
    let (tag_scope, title) = split_once_unquoted(reference, ':')?;
    let tag_scope = parse_tag_path_reference(tag_scope)?;
    let title = normalize_include_part(title);
    if tag_scope.is_empty() || title.is_empty() {
        None
    } else {
        Some((tag_scope, title))
    }
}

fn parse_tag_path_reference(value: &str) -> Option<String> {
    let parts = split_unquoted(value, '/')
        .into_iter()
        .map(|part| normalize_include_part(part))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        None
    } else {
        Some(normalize_tag_path(&parts.join("/")))
    }
}

fn split_once_unquoted(value: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote.is_some() && character == '\\' {
            escaped = true;
            continue;
        }
        if character == '\'' || character == '"' {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            continue;
        }
        if quote.is_none() && character == delimiter {
            let next = index + character.len_utf8();
            return Some((&value[..index], &value[next..]));
        }
    }
    None
}

fn split_unquoted(value: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote.is_some() && character == '\\' {
            escaped = true;
            continue;
        }
        if character == '\'' || character == '"' {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            continue;
        }
        if quote.is_none() && character == delimiter {
            parts.push(&value[start..index]);
            start = index + character.len_utf8();
        }
    }
    parts.push(&value[start..]);
    parts
}

fn normalize_include_part(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return trimmed.to_string();
    }

    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    if first != '\'' && first != '"' {
        return trimmed.to_string();
    }
    if !trimmed.ends_with(first) {
        return trimmed.to_string();
    }

    let inner = &trimmed[first.len_utf8()..trimmed.len() - first.len_utf8()];
    let mut result = String::new();
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            result.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let value = normalize_tag_path(&tag);
        if !value.is_empty() && !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    normalized
}

fn normalize_tag_path(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('#')
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn rename_tags(tags: Vec<String>, old_tag_path: &str, new_tag_path: &str) -> Vec<String> {
    normalize_tags(
        tags.into_iter()
            .map(|tag| {
                if tag == old_tag_path {
                    new_tag_path.to_string()
                } else if let Some(suffix) = tag.strip_prefix(&format!("{old_tag_path}/")) {
                    format!("{new_tag_path}/{suffix}")
                } else {
                    tag
                }
            })
            .collect(),
    )
}

fn update_legacy_frontmatter_tags(path: &Path, tags: &[String]) -> Result<(), String> {
    let raw_source =
        fs::read_to_string(path).map_err(|err| format!("Failed to read prompt file: {err}"))?;
    let Some(mut rest) = raw_source.strip_prefix("---") else {
        return Ok(());
    };
    let mut rest_start = 3;
    if let Some(stripped) = rest.strip_prefix('\n') {
        rest = stripped;
        rest_start += 1;
    }
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim() == "---" {
            let frontmatter = &rest[..offset];
            if !frontmatter
                .lines()
                .any(|item| item.trim_start().starts_with("tags:"))
            {
                return Ok(());
            }
            let replacement_tags = format!(
                "tags: [{}]",
                tags.iter()
                    .map(|tag| format!("\"{tag}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let mut replaced = false;
            let next_frontmatter = frontmatter
                .lines()
                .map(|item| {
                    if item.trim_start().starts_with("tags:") {
                        replaced = true;
                        replacement_tags.clone()
                    } else {
                        item.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !replaced {
                return Ok(());
            }
            let body_start = rest_start + offset + line.len();
            let next_source = format!(
                "---\n{next_frontmatter}\n---\n{}",
                &raw_source[body_start..]
            );
            fs::write(path, next_source)
                .map_err(|err| format!("Failed to update prompt frontmatter: {err}"))?;
            return Ok(());
        }
        offset += line.len();
    }
    Ok(())
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

    #[test]
    fn indexing_unchanged_file_does_not_rewrite_row() {
        let temp_root =
            std::env::temp_dir().join(format!("sozocraft-prompt-idempotent-{}", Uuid::new_v4()));
        let root = temp_root.join("prompts");
        fs::create_dir_all(&root).unwrap();

        let id = Uuid::new_v4().to_string();
        let path = root.join(format!("{id}.md"));
        fs::write(&path, "prompt = {\nStable body.\n}").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        index_file(&conn, &root, &path).unwrap();
        conn.execute(
            "UPDATE prompts SET last_indexed_at = 'sentinel' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        index_file(&conn, &root, &path).unwrap();
        let last_indexed_at: String = conn
            .query_row(
                "SELECT last_indexed_at FROM prompts WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(last_indexed_at, "sentinel");

        fs::remove_dir_all(temp_root).unwrap();
    }

    #[test]
    fn path_for_id_rejects_file_outside_prompt_root() {
        let temp_root =
            std::env::temp_dir().join(format!("sozocraft-prompt-path-check-{}", Uuid::new_v4()));
        let root = temp_root.join("prompts");
        let outside = temp_root.join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let id = Uuid::new_v4().to_string();
        let outside_path = outside.join(format!("{id}.md"));
        fs::write(&outside_path, "Outside prompt").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        insert_prompt_for_test(
            &conn,
            &root,
            &id,
            &outside_path,
            "outside",
            "[]",
            "2026-01-01T00:00:00Z",
        );

        let err = path_for_id(&conn, &root, &id).unwrap_err();
        assert!(err.contains("outside the configured prompt directory"));

        fs::remove_dir_all(temp_root).unwrap();
    }

    #[test]
    fn resolves_include_by_tag_leaf_and_exact_tag_path() {
        let temp_root =
            std::env::temp_dir().join(format!("sozocraft-prompt-include-{}", Uuid::new_v4()));
        let root = temp_root.join("prompts");
        fs::create_dir_all(&root).unwrap();

        let first_id = Uuid::new_v4().to_string();
        let second_id = Uuid::new_v4().to_string();
        let first_path = root.join(format!("{first_id}.md"));
        let second_path = root.join(format!("{second_id}.md"));
        fs::write(&first_path, "Prompt A").unwrap();
        fs::write(&second_path, "Prompt B").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        insert_prompt_for_test(
            &conn,
            &root,
            &first_id,
            &first_path,
            "my_prompt",
            r#"["gpt-image/abc"]"#,
            "2026-01-01T00:00:00Z",
        );
        insert_prompt_for_test(
            &conn,
            &root,
            &second_id,
            &second_path,
            "my_prompt",
            r#"["nanobanana/abc"]"#,
            "2026-01-02T00:00:00Z",
        );

        let mut visited = HashSet::new();
        let exact =
            resolve_prompt_includes(&conn, &root, "{# gpt-image/abc}", None, &mut visited, 0)
                .unwrap();
        assert_eq!(exact, "Prompt A");

        let mut visited = HashSet::new();
        let leaf = resolve_prompt_includes(&conn, &root, "{# abc}", None, &mut visited, 0).unwrap();
        assert_eq!(leaf, "Prompt B");

        fs::remove_dir_all(temp_root).unwrap();
    }

    #[test]
    fn resolves_include_by_colon_tag_scope_and_title() {
        let temp_root =
            std::env::temp_dir().join(format!("sozocraft-prompt-scoped-{}", Uuid::new_v4()));
        let root = temp_root.join("prompts");
        fs::create_dir_all(&root).unwrap();

        let id = Uuid::new_v4().to_string();
        let path = root.join(format!("{id}.md"));
        fs::write(&path, "Scoped prompt").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        insert_prompt_for_test(
            &conn,
            &root,
            &id,
            &path,
            "identify_ref",
            r#"["gpt-image-2"]"#,
            "2026-01-01T00:00:00Z",
        );

        let mut visited = HashSet::new();
        let resolved = resolve_prompt_includes(
            &conn,
            &root,
            "{# gpt-image-2:identify_ref}",
            None,
            &mut visited,
            0,
        )
        .unwrap();
        assert_eq!(resolved, "Scoped prompt");

        let mut visited = HashSet::new();
        let deprecated = resolve_prompt_includes(
            &conn,
            &root,
            "{# gpt-image-2/identify_ref}",
            None,
            &mut visited,
            0,
        )
        .unwrap();
        assert_eq!(deprecated, "Scoped prompt");

        fs::remove_dir_all(temp_root).unwrap();
    }

    #[test]
    fn omits_commented_include_from_plain_body_render() {
        let temp_root = std::env::temp_dir().join(format!(
            "sozocraft-prompt-comment-include-{}",
            Uuid::new_v4()
        ));
        let root = temp_root.join("prompts");
        fs::create_dir_all(&root).unwrap();

        let id = Uuid::new_v4().to_string();
        let path = root.join(format!("{id}.md"));
        fs::write(&path, "Identity reference").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        insert_prompt_for_test(
            &conn,
            &root,
            &id,
            &path,
            "identify_ref",
            r#"["gpt-image-2"]"#,
            "2026-01-01T00:00:00Z",
        );

        let mut visited = HashSet::new();
        let resolved = resolve_prompt_includes(
            &conn,
            &root,
            &render_source_body("// {# gpt-image-2:identify_ref}\nMain prompt"),
            None,
            &mut visited,
            0,
        )
        .unwrap();
        assert_eq!(resolved, "Main prompt");

        fs::remove_dir_all(temp_root).unwrap();
    }

    #[test]
    fn resolves_include_by_quoted_tag_scope_and_title() {
        let temp_root =
            std::env::temp_dir().join(format!("sozocraft-prompt-quoted-{}", Uuid::new_v4()));
        let root = temp_root.join("prompts");
        fs::create_dir_all(&root).unwrap();

        let id = Uuid::new_v4().to_string();
        let path = root.join(format!("{id}.md"));
        fs::write(&path, "Quoted prompt").unwrap();

        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        insert_prompt_for_test(
            &conn,
            &root,
            &id,
            &path,
            "ai girl",
            r#"["nano banana/sub1/sub2"]"#,
            "2026-01-01T00:00:00Z",
        );

        let mut visited = HashSet::new();
        let resolved = resolve_prompt_includes(
            &conn,
            &root,
            r#"{# "nano banana"/sub1/sub2:"ai girl"}"#,
            None,
            &mut visited,
            0,
        )
        .unwrap();
        assert_eq!(resolved, "Quoted prompt");

        fs::remove_dir_all(temp_root).unwrap();
    }

    #[test]
    fn renames_tag_paths_with_descendants() {
        assert_eq!(
            rename_tags(
                vec![
                    "gpt-image/abc".to_string(),
                    "gpt-image/abc/detail".to_string(),
                    "nanobanana/abc".to_string(),
                ],
                "gpt-image/abc",
                "gpt-image/def",
            ),
            vec![
                "gpt-image/def".to_string(),
                "gpt-image/def/detail".to_string(),
                "nanobanana/abc".to_string(),
            ]
        );
    }

    fn insert_prompt_for_test(
        conn: &Connection,
        root: &Path,
        id: &str,
        path: &Path,
        name: &str,
        tags: &str,
        updated_at: &str,
    ) {
        conn.execute(
            "INSERT INTO prompts (
                id, root, path, name, tags, description, created_at, updated_at,
                content_hash, file_mtime, schema_version, last_indexed_at, missing
            ) VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, ?7, 'hash', 0, ?8, ?7, 0)",
            params![
                id,
                root.to_string_lossy().to_string(),
                path.to_string_lossy().to_string(),
                name,
                tags,
                "2026-01-01T00:00:00Z",
                updated_at,
                SCHEMA_VERSION,
            ],
        )
        .unwrap();
    }
}
