use std::fmt::{Display, Formatter};
use std::fs;
use std::io::ErrorKind;
use std::io::ErrorKind::NotFound;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use fltk::dialog::{alert_default, choice_default};
use glob::{glob, Pattern};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::settings::SettingsError::{SError, SNotFound, SWarning};

pub const SETTINGS_VERSION: &str = "1";

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Settings {
    pub settings_version: String,
    pub backup_paths: Vec<BackupFilePattern>,
    pub backup_dest_path: PathBuf,
    pub backup_count: u8,
    pub backup_delay_sec: u8,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BackupFilePattern {
    pub source_dir: PathBuf,
    pub file_pattern: String
}

impl BackupFilePattern {
    pub fn to_path(&self) -> PathBuf {
        self.source_dir.join(self.file_pattern.clone())
    }
}

#[derive(Error, Debug)]
pub enum SettingsError {
    SNotFound(Option<Settings>),
    SWarning(Settings, String),
    SError(String)
}

impl Display for SettingsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub fn get_settings() -> Result<Settings, SettingsError> {
    let settings = match read_settings() {
        Err(SettingsError::SNotFound(None)) => {
            let settings = write_settings(get_default_settings()?)?;
            Err(SNotFound(Some(settings)))
        },
        Err(err) =>
            Err(err),
        Ok(settings) =>
            Ok(settings)
    }?;

    validate_settings(settings)
}

pub fn validate_settings(settings: Settings) -> Result<Settings, SettingsError> {
    let mut err = Ok(());
    for backup in settings.backup_paths.iter() {
        if !backup.source_dir.is_dir() {
            err = Err(
                format!("Backup folder does not exist: {}", backup.source_dir.to_str().unwrap()));
            break;
        }
        if let Err(_) = Pattern::new(&backup.file_pattern) {
            err = Err(format!("Invalid file pattern: {}", backup.file_pattern));
        }
    }
    if let Err(err_msg) = err {
        return Err(SWarning(settings, err_msg));
    }

    if !settings.backup_paths.is_empty() && settings.backup_dest_path == PathBuf::new() {
        let err_msg = "Missing destination folder".to_string();
        return Err(SWarning(settings, err_msg));
    }
    if settings.backup_dest_path != PathBuf::new() && !settings.backup_dest_path.is_dir() {
        match choice_default(
            format!("Destination folder does not exist: {}\nCreate it?",
                settings.backup_dest_path.to_str().unwrap()).as_str(),
            "Cancel", "Yes", ""
        ) {
            0 => {  // Cancel
                return Err(SWarning(settings, "".to_string()));
            }
            _ => {  // Yes
                if let Err(err) = std::fs::create_dir_all(settings.backup_dest_path.clone()) {
                    alert_default(format!("Error: {}", err).as_str());
                }
            }
        }
    }

    if let Err(err_msg) = err {
        return Err(SWarning(settings, err_msg));
    }

    Ok(settings)
}

fn read_settings() -> Result<Settings, SettingsError> {
    let settings_path = get_settings_path()?;

    let settings_str = match fs::read_to_string(settings_path) {
        Err(err) if err.kind() == NotFound =>
            return Err(SNotFound(None)),
        Err(err) =>
            return Err(SError(format!("Failed to read settings file: {}", err))),
        Ok(str) =>
            str
    };

    let settings: Settings = match serde_json::from_str(&settings_str) {
        Err(err) => return Err(SError(format!("Error reading settings file: {}", err))),
        Ok(settings) => settings
    };

    println!("Read settings: {:?}", settings);
    Ok(settings)
}

pub fn write_settings(settings: Settings) -> Result<Settings, SettingsError> {
    let settings_path = get_settings_path()?;

    let settings_dir_path = settings_path.parent().unwrap();
    if let Err(err) = std::fs::create_dir_all(settings_dir_path) {
        if err.kind() != ErrorKind::AlreadyExists {
            let err_msg = format!("Error creating settings directory {}: {}",
                settings_dir_path.to_str().unwrap(), err);
            println!("{}", err_msg);
            return Err(SWarning(settings, err_msg));
        }
    }

    let settings_str = match serde_json::to_string(&settings) {
        Err(err) => return Err(SError(format!("Error writing settings: {}", err))),
        Ok(settings_str) => settings_str
    };

    match fs::write(settings_path, settings_str.as_bytes()) {
        Err(err) =>
            Err(SWarning( settings, format!("Failed to write settings file: {}", err))),
        Ok(()) =>
            Ok(settings)
    }
}

fn get_settings_path() -> Result<PathBuf, SettingsError> {
    let project_dirs = ProjectDirs::from("org", "valbak", "Valbak");
    match project_dirs {
        None =>
            Err(SError("Failed to find settings folder".to_string())),
        Some(project_dirs) => {
            let settings_path = project_dirs.config_dir();
            let settings_path = settings_path.join(Path::new("settings.json"));
            println!("Using settings file: {:?}", settings_path);
            Ok(settings_path)
        }
    }
}

pub fn get_default_settings() -> Result<Settings, SettingsError> {
    let mut backup_dest_dir = PathBuf::new();

    let backup_paths = match dirs::data_local_dir() {
        None => {
            vec![]
        }
        Some(local_dir) => {
            let mut local_low_dir = local_dir.to_str().unwrap().to_string();
            local_low_dir.push_str("Low");

            let valheim_src_dir = Path::new(&local_low_dir)
                .join("IronGate")
                .join("Valheim");
            let worlds_src_dir = valheim_src_dir.join("worlds");
            let characters_src_dir = valheim_src_dir.join("characters");

            backup_dest_dir = match dirs::document_dir() {
                None => PathBuf::from(""),
                Some(doc_dir) => doc_dir
            };
            backup_dest_dir.push("Valbak");

            (
                vec![
                    BackupFilePattern {
                        source_dir: worlds_src_dir.clone(),
                        // dest_dir: worlds_dest_dir.to_str().unwrap().to_string(),
                        file_pattern: "*.db".to_string()
                    },
                    BackupFilePattern {
                        source_dir: worlds_src_dir.clone(),
                        file_pattern: "*.fwl".to_string()
                    },
                    BackupFilePattern {
                        source_dir: characters_src_dir.clone(),
                        file_pattern: "*.fch".to_string()
                    }
                ]
            )
        }
    };

    Ok(Settings {
        settings_version: SETTINGS_VERSION.to_string(),
        backup_paths,
        backup_dest_path: backup_dest_dir,
        backup_count: 5,
        backup_delay_sec: 10
    })
}