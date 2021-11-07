use std::io::ErrorKind;
use std::path::PathBuf;

use fltk::app;

use crate::settings::Settings;
use crate::UiMessage;

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