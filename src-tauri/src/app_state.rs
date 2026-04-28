use crate::{local_config, models::AppState};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn load_state() -> Result<AppState, std::io::Error> {
    let path = state_path();
    if !path.exists() {
        let mut state = AppState::default();
        state.settings = local_config::load_settings(state.settings);
        save_state(&state)?;
        return Ok(state);
    }

    let raw = fs::read_to_string(path)?;
    let mut state = serde_json::from_str(&raw).unwrap_or_else(|_| AppState::default());
    state.settings = local_config::load_settings(state.settings);
    Ok(state)
}

pub fn save_state(state: &AppState) -> Result<(), std::io::Error> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(state)?;
    fs::write(path, raw)
}

fn state_path() -> PathBuf {
    app_data_dir().join("state.json")
}

fn app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join("SozoCraft")
}
