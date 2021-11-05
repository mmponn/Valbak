use std::io::ErrorKind;
use std::path::PathBuf;

use crate::settings::Settings;

pub fn backup_file(settings: Settings, backup_file_path: PathBuf) {
    // Copy the file and its containing folder name
    let file_parent_folder = backup_file_path.parent().unwrap().file_name().unwrap();
    let new_file_folder = settings.backup_dest_path.join(file_parent_folder);
    if let Err(err) = std::fs::create_dir(new_file_folder.clone()) {
        if err.kind() != ErrorKind::AlreadyExists {
            println!("Error copying file: {}", err);
            return;
        }
    }
    let new_file_path = new_file_folder.join(backup_file_path.file_name().unwrap());
    println!("Copying {} to {}", backup_file_path.to_str().unwrap(), new_file_path.to_str().unwrap());
    if let Err(err) = std::fs::copy(backup_file_path, new_file_path) {
        println!("Error copying file: {}", err);
        return;
    }
}