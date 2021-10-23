use std::fmt::{Display, Formatter};
use std::fs;
use std::io::ErrorKind::NotFound;
use std::path::{Path, PathBuf};
use thiserror::Error;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use crate::settings::SettingsError::{SWarning, SError, SNotFound};

#[derive(Deserialize, Serialize, Debug)]
pub struct Settings {
    pub settings_version: String,
    pub backup_paths: Vec<Backup>,
    pub redirect_paths: Vec<RedirectPath>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Backup {
    pub source_dir: String,
    pub dest_dir: String,
    pub file_pattern: String
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RedirectPath {
    pub from_dir: String,
    pub to_dir: String
}

#[derive(Error, Debug)]
pub enum SettingsError {
    SNotFound(Option<Settings>),
    SWarning(Option<Settings>, String),
    SError(String)
}

impl Display for SettingsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub fn get_settings() -> Result<Settings, SettingsError> {
    match read_settings() {
        Err(SettingsError::SNotFound(None)) => {
            let settings = write_settings(default_settings()?)?;
            Err(SNotFound(Some(settings)))
        },
        Err(err) =>
            Err(err),
        Ok(settings) =>
            Ok(settings),
    }
}

fn validate_settings(settings: Settings) -> Result<Settings, SettingsError> {
    let mut err = None;
    for backup in settings.backup_paths.iter() {
        if !backup.source_dir.is_empty() && !Path::new(&backup.source_dir).is_dir() {
            err = Some(SWarning(None, format!("Folder does not exist: {}", backup.source_dir)));
            break;
        }
        if !backup.dest_dir.is_empty() && !Path::new(&backup.dest_dir).is_dir() {
            err = Some(SWarning(None, format!("Folder does not exist: {}", backup.dest_dir)));
            break;
        }
    }
   if let Some(SWarning(None, msg)) = err {
        return Err(SWarning(Some(settings), msg));
   }

    for redirect in settings.redirect_paths.iter() {
        if !redirect.from_dir.is_empty() && !Path::new(&redirect.from_dir).is_dir() {
            err = Some(SWarning(None, format!("Folder does not exist: {}", redirect.to_dir)));
            break;
        }
        if !redirect.to_dir.is_empty() && !Path::new(&redirect.to_dir).is_dir() {
            err = Some(SWarning(None, format!("Folder does not exist: {}", redirect.to_dir)));
            break;
        }
    }
    if let Some(SWarning(None, msg)) = err {
        return Err(SWarning(Some(settings), msg));
    }

    Ok(settings)
}

fn read_settings() -> Result<Settings, SettingsError> {
    let settings_path = get_settings_path()?;

    let settings_str = match fs::read_to_string(settings_path) {
        Err(err) if err.kind() == NotFound =>
            return Err(SNotFound(None)),
        Err(err) =>
            return Err(SWarning(
                None,
                format!("Failed to read settings file: {}", err))),
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

fn write_settings(settings: Settings) -> Result<Settings, SettingsError> {
    let settings_path = get_settings_path()?;

    let settings_str = match serde_json::to_string(&settings) {
        Err(err) => return Err(SError(format!("Error writing settings: {}", err))),
        Ok(settings_str) => settings_str
    };

    match fs::write(settings_path, settings_str.as_bytes()) {
        Err(err) =>
            Err(SWarning(
                Some(settings),
                format!("Failed to write settings file: {}", err))),
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

fn default_settings() -> Result<Settings, SettingsError> {
    let (backup_paths, redirect_paths) = match dirs::data_local_dir() {
        None => {
            (vec!(), vec!())
        }
        Some(local_dir) => {
            let mut local_low_dir = local_dir.to_str().unwrap().to_string();
            local_low_dir.push_str("Low");

            let valheim_dir = Path::new(&local_low_dir)
                .join("IronGate")
                .join("Valheim");
            let worlds_src_dir = valheim_dir.join("worlds");
            let characters_src_dir = valheim_dir.join("characters");

            let mut dest_dir = match dirs::document_dir() {
                None => PathBuf::from(""),
                Some(doc_dir) => doc_dir
            };
            dest_dir.push("Valbak");
            let worlds_dest_dir = dest_dir.join("worlds");
            let characters_dest_dir = dest_dir.join("characters");

            (
                vec!(
                    Backup {
                        source_dir: worlds_src_dir.to_str().unwrap().to_string(),
                        dest_dir: worlds_dest_dir.to_str().unwrap().to_string(),
                        file_pattern: "*.db".to_string()
                    },
                    Backup {
                        source_dir: worlds_src_dir.to_str().unwrap().to_string(),
                        dest_dir: worlds_dest_dir.to_str().unwrap().to_string(),
                        file_pattern: "*.fwl".to_string()
                    },
                    Backup {
                        source_dir: characters_src_dir.to_str().unwrap().to_string(),
                        dest_dir: characters_dest_dir.to_str().unwrap().to_string(),
                        file_pattern: "*.fch".to_string()
                    }
                ),
                vec!(
                    RedirectPath {
                        from_dir: worlds_src_dir.to_str().unwrap().to_string(),
                        to_dir: "".to_string()
                    },
                    RedirectPath {
                        from_dir: characters_src_dir.to_str().unwrap().to_string(),
                        to_dir: "".to_string()
                    }
                )
            )
        }
    };

    Ok(Settings {
        settings_version: String::from("1"),
        backup_paths,
        redirect_paths
    })
}
