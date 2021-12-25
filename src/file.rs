use core::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::format;
use std::fs::Metadata;
use std::io::ErrorKind;
use std::iter::FromIterator;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use filetime::{FileTime, set_file_mtime};
use fltk::app;
use fltk::app::version;
use glob::{glob, GlobResult, Paths, Pattern, PatternError};
use multimap::MultiMap;

use crate::{AlertQuit, SetStatus, UiMessage};
use crate::settings::{BackupFilePattern, Settings};

pub fn get_live_files(settings: Settings) -> Vec<PathBuf> {
    let mut live_files = Vec::new();
    for backup_file_pattern in &settings.backup_paths {
        let glob_pattern = backup_file_pattern.source_dir.join(&backup_file_pattern.file_pattern);
        let glob_paths = match glob(glob_pattern.to_str().unwrap()) {
            Err(err) =>
                // This should have already been caught
                panic!("illegal state: {}", err),
            Ok(glob_paths) =>
                glob_paths
        };
        for glob_path in glob_paths {
            match glob_path {
                Err(err) =>
                    println!("Error reading live files: {}", err),
                Ok(file_path) =>
                    live_files.push(file_path)
            }
        }
    }
    live_files
}

pub fn get_backed_up_files(settings: Settings) -> Vec<PathBuf> {
    let mut backed_up_files = Vec::new();
    for backup_file_pattern in settings.backup_paths {
        let backup_file_pattern_history_pattern = backup_file_pattern.file_pattern + ".*";
        let backup_file_folder_name = backup_file_pattern.source_dir.file_name().unwrap();
        let backed_up_pattern = settings.backup_dest_path
            .join(backup_file_folder_name)
            .join(backup_file_pattern_history_pattern);

        let glob_paths = match glob(backed_up_pattern.to_str().unwrap()) {
            Err(err) => {
                println!("Error scanning backed up files for {}: {}", backed_up_pattern.to_str().unwrap(), err);
                continue;
            }
            Ok(glob_paths) =>
                glob_paths
        };

        for glob_path in glob_paths {
            match glob_path {
                Err(err) =>
                    println!("Error reading backed up files: {}", err),
                Ok(file_path) => {
                    if get_history_file_number(&file_path).is_some() {
                        backed_up_files.push(file_path);
                    }
                }
            }
        }
    }
    backed_up_files
}

pub fn backup_all_changed_files(settings: Settings) -> Result<(), String> {
    let live_files = get_live_files(settings.clone());
    for backup_file_path in live_files {
        let file_has_backup = match file_has_backup(settings.clone(), backup_file_path.clone()) {
            Ok(has_backup) => has_backup,
            Err(err) => {
                return Err(err);
            }
        };
        if !file_has_backup {
            backup_file(settings.clone(), backup_file_path);
            clean_backups(settings.clone());
        }
    }
    Ok(())
}

pub fn file_has_backup(settings: Settings, backup_file_path: PathBuf) -> Result<bool, String> {
    // 1. Find the backup pattern related to this file

    let mut backup_file_pattern = None;
    for backup_pattern in &settings.backup_paths {
        match Pattern::new(backup_pattern.file_pattern.as_str()) {
            Ok(file_pattern) => {
                if file_pattern.matches_path(&backup_file_path) {
                    backup_file_pattern = Some(backup_pattern);
                }
            },
            Err(err) => {
                // This should have already been caught
                panic!("illegal state: {}", err)
            }
        }
    }
    let backup_file_pattern = match backup_file_pattern {
        Some(pattern) => pattern,
        None => {
            let error_msg =
                format!("Cannot find backup configuration for changed file {}", backup_file_path.to_str().unwrap());
            // println!("{}", error_msg);
            // ui_thread_tx.send( SetStatus(error_msg.to_string()));
            return Err(error_msg);
        }
    };

    // 2. Now use the pattern to search already backed up files to see if any of them appear to be an exact match, or in
    //    other words, determine if the file that just changed appears to be a copy of an already backed up file.

    let backed_up_file_paths = get_backed_up_file_paths(
        backup_file_pattern.clone(), settings.backup_dest_path.clone());
    let backed_up_file_paths = match backed_up_file_paths {
        Ok(paths) => paths,
        Err(err) => {
            let error_msg = format!("Failed to read backed up files: {}", err);
            // println!("{}", error_msg);
            // ui_thread_tx.send( SetStatus(error_msg.to_string()));
            return Err(error_msg) ;
        }
    };

    let (backup_file_metadata, backup_file_modified) = match get_file_metadata(backup_file_path.clone()) {
        Ok((metadata, modified)) => (metadata, modified),
        Err(err) => {
            // println!("{}", err);
            // ui_thread_tx.send(UiMessage::SetStatus(err));
            return Err(err);
        }
    };

    for backed_up_file_path in backed_up_file_paths {
        let (backed_up_file_metadata, backed_up_file_modified) = match get_file_metadata(backed_up_file_path.clone()) {
            Ok((metadata, modified)) => (metadata, modified),
            Err(err) => {
                // println!("{}", err);
                // ui_thread_tx.send(SetStatus(err));
                return Err(err);
            }
        };
        if backed_up_file_metadata.len() == backup_file_metadata.len()
            && backed_up_file_modified == backup_file_modified {
            println!("{} appears to be a copy of {}",
                backup_file_path.to_str().unwrap(), backed_up_file_path.to_str().unwrap());
            // ui_thread_tx.send(UiMessage::RefreshFilesLists);
            return Ok(true);
        }
    }

    return Ok(false);
}

