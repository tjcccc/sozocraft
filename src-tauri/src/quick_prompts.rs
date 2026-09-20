//! Quick drafts are private plain-text files, never prompt-library records.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io::Write, path::Path, sync::Mutex};
use uuid::Uuid;

const MAX_SOURCE: usize = 1024 * 1024;
static STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickTab {
    id: String,
    number: u8,
    source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickState {
    enabled: bool,
    pending: bool,
    active_id: String,
    title: String,
    tags: String,
    tabs: Vec<QuickTab>,
}

impl Default for QuickState {
    fn default() -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            enabled: false,
            pending: false,
            active_id: id.clone(),
            title: String::new(),
            tags: String::new(),
            tabs: vec![QuickTab {
                id,
                number: 1,
                source: String::new(),
            }],
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    state: QuickState,
    files: Vec<String>,
}

fn validate(state: &QuickState) -> Result<(), String> {
    if state.tabs.is_empty()
        || state.tabs.len() > 8
        || state.title.len() > 512
        || state.tags.len() > 4096
        || (state.enabled && state.pending)
    {
        return Err("Invalid Quick workspace".into());
    }
    let mut ids = HashSet::new();
    let mut numbers = HashSet::new();
    for tab in &state.tabs {
        if Uuid::parse_str(&tab.id).is_err()
            || !ids.insert(&tab.id)
            || !(1..=8).contains(&tab.number)
            || !numbers.insert(tab.number)
            || tab.source.len() > MAX_SOURCE
        {
            return Err("Invalid Quick tab (maximum text size is 1 MB)".into());
        }
    }
    if !ids.contains(&state.active_id) {
        return Err("Quick active tab is missing".into());
    }
    Ok(())
}

fn regular_file(path: &Path, max: u64) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() || meta.len() > max {
        return Err("Invalid Quick storage file".into());
    }
    Ok(())
}

fn valid_filename(name: &str) -> bool {
    name.starts_with("prompt_")
        && name.ends_with(".md")
        && name.len() < 100
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-.".contains(&c))
}

fn read_manifest(root: &Path) -> Result<Option<Manifest>, String> {
    match fs::symlink_metadata(root) {
        Ok(meta) if !meta.file_type().is_dir() => {
            return Err("Invalid Quick storage directory".into())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
        _ => {}
    }
    let path = root.join("workspace.json");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
        Ok(_) => {}
    }
    regular_file(&path, 64 * 1024)?;
    let manifest: Manifest = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    validate(&manifest.state)?;
    if manifest.files.len() != manifest.state.tabs.len()
        || manifest.files.iter().any(|f| !valid_filename(f))
    {
        return Err("Invalid Quick manifest files".into());
    }
    Ok(Some(manifest))
}

fn load(root: &Path) -> Result<QuickState, String> {
    let Some(mut manifest) = read_manifest(root)? else {
        return Ok(QuickState::default());
    };
    for (tab, name) in manifest.state.tabs.iter_mut().zip(&manifest.files) {
        let path = root.join(name);
        regular_file(&path, MAX_SOURCE as u64)?;
        tab.source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    }
    validate(&manifest.state)?;
    Ok(manifest.state)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

fn save(root: &Path, state: QuickState) -> Result<(), String> {
    validate(&state)?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    if !fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_dir()
    {
        return Err("Quick storage must be a directory, not a symbolic link".into());
    }
    let previous = read_manifest(root)?;
    let mut manifest = Manifest {
        state,
        files: vec![],
    };
    for tab in &mut manifest.state.tabs {
        let name = format!(
            "prompt_{}_{}.md",
            chrono::Utc::now().timestamp_millis(),
            Uuid::new_v4()
        );
        write_new(&root.join(&name), tab.source.as_bytes())?;
        manifest.files.push(name);
        tab.source.clear();
    }
    let temporary = root.join(format!("workspace-{}.tmp", Uuid::new_v4()));
    write_new(
        &temporary,
        &serde_json::to_vec(&manifest).map_err(|e| e.to_string())?,
    )?;
    fs::rename(&temporary, root.join("workspace.json")).map_err(|e| e.to_string())?;
    // Only remove files owned by the previous manifest, after committing its replacement.
    if let Some(previous) = previous {
        for name in previous.files {
            let _ = fs::remove_file(root.join(name));
        }
    }
    Ok(())
}

fn root() -> Result<std::path::PathBuf, String> {
    dirs::home_dir()
        .map(|p| p.join(".sozocraft").join("quick-prompts"))
        .ok_or_else(|| "Home directory is unavailable".into())
}

#[tauri::command]
pub fn load_quick_prompts() -> Result<QuickState, String> {
    let _guard = STORE_LOCK.lock().map_err(|e| e.to_string())?;
    load(&root()?)
}

#[tauri::command]
pub fn save_quick_prompts(state: QuickState) -> Result<(), String> {
    let _guard = STORE_LOCK.lock().map_err(|e| e.to_string())?;
    save(&root()?, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_plain_text_tabs_and_replaces_old_files() {
        let root = std::env::temp_dir().join(format!("sozocraft-quick-{}", Uuid::new_v4()));
        let mut state = QuickState::default();
        state.enabled = true;
        state.tabs[0].source = "# literal text\n{not DSL}".into();
        save(&root, state.clone()).unwrap();
        assert_eq!(load(&root).unwrap().tabs[0].source, state.tabs[0].source);
        state.enabled = false;
        state.pending = true;
        save(&root, state).unwrap();
        assert!(load(&root).unwrap().pending);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_invalid_tabs_and_paths() {
        let mut state = QuickState::default();
        state.tabs.push(state.tabs[0].clone());
        assert!(validate(&state).is_err());
        assert!(!valid_filename("../prompt_a.md"));
        assert!(!valid_filename("prompt_/etc/passwd.md"));
        state.tabs.truncate(1);
        state.tabs[0].source = "a".repeat(MAX_SOURCE + 1);
        assert!(validate(&state).is_err());
    }
    #[test]
    fn rejects_corrupt_manifest_without_overwriting_it() {
        let root = std::env::temp_dir().join(format!("sozocraft-quick-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("workspace.json"), b"broken").unwrap();
        assert!(load(&root).is_err());
        assert!(save(&root, QuickState::default()).is_err());
        assert_eq!(fs::read(root.join("workspace.json")).unwrap(), b"broken");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_draft_files() {
        let root = std::env::temp_dir().join(format!("sozocraft-quick-{}", Uuid::new_v4()));
        save(&root, QuickState::default()).unwrap();
        let name = read_manifest(&root).unwrap().unwrap().files[0].clone();
        fs::remove_file(root.join(&name)).unwrap();
        std::os::unix::fs::symlink(root.join("workspace.json"), root.join(name)).unwrap();
        assert!(load(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
