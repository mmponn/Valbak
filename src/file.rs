/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Display, Formatter};
use std::fs::Metadata;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{bail, Result};
use filetime::{FileTime, set_file_mtime};
use glob::{glob, Pattern};
use log::{error, info, warn};
use multimap::MultiMap;

use crate::file::FileError::{FError, FWarning};
use crate::settings::{BackupFilePattern, Settings};

#[derive(thiserror::Error, Debug)]
pub enum FileError {
    FWarning(Vec<String>),
    FError(Vec<String>),
}

impl Display for FileError {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub trait PathExt {
    fn file_name_str(&self) -> &str;
    fn str(&self) -> &str;
}

impl PathExt for Path {
    fn file_name_str(&self) -> &str {
        self.file_name().unwrap().to_str().unwrap()
    }

    fn str(&self) -> &str {
        self.to_str().unwrap()
    }
}

impl PathExt for PathBuf {
    fn file_name_str(&self) -> &str {
        self.file_name().unwrap().to_str().unwrap()
    }

    fn str(&self) -> &str {
        self.to_str().unwrap()
    }
}

/// Queries the filesystem and returns all live files as specified by `settings`
pub fn get_live_files(settings: Settings) -> Result<Vec<PathBuf>> {
    let mut live_files = Vec::new();
    for backup_pattern in &settings.backup_patterns {
        let glob_pattern = backup_pattern.source_dir.join(&backup_pattern.filename_pattern);
        let glob_paths = match glob(glob_pattern.str()) {
            Err(err) =>
                // This should have already happened and been handled
                panic!("illegal state: {}", err),
            Ok(glob_paths) =>
                glob_paths
        };
        for glob_path in glob_paths {
            match glob_path {
                Err(err) =>
                    return Err(FError( vec![format!("Error reading live files: {}", err)] ).into()),
                Ok(file_path) =>
                    live_files.push(file_path)
            }
        }
    }
    Ok(live_files)
}

/// Queries the filesystem and returns all backed up files as specified by `settings`
pub fn get_backed_up_files(settings: Settings) -> Result<Vec<PathBuf>> {
    let mut backed_up_files = Vec::new();
    for backup_pattern in settings.backup_patterns {
        let backup_folder_name = backup_pattern.source_dir.file_name().unwrap();
        let backed_up_versions_filename_pattern = backup_pattern.filename_pattern + ".*";

        let backed_up_versions_pattern = settings.backup_dest_path
            .join(backup_folder_name)
            .join(backed_up_versions_filename_pattern);

        let glob_paths = match glob(backed_up_versions_pattern.str()) {
            Err(err) => {
                error!("Error scanning backed up files for {}: {}", backed_up_versions_pattern.str(), err);
                continue;
            }
            Ok(glob_paths) =>
                glob_paths
        };

        for glob_path in glob_paths {
            match glob_path {
                Err(err) =>
                    error!("Error reading backed up files: {}", err),
                Ok(file_path) => {
                    if get_backed_up_version_number(&file_path).is_some() {
                        backed_up_files.push(file_path);
                    }
                }
            }
        }
    }
    Ok(backed_up_files)
}

/// Searches all live files for any that do not have a backed up version, and creates backups for such files.
pub fn backup_all_changed_files(settings: Settings) -> Result<()> {
    let live_file_paths = get_live_files(settings.clone())?;
    for live_file_path in live_file_paths {
        if !live_file_has_backup(settings.clone(), live_file_path.clone())? {
            back_up_live_file(settings.clone(), live_file_path)?;
            delete_old_backups(settings.clone())?;
        }
    }
    Ok(())
}

/// Determines whether the given live file path has been previously backed up.
/// A live file is considered backed up if a version file is found that matches the live file's size and last-modified
/// timestamp.
pub fn live_file_has_backup(settings: Settings, live_file_path: PathBuf) -> Result<bool> {
    // 1. Find the backup pattern related to this file

    let live_file_folder_name = live_file_path.parent().unwrap().file_name_str();
    let mut found_backup_pattern = None;
    for backup_pattern in &settings.backup_patterns {
        let backup_pattern_folder_name = backup_pattern.source_dir.file_name_str();
        if backup_pattern_folder_name == live_file_folder_name {
            match Pattern::new(backup_pattern.filename_pattern.as_str()) {
                Ok(file_pattern) => {
                    if file_pattern.matches_path(&live_file_path) {
                        found_backup_pattern = Some(backup_pattern);
                    }
                },
                Err(err) => {
                    // This should have already happened and been handled
                    panic!("illegal state: {}", err)
                }
            }
        }
    }
    let backup_pattern = match found_backup_pattern {
        Some(pattern) => pattern,
        None => {
            let error_msg =
                format!("Cannot find backup configuration for changed file {}", live_file_path.str());
            // ui_thread_tx.send( SetStatus(error_msg.to_string()));
            return Err(FWarning( vec![error_msg] ).into());
        }
    };

    // 2. Now use the pattern to search already backed up files to see if any of them appear to be an exact match, or in
    //    other words, determine if the file that just changed appears to be a copy of an already backed up file.

    let backed_up_version_paths =
        get_backed_up_version_paths(settings.backup_dest_path.clone(), backup_pattern.clone())?;

    let (live_file_metadata, live_file_modified) = get_file_metadata(live_file_path.clone())?;

    for backed_up_version_path in backed_up_version_paths {
        let (backed_up_file_metadata, backed_up_file_modified) = get_file_metadata(backed_up_version_path.clone())?;
        if backed_up_file_metadata.len() == live_file_metadata.len() && backed_up_file_modified == live_file_modified {
            info!("{} appears to be a copy of {}", live_file_path.str(), backed_up_version_path.str());
            // ui_thread_tx.send(UiMessage::RefreshFilesLists);
            return Ok(true);
        }
    }

    return Ok(false);
}

/// Creates a new backup version file for `live_file_path`
pub fn back_up_live_file(settings: Settings, live_file_path: PathBuf) -> Result<()> {
    // Copy the file and its containing folder name
    let live_file_folder_name = live_file_path.parent().unwrap().file_name().unwrap();

    let backup_dest_path = settings.backup_dest_path.join(live_file_folder_name);
    if let Err(err) = std::fs::create_dir(backup_dest_path.clone()) {
        if err.kind() != ErrorKind::AlreadyExists {
            bail!("Error copying file: {}", err);
        }
    }
    let live_filename = live_file_path.file_name_str();
    let temp_backup_filename = "_".to_string() + live_filename;

    let temp_backup_file_path = backup_dest_path.join(temp_backup_filename);

    // ui_thread_tx.send(UiMessage::PushStatus(format!("Copying {}", backup_file_from.str())));
    if let Err(err) = std::fs::copy(live_file_path.clone(), temp_backup_file_path.clone()) {
        // ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
        bail!("Error copying file: {}", err);
    }

    let (live_file_metadata, _live_file_modified) = match get_file_metadata(live_file_path.clone()) {
        Ok((metadata, modified)) => (metadata, modified),
        Err(err) => {
            // ui_thread_tx.send(SetStatus(err));
            bail!("{}", err);
        }
    };
    let live_file_modified_filetime = FileTime::from_last_modification_time(&live_file_metadata);
    if let Err(err) = set_file_mtime(temp_backup_file_path.clone(), live_file_modified_filetime) {
        bail!("Error setting backup timestamp for {}: {}", temp_backup_file_path.str(), err);
    }

    let next_version = next_backup_version(&settings, backup_dest_path.clone(), live_filename.to_string())?;
    let backed_up_filename = format!("{}.{}", live_filename, next_version);
    let backed_up_file_path = backup_dest_path.join(backed_up_filename);

    info!("Copying {} to {}", live_file_path.str(), backed_up_file_path.str());

    if let Err(err) = std::fs::rename(temp_backup_file_path, backed_up_file_path) {
        // ui_thread_tx.send(SetStatus(err.to_string()));
        bail!("{}", err);
    }

    // ui_thread_tx.send(UiMessage::PopStatus);
    Ok(())
}

/// Determines the version number for the next backup of `backup_filename` in `backed_up_folder`
fn next_backup_version(_settings: &Settings, backed_up_folder: PathBuf, backup_filename: String) -> Result<u32> {
    let backed_up_versions_pattern = backed_up_folder
        .join(backup_filename + ".*");
    let backed_up_history_files = match glob(backed_up_versions_pattern.str()) {
        Ok(history_paths) => Some(history_paths),
        Err(err) => {
            return Err( FWarning(
                vec![format!("Error scanning backed up files for {}: {}", backed_up_versions_pattern.str(), err)]
            ).into() );
        }
    };
    let oldest_backed_up_version = match backed_up_history_files {
        Some(history_paths) => {
            let mut history_number = 0u32;
            for history_path in history_paths {
                match history_path {
                    Ok(history_path) => {
                        let history_path_number = match get_backed_up_version_number(&history_path) {
                            None =>
                                0u32,
                            Some(history_path_number) =>
                                history_path_number
                        };
                        if history_path_number > history_number {
                            history_number = history_path_number;
                        }
                    }
                    Err(err) => {
                        warn!("Error reading backed up history file: {}", err);
                        continue;
                    }
                }
            }
            history_number
        }
        None =>
            0u32
    };
    Ok(oldest_backed_up_version + 1)
}

/// Parses `backed_up_file_path` and returns its version number
pub fn get_backed_up_version_number(backed_up_file_path: &PathBuf) -> Option<u32> {
    let backed_up_filename = backed_up_file_path.file_name_str();
    match backed_up_filename.rfind(".") {
        None => None,
        Some(dot_index) => {
            let backed_up_filename_suffix = &backed_up_filename[dot_index + 1..];
            match backed_up_filename_suffix.parse::<u32>() {
                Err(_) => None,
                Ok(version_number) => Some(version_number)
            }
        }
    }
}

/// Parses `backed_up_file_path` and returns its filename without any version suffix
pub fn get_backed_up_filename(backed_up_file_path: &PathBuf) -> Option<&str> {
    let backed_up_filename = backed_up_file_path.file_name_str();
    match backed_up_filename.rfind(".") {
        None => None,
        Some(dot_index) => Some(&backed_up_filename[..dot_index])
    }
}

/// Parses `backed_up_file_path` and returns it without any version suffix
pub fn strip_version_suffix_from_backed_up_file_path(backed_up_file_path: &PathBuf) -> Option<PathBuf> {
    let backed_up_file_path_str = backed_up_file_path.str();
    match backed_up_file_path_str.rfind(".") {
        None => None,
        Some(dot_index) => Some(PathBuf::from(&backed_up_file_path_str[..dot_index]))
    }
}

/// Check if there exists more backed up files than is allowed by `settings` and, if there are too many, deletes the
/// oldest backed up file until the number of files complies with the maximum specified by `settings`
pub fn delete_old_backups(settings: Settings) -> Result<()> {
    let mut backed_up_file_paths_by_stripped_file_paths = MultiMap::new();

    let backed_up_file_paths = get_backed_up_files(settings.clone())?;
    for backed_up_file_path in backed_up_file_paths {
        let stripped_backed_up_file_path = match strip_version_suffix_from_backed_up_file_path(&backed_up_file_path) {
            Some(path) => path,
            None => {
                bail!("Unable to find version suffix in {}", backed_up_file_path.str());
            }
        };
        backed_up_file_paths_by_stripped_file_paths.insert(stripped_backed_up_file_path.str().to_string(), backed_up_file_path);
    }

    for (_stripped_path, mut backed_up_paths) in backed_up_file_paths_by_stripped_file_paths {
        if backed_up_paths.len() > settings.backup_count as usize {
            backed_up_paths.sort_by(|a, b| {
                let a_num = get_backed_up_version_number(a).unwrap();
                let b_num = get_backed_up_version_number(b).unwrap();
                a_num.cmp(&b_num)
            });
            let doomed_paths = &backed_up_paths[..backed_up_paths.len() - settings.backup_count as usize];
            doomed_paths.iter()
                .for_each(|path| {
                    info!("Removing {}", path.str());
                    if let Err(err) = std::fs::remove_file(path) {
                        error!("Error removing file {}: {}", path.str(), err);
                    }
                });
        }
    }
    Ok(())
}

/// Deletes each file found in `backed_up_file_paths`
pub fn delete_backed_up_files(backed_up_file_paths: Vec<PathBuf>) -> Result<()> {
    let mut errs = Vec::new();
    for backed_up_path in backed_up_file_paths {
        info!("Deleting backed up file {}", backed_up_path.str());
        if let Err(err) = std::fs::remove_file(backed_up_path.clone()) {
            errs.push(format!("Error deleting file {}: {}", backed_up_path.str(), err));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(FWarning(errs).into())
    }
}

/// Restores each file found in `backed_up_file_paths`
pub fn restore_backed_up_files(settings: Settings, backed_up_file_paths: Vec<PathBuf>) -> Result<()>{
    for backed_up_path in backed_up_file_paths {
        let backed_up_folder_path = backed_up_path.parent().unwrap();

        let source_file_path = get_live_file_for_backed_up_file(settings.clone(), backed_up_path.clone())?;
        let source_filename = source_file_path.file_name_str();

        let temp_source_filename = "_".to_string() + source_filename;
        let temp_source_file_path = backed_up_folder_path.join(temp_source_filename);

        let (backed_up_file_metadata, _backup_file_modified) = match get_file_metadata(backed_up_path.clone()) {
            Ok((metadata, modified)) => (metadata, modified),
            Err(err) => {
                error!("{}: {}", backed_up_path.str(), err);
                continue;
            }
        };
        let backed_up_file_modified_filetime = FileTime::from_last_modification_time(&backed_up_file_metadata);

        if let Err(err) = std::fs::copy(backed_up_path.clone(), temp_source_file_path.clone()) {
            error!("Error copying file from {} to {}: {}",
                backed_up_path.str(), temp_source_file_path.str(), err);
            continue;
        }

        if let Err(err) = set_file_mtime(temp_source_file_path.clone(), backed_up_file_modified_filetime) {
            error!("{}: {}", temp_source_file_path.str(), err);
            continue;
        }

        if let Err(err) = std::fs::rename(temp_source_file_path.clone(), source_file_path.clone()) {
            error!("{}: {}", temp_source_file_path.str(), err);
            continue;
        }

        info!("Restored {}", source_file_path.str());
    }
    Ok(())
}

/// Finds all version files matching `backup_pattern` in `backup_dest_path`
pub fn get_backed_up_version_paths(
    backup_dest_path: PathBuf, backup_pattern: BackupFilePattern
) -> Result<Vec<PathBuf>> {

    // 1. Create an absolute backed up file pattern

    let backed_up_versions_filename_pattern = backup_pattern.filename_pattern + ".*";
    let backed_up_folder_name = backup_pattern.source_dir.file_name().unwrap();
    let backed_up_versions_pattern = backup_dest_path
        .join(backed_up_folder_name)
        .join(backed_up_versions_filename_pattern);

    // 2. Get a list of all files matching the pattern

    let glob_paths = match glob(backed_up_versions_pattern.str()) {
        Err(err) => {
            return Err(
                FError(
                    vec![format!("Error scanning backed up files for {}: {}", backed_up_versions_pattern.str(), err)]
                ).into()
            );
        }
        Ok(glob_paths) =>
            glob_paths
    };

    // 3. Convert matching paths into a path vector

    let mut backed_up_version_paths = vec![];
    for glob_path in glob_paths {
        match glob_path {
            Err(err) =>
                return Err(FError(
                    vec![format!("Error scanning backed up files for {}: {}", backed_up_versions_pattern.str(), err)]
                ).into()
            ),
            Ok(glob_path) =>
                backed_up_version_paths.push(glob_path)
        }
    }

    Ok(backed_up_version_paths)
}

/// Transforms `backed_up_file` into a [`PathBuf`] representing the live file for which `backed_up_file` was originally
/// created. Note that the returned path is not confirmed to exist.
fn get_live_file_for_backed_up_file(settings: Settings, backed_up_file: PathBuf) -> Result<PathBuf> {
    let backed_up_folder_name = backed_up_file.parent().unwrap().file_name().unwrap();

    let stripped_backed_up_filename = match strip_version_suffix_from_backed_up_file_path(&backed_up_file) {
        Some(path) => {
            path.file_name_str().to_string()
        },
        None =>
            return Err(FError( vec![format!("invalid backed up file name: {}", backed_up_file.str())] ).into())
    };

    for backup_pattern in settings.backup_patterns {
        let backup_pattern_path = backup_pattern.to_path();
        let backup_pattern_folder_name = backup_pattern_path.parent().unwrap().file_name().unwrap();

        if backup_pattern_folder_name == backed_up_folder_name {
            let backup_file_pattern = match Pattern::new(backup_pattern_path.str()) {
                Ok(pattern) => pattern,
                Err(err) =>
                    return Err(
                        FError(
                            vec![format!("invalid file pattern \"{}\": {}", backup_pattern_path.str(), err)]
                        ).into()
                    )
            };

            // The file name of the backed up file grafted onto the source path
            let expected_live_file_path = backup_pattern_path.parent().unwrap()
                .join(stripped_backed_up_filename.as_str());

            if backup_file_pattern.matches_path(&expected_live_file_path) {
                return Ok(expected_live_file_path);
            }
        }
    }

    Err(FError( vec![format!("Failed to find source file for backed up file {}", backed_up_file.str())] ).into())
}

/// Queries the filesystem for `file_path` and returns the file's metadata and modification timestamp
pub fn get_file_metadata(file_path: PathBuf ) -> Result<(Metadata, SystemTime)> {
    return match file_path.metadata() {
        Err(err) => {
            let error_msg =
                format!("Cannot read metadata for changed file {}: {}",
                    file_path.str(), err
                ).to_string();
            Err(FError( vec![error_msg] ).into())
        }
        Ok(metadata) => {
            match metadata.modified() {
                Err(err) => {
                    let error_msg =
                        format!("Cannot read metadata for changed file {}: {}",
                            file_path.str(), err
                        ).to_string();
                    Err(FError( vec![error_msg] ).into())
                }
                Ok(modified) => {
                    Ok((metadata, modified))
                }
            }
        }
    }
}