pub fn backup_file(settings: Settings, backup_file_from: PathBuf) {
    // Copy the file and its containing folder name
    let backup_file_from_folder = backup_file_from.parent().unwrap().file_name().unwrap();

    let backup_file_to_folder = settings.backup_dest_path.join(backup_file_from_folder);
    if let Err(err) = std::fs::create_dir(backup_file_to_folder.clone()) {
        if err.kind() != ErrorKind::AlreadyExists {
            println!("Error copying file: {}", err);
            return;
        }
    }
    let backup_file_name = backup_file_from.file_name().unwrap().to_str().unwrap();
    let temp_backup_file_name = "_".to_string() + backup_file_name;

    let backup_file_to = backup_file_to_folder.join(backup_file_name);
    let temp_backup_file_to = backup_file_to_folder.join(temp_backup_file_name);

    // ui_thread_tx.send(UiMessage::PushStatus(format!("Copying {}", backup_file_from.to_str().unwrap())));
    if let Err(err) = std::fs::copy(backup_file_from.clone(), temp_backup_file_to.clone()) {
        println!("Error copying file: {}", err);
        // ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
        return;
    }

    let (backup_file_metadata, _backup_file_modified) = match get_file_metadata(backup_file_from.clone()) {
        Ok((metadata, modified)) => (metadata, modified),
        Err(err) => {
            println!("{}", err);
            // ui_thread_tx.send(SetStatus(err));
            return;
        }
    };
    let backup_file_modified_filetime = FileTime::from_last_modification_time(&backup_file_metadata);
    set_file_mtime(temp_backup_file_to.clone(), backup_file_modified_filetime);

    let versioned_backed_up_file_path = get_next_backup_filename(&settings, backup_file_to.clone());

    println!("Copying {} to {}", backup_file_from.to_str().unwrap(), versioned_backed_up_file_path.to_str().unwrap());

    if let Err(err) = std::fs::rename(temp_backup_file_to, versioned_backed_up_file_path) {
        println!("{}", err);
        // ui_thread_tx.send(SetStatus(err.to_string()));
        return;
    }

    // ui_thread_tx.send(UiMessage::PopStatus);
}

