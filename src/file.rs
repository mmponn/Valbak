use core::cmp::Ordering;
use std::io::ErrorKind;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};

use fltk::app;
use glob::{glob, GlobResult, Paths, PatternError};

use crate::settings::Settings;
use crate::UiMessage;

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
    let mut backup_files = Vec::new();
    for backup_file_pattern in settings.backup_paths {
        let backup_file_pattern_history_pattern = backup_file_pattern.file_pattern + ".*";
        let backup_file_folder_name = backup_file_pattern.source_dir.file_name().unwrap();
        let backup_dest_pattern = settings.backup_dest_path
            .join(backup_file_folder_name)
            .join(backup_file_pattern_history_pattern);
        let glob_paths = match glob(backup_dest_pattern.to_str().unwrap()) {
            Err(err) => {
                println!("Error scanning backed up files for {}: {}", backup_dest_pattern.to_str().unwrap(), err);
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
                    match get_history_file_number(&file_path) {
                        None => {}
                        Some(_) => backup_files.push(file_path)
                    }
                }
            }
        }
    }
    backup_files
}

pub fn backup_file(settings: Settings, backup_file_from: PathBuf, ui_thread_tx: app::Sender<UiMessage>) {
    // Copy the file and its containing folder name
    let backup_file_from_folder = backup_file_from.parent().unwrap().file_name().unwrap();

    let backup_file_to_folder = settings.backup_dest_path.join(backup_file_from_folder);
    if let Err(err) = std::fs::create_dir(backup_file_to_folder.clone()) {
        if err.kind() != ErrorKind::AlreadyExists {
            println!("Error copying file: {}", err);
            return;
        }
    }
    let backup_file_to = backup_file_to_folder.join(backup_file_from.file_name().unwrap());
    let backup_file_to = get_next_backup_filename(&settings, backup_file_to);

    println!("Copying {} to {}", backup_file_from.to_str().unwrap(), backup_file_to.to_str().unwrap());

    // fltk send() appears to be a blocking call so we must release the main state lock before calling it
    ui_thread_tx.send(UiMessage::PushStatus(format!("Copying {}", backup_file_from.to_str().unwrap())));
    if let Err(err) = std::fs::copy(backup_file_from, backup_file_to) {
        println!("Error copying file: {}", err);
        ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
        return;
    }
    ui_thread_tx.send(UiMessage::PopStatus);
}

fn get_next_backup_filename(settings: &Settings, backup_file_to: PathBuf) -> PathBuf {
    let backup_file_to_folder = backup_file_to.parent().unwrap();
    let backup_file_to_filename = backup_file_to.file_name().unwrap();
    let backup_file_history_pattern = backup_file_to_folder
        .join(backup_file_to_filename.to_str().unwrap().to_string() + ".*");
    let backup_file_history = match glob(backup_file_history_pattern.to_str().unwrap()) {
        Err(err) => {
            println!("Error scanning backed up files for {}: {}", backup_file_history_pattern.to_str().unwrap(), err);
            None
        }
        Ok(history_paths) =>
            Some(history_paths)
    };
    let last_backup_file_history_number = match backup_file_history {
        None =>
            0u32,
        Some(history_paths) => {
            let mut history_number = 0u32;
            for history_path in history_paths {
                match history_path {
                    Err(err) => {
                        println!("Error reading backed up history files: {}", err);
                        continue;
                    }
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
                }
            }
            history_number
        }
    };
    backup_file_to_folder.join(format!("{}.{}",
        backup_file_to_filename.to_str().unwrap(),
        last_backup_file_history_number + 1
    ))
}

fn get_history_file_number(history_file: &PathBuf) -> Option<u32> {
    let history_filename = history_file.file_name().unwrap().to_str().unwrap();
    match history_filename.rfind(".") {
        None =>
            None,
        Some(dot_index) => {
            let history_filename_prefix = &history_filename[0..dot_index];
            let history_filename_suffix = &history_filename[dot_index + 1..];
            match history_filename_suffix.parse::<u32>() {
                Err(_) => None,
                Ok(history_num) => Some(history_num)
            }
        }
    }
}

pub fn clean_backups(settings: Settings) {
    for backup_file_pattern in settings.backup_paths {
        let backup_file_folder_name = backup_file_pattern.source_dir.file_name().unwrap().to_str().unwrap();
        let backup_file_history_pattern = settings.backup_dest_path
            .join(backup_file_folder_name)
            .join(backup_file_pattern.file_pattern + ".*");
        let backup_file_history = match glob(backup_file_history_pattern.to_str().unwrap()) {
            Err(err) => {
                println!("Error scanning backed up files for {}: {}", backup_file_history_pattern.to_str().unwrap(), err);
                None
            }
            Ok(history_paths) =>
                Some(history_paths)
        };
        let mut backup_file_history_paths = match backup_file_history {
            None =>
                Vec::new(),
            Some(history_paths) => {
                let mut backup_file_history_paths = Vec::new();
                for history_path in history_paths {
                    match history_path {
                        Err(err) => {
                            println!("Error reading backed up history files: {}", err);
                            continue;
                        }
                        Ok(history_path) => {
                            if get_history_file_number(&history_path).is_some() {
                                backup_file_history_paths.push(history_path)
                            }
                        }
                    }
                }
                backup_file_history_paths
            }
        };
        if backup_file_history_paths.len() > settings.backup_count as usize {
            // Reverse sort
            backup_file_history_paths.sort_by(
                |a, b| {
                    let a_num = get_history_file_number(a).unwrap();
                    let b_num = get_history_file_number(b).unwrap();
                    b_num.cmp(&a_num)
                });
            let old_history_paths = &backup_file_history_paths[settings.backup_count as usize..];
            for old_history_path in old_history_paths {
                println!("Removing old backup file: {}", old_history_path.to_str().unwrap());
                if let Err(err) = std::fs::remove_file(old_history_path) {
                    println!("Failed to remove old backup file at {}: {}", old_history_path.to_str().unwrap(), err);
                }
            }
        }
    }
}