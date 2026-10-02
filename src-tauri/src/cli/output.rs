use crate::{filename_template::resolve_output_path, models::AppSettings};
use chrono::Local;
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub(super) fn validate_output(path: &Path, extension: &str) -> Result<(), String> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case(extension))
    {
        return Err(format!("Output must be a new .{extension} file."));
    }
    match fs::symlink_metadata(path) {
        Ok(_) => Err("Output already exists; choose a new file.".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Cannot inspect output path.".to_string()),
    }
}

pub(super) struct Output {
    pub(super) final_path: PathBuf,
    pub(super) partial: PathBuf,
}

impl Output {
    #[cfg(test)]
    pub(super) fn prepare(
        path: Option<PathBuf>,
        settings: &AppSettings,
        model: &str,
    ) -> Result<Self, String> {
        Self::prepare_video(path, settings, "google", model)
    }

    pub(super) fn prepare_video(
        path: Option<PathBuf>,
        settings: &AppSettings,
        provider: &str,
        model: &str,
    ) -> Result<Self, String> {
        let path = match path {
            Some(path) => path,
            None => {
                let batch_id = Uuid::new_v4().to_string();
                resolve_output_path(
                    &settings.output_directory,
                    &settings.output_template,
                    provider,
                    model,
                    "001",
                    &batch_id[..6],
                    "mp4",
                    Local::now(),
                )
                .map_err(|error| error.to_string())?
            }
        };
        Self::for_path(path, "mp4")
    }

    pub(super) fn for_path(path: PathBuf, extension: &str) -> Result<Self, String> {
        validate_output(&path, extension)?;
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|_| "Cannot determine working directory.")?
                .join(path)
        };
        let parent = path.parent().ok_or("Invalid output directory.")?;
        fs::create_dir_all(parent).map_err(|_| "Cannot create output directory.")?;
        let parent = parent
            .canonicalize()
            .map_err(|_| "Cannot resolve output directory.")?;
        let final_path = parent.join(path.file_name().ok_or("Invalid output filename.")?);
        let partial = parent.join(format!(".sozocraft-cli-{}.part", Uuid::new_v4()));
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|_| "Cannot write to output directory.")?;
        Ok(Self {
            final_path,
            partial,
        })
    }

    pub(super) fn publish(&self) -> Result<(), String> {
        // A hard link atomically publishes the complete file without replacing any existing path.
        fs::hard_link(&self.partial, &self.final_path).map_err(|_| "Cannot publish output; the destination may already exist or the filesystem may not support hard links.".to_string())
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.partial);
    }
}