fn get_next_backup_filename(settings: &Settings, unversioned_backed_up_file_path: PathBuf) -> PathBuf {
    let backed_up_folder = unversioned_backed_up_file_path.parent().unwrap();
    let unversioned_filename = unversioned_backed_up_file_path.file_name().unwrap().to_str().unwrap();
    let backed_up_history_pattern = backed_up_folder
        .join(unversioned_filename.to_string() + ".*");
    let backed_up_history_files = match glob(backed_up_history_pattern.to_str().unwrap()) {
        Ok(history_paths) => Some(history_paths),
        Err(err) => {
            println!("Error scanning backed up files for {}: {}", backed_up_history_pattern.to_str().unwrap(), err);
            None
        }
    };
    let last_backup_file_history_number = match backed_up_history_files {
        Some(history_paths) => {
            let mut history_number = 0u32;
            for history_path in history_paths {
                match history_path {
                    Ok(history_path) => {
                        let history_path_number = match get_history_file_number(&history_path) {
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
                        println!("Error reading backed up history files: {}", err);
                        continue;
                    }
                }
            }
            history_number
        }
        None =>
            0u32
    };
    backed_up_folder.join(
        format!("{}.{}",
            unversioned_filename,
            last_backup_file_history_number + 1)
    )
}

pub fn get_history_file_number(history_file: &PathBuf) -> Option<u32> {
    let history_filename = history_file.file_name().unwrap().to_str().unwrap();
    match history_filename.rfind(".") {
        None =>
            None,
        Some(dot_index) => {
            let history_filename_suffix = &history_filename[dot_index + 1..];
            match history_filename_suffix.parse::<u32>() {
                Err(_) => None,
                Ok(history_num) => Some(history_num)
            }
        }
    }
}

pub fn get_history_file_name(history_file: &PathBuf) -> Option<&str> {
    let history_filename = history_file.file_name().unwrap().to_str().unwrap();
    match history_filename.rfind(".") {
        None => None,
        Some(dot_index) => Some(&history_filename[..dot_index])
    }
}

pub fn clean_backups(settings: Settings) {
    let mut backed_up_file_paths_by_stripped_file_paths = MultiMap::new();

    let backed_up_files = get_backed_up_files(settings.clone());
    for backed_up_file in backed_up_files {
        let history_file_number = get_history_file_number(&backed_up_file).unwrap();
        let history_file_number_str = history_file_number.to_string();
        let backed_up_file_str = backed_up_file.to_str().unwrap();
        let stripped_backed_up_file = &backed_up_file_str[..backed_up_file_str.len() - history_file_number_str.len() - 1];
        backed_up_file_paths_by_stripped_file_paths.insert(stripped_backed_up_file.to_string(), backed_up_file);
    }

    for (_stripped_path, mut backed_up_paths) in backed_up_file_paths_by_stripped_file_paths {
        if backed_up_paths.len() > settings.backup_count as usize {
            backed_up_paths.sort_by(|a, b| {
                let a_num = get_history_file_number(a).unwrap();
                let b_num = get_history_file_number(a).unwrap();
                a_num.cmp(&b_num)
            });
            let doomed_paths = &backed_up_paths[..backed_up_paths.len() - settings.backup_count as usize];
            doomed_paths.iter()
                .for_each(|path| {
                    println!("Removing {}", path.to_str().unwrap());
                    if let Err(err) = std::fs::remove_file(path) {
                        println!("Error removing file {}: {}", path.to_str().unwrap(), err);
                    }
                });
        }
    }
}

pub fn delete_backed_up_files(backed_up_files: Vec<PathBuf>) {
    for backed_up_path in backed_up_files {
        println!("Deleting backed up file {}", backed_up_path.to_str().unwrap());
        if let Err(err) = std::fs::remove_file(backed_up_path.clone()) {
            println!("Error deleting file {}: {}", backed_up_path.to_str().unwrap(), err);
        }
    }
}

pub fn restore_backed_up_files(settings: Settings, selected_backed_up_paths: Vec<PathBuf>) {
    for backed_up_path in selected_backed_up_paths {
        let backed_up_folder_path = backed_up_path.parent().unwrap();

        let source_file_path = match get_source_file_for_backed_up_file(settings.clone(), backed_up_path.clone()) {
            Ok(path) => path,
            Err(err) => {
                println!("{}: {}", backed_up_path.to_str().unwrap(), err);
                continue;
            }
        };
        let source_filename = source_file_path.file_name().unwrap().to_str().unwrap();

        let temp_source_filename = "_".to_string() + source_filename;
        let temp_source_file_path = backed_up_folder_path.join(temp_source_filename);

        let (backed_up_file_metadata, backup_file_modified) = match get_file_metadata(backed_up_path.clone()) {
            Ok((metadata, modified)) => (metadata, modified),
            Err(err) => {
                println!("{}: {}", backed_up_path.to_str().unwrap(), err);
                continue;
            }
        };
        let backed_up_file_modified_filetime = FileTime::from_last_modification_time(&backed_up_file_metadata);

        if let Err(err) = std::fs::copy(backed_up_path.clone(), temp_source_file_path.clone()) {
            println!("Error copying file from {} to {}: {}",
                backed_up_path.to_str().unwrap(), temp_source_file_path.to_str().unwrap(), err);
            continue;
        }

        if let Err(err) = set_file_mtime(temp_source_file_path.clone(), backed_up_file_modified_filetime) {
            println!("{}: {}", temp_source_file_path.to_str().unwrap(), err);
            continue;
        }

        if let Err(err) = std::fs::rename(temp_source_file_path.clone(), source_file_path.clone()) {
            println!("{}: {}", temp_source_file_path.to_str().unwrap(), err);
            continue;
        }

        println!("Restored {}", source_file_path.to_str().unwrap());
    }
}

pub fn get_backed_up_file_paths(
    backup_file_pattern: BackupFilePattern, backup_dest_path: PathBuf
) -> Result<Vec<PathBuf>, String> {

    // 1. Create an absolute backed up file pattern

    let backup_file_pattern_history_pattern = backup_file_pattern.file_pattern + ".*";
    let backup_file_folder_name = backup_file_pattern.source_dir.file_name().unwrap();
    let backed_up_pattern = backup_dest_path
        .join(backup_file_folder_name)
        .join(backup_file_pattern_history_pattern);

    // 2. Get a list of all files matching the pattern

    let glob_paths = match glob(backed_up_pattern.to_str().unwrap()) {
        Err(err) => {
            return Err(format!("Error scanning backed up files for {}: {}", backed_up_pattern.to_str().unwrap(), err));
        }
        Ok(glob_paths) =>
            glob_paths
    };

    // 3. Convert matching paths into a path vector

    let mut backed_up_file_paths = vec![];
    for glob_path in glob_paths {
        let glob_path = match glob_path {
            Err(err) =>
                return Err(format!("Error scanning backed up files for {}: {}", backed_up_pattern.to_str().unwrap(), err)),
            Ok(glob_path) =>
                backed_up_file_paths.push(glob_path)
        };
    }

    Ok(backed_up_file_paths)
}

fn get_source_file_for_backed_up_file(settings: Settings, backed_up_file: PathBuf) -> Result<PathBuf, String> {
    let backed_up_file_folder_name = backed_up_file.parent().unwrap().file_name().unwrap();

    // Strip off the numeric suffix from the backed up filename
    let backed_up_file_name = backed_up_file.file_name().unwrap().to_str().unwrap();
    let backed_up_file_name_index = match backed_up_file_name.rfind(".") {
        Some(i) => i,
        None =>
            return Err(format!("invalid backed up file name: {}", backed_up_file_name))
    };
    let stripped_backed_up_file_name = &backed_up_file_name[..backed_up_file_name_index];

    for backup_file_pattern in settings.backup_paths {
        let backup_file_pattern_path = backup_file_pattern.to_path();
        let backup_file_pattern_folder_name = backup_file_pattern_path.parent().unwrap().file_name().unwrap();

        if backup_file_pattern_folder_name == backed_up_file_folder_name {
            let file_pattern = match Pattern::new(backup_file_pattern_path.to_str().unwrap()) {
                Ok(pattern) => pattern,
                Err(err) =>
                    return Err(format!("invalid file pattern \"{}\": {}", backup_file_pattern_path.to_str().unwrap(), err))
            };

            // The file name of the backed up file grafted onto the source path
            let source_path = backup_file_pattern_path.parent().unwrap()
                .join(stripped_backed_up_file_name);

            if file_pattern.matches_path(&source_path) {
                return Ok(source_path);
            }
        }
    }

    Err(format!("Failed to find source file for backed up file {}", backed_up_file.to_str().unwrap()))
}

pub fn get_file_metadata(file_path: PathBuf ) -> Result<(Metadata, SystemTime), String> {
    match file_path.metadata() {
        Err(err) => {
            let error_msg =
                format!("Cannot read metadata for changed file {}: {}",
                    file_path.to_str().unwrap(), err
                ).to_string();
            return Err(error_msg);
        }
        Ok(metadata) => {
            match metadata.modified() {
                Err(err) => {
                    let error_msg =
                        format!("Cannot read metadata for changed file {}: {}",
                            file_path.to_str().unwrap(), err
                        ).to_string();
                    return Err(error_msg);
                }
                Ok(modified) => {
                    return Ok((metadata, modified))
                }
            }
        }
    }
}