use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Error;
use fltk::app;
use log::{debug, error};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

use FileError::{FError, FWarning};

use crate::{FileError, MainState, UiMessage};
use crate::file::{back_up_live_file, delete_old_backups, live_file_has_backup, PathExt};
use crate::settings::Settings;

const STOP_WATCHER_ERROR: &str = "STOP";

#[derive(Debug)]
pub enum BackupMessage {
    Run { settings: Settings },
    Stop {},
}

#[derive(Error, Debug)]
pub enum BackupStatus {
    Status(String),
    Error(String),
}

impl Display for BackupStatus {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub fn start_backup_thread(state: &mut MainState) {
    debug!("Starting backup thread");
    assert!(state.settings.is_some(), "illegal state");
    assert!(state.backup_thread.is_none(), "illegal state");

    let (backup_message_tx, backup_message_rx) = mpsc::channel();
    state.backup_thread_tx = Some(backup_message_tx.clone());
    let ui_thread_tx_copy = state.ui_thread_tx.clone();
    state.backup_thread = Some(
        std::thread::spawn(
            move || backup_thread_main(backup_message_rx, ui_thread_tx_copy))
    );

    if let Err(err) = state.backup_thread_tx.as_ref().unwrap().send(
        BackupMessage::Run {
            settings: state.settings.clone().unwrap()
        }
    ) {
        panic!("Error sending run message to backup thread: {}", err);
    }
}

pub fn stop_backup_thread(state: &mut MainState) -> JoinHandle<()> {
    debug!("Signaling backup thread to stop");
    assert!(state.backup_thread.is_some(), "illegal state");
    assert!(state.backup_thread_tx.is_some(), "illegal state");

    if let Err(err) = state.backup_thread_tx.as_ref().unwrap().send(BackupMessage::Stop {}) {
        panic!("Error sending stop message to backup thread: {}", err);
    }
    let mut backup_thread = None;
    std::mem::swap(&mut backup_thread, &mut state.backup_thread);
    backup_thread.unwrap()
}

fn backup_thread_main(
    backup_thread_rx: mpsc::Receiver<BackupMessage>,
    ui_thread_tx: app::Sender<UiMessage>
) {
    debug!("Backup thread started");
    let mut current_watcher = None;
    let mut current_watcher_thread: Option<JoinHandle<()>> = None;
    let mut current_watcher_thread_tx: Option<mpsc::Sender<DebouncedEvent>> = None;

    loop {
        match backup_thread_rx.recv() {
            Err(err) => {
                ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
                debug!("Backup thread stopped");
                // Drops current_watcher if it exists, which will drop watcher_thread_tx, which will return an error
                // from watcher_thread_rx.recv(), which will cause watcher_thread_main to return
                return;
            }
            Ok(msg) => {
                match msg {
                    BackupMessage::Stop {} => {
                        debug!("Stopping backup thread");
                        if current_watcher_thread_tx.is_some() {
                            if let Err(err) = current_watcher_thread_tx.unwrap().send(
                                DebouncedEvent::Error(
                                    notify::Error::Generic(STOP_WATCHER_ERROR.to_string()),
                                    None)
                            ) {
                                panic!("Error sending stop message to watcher thread: {}", err);
                            }
                        }
                        if current_watcher_thread.is_some() {
                            if let Err(err) = current_watcher_thread.unwrap().join() {
                                panic!("Panic from watcher thread: {:?}", err);
                            }
                        }
                        ui_thread_tx.send(UiMessage::SetStatus("Stopped".to_string()));
                        debug!("Backup thread stopped");
                        return;
                    }
                    BackupMessage::Run { settings } => {
                        debug!("Starting watcher thread");
                        assert!(current_watcher.is_none(), "illegal state");

                        let (watcher_thread_tx, watcher_thread_rx) = mpsc::channel();

                        current_watcher_thread_tx = Some(watcher_thread_tx.clone());

                        let new_watcher = Watcher::new(
                            watcher_thread_tx, Duration::from_secs(settings.backup_delay_sec as u64));

                        if let Err(err) = new_watcher {
                            ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
                            debug!("Backup thread stopped");
                            // Drops current_watcher if it exists, which will drop watcher_thread_tx, which will return
                            // an error from watcher_thread_rx.recv(), which will cause watcher_thread_main to return
                            return;
                        }
                        let mut new_watcher: RecommendedWatcher = new_watcher.unwrap();

                        //TODO dedup directories - multiple patterns will use the same source dir
                        for backup_pattern in &settings.backup_patterns {
                            if let Err(err) = new_watcher.watch(&backup_pattern.source_dir, RecursiveMode::NonRecursive) {
                                panic!("Error watching directory {}: {}", backup_pattern.source_dir.str(), err);
                            }
                            debug!("Watching {} for {}",
                                backup_pattern.source_dir.str(),
                                backup_pattern.filename_pattern.as_str()
                            );
                        }

                        let ui_thread_tx_copy = ui_thread_tx.clone();
                        current_watcher_thread = Some(
                            std::thread::spawn(
                                move || watcher_thread_main(settings, watcher_thread_rx, ui_thread_tx_copy))
                        );

                        current_watcher = Some(new_watcher);
                        ui_thread_tx.send(UiMessage::SetStatus("Running".to_string()));
                    }
                }
            }
        }
    }
}

fn watcher_thread_main(settings: Settings, watcher_thread_rx: mpsc::Receiver<DebouncedEvent>, ui_thread_tx: app::Sender<UiMessage>) {
    debug!("Watcher thread started");
    loop {
        match watcher_thread_rx.recv() {
            Err(err) => {
                panic!("Watcher error: {}", err);
            }
            Ok(file_event) => {
                match file_event {
                    DebouncedEvent::Create(file_path)
                    | DebouncedEvent::Write(file_path) => {
                        on_file_change(file_path, &settings, ui_thread_tx.clone());
                    }
                    DebouncedEvent::Error(err, path) => {
                        match err {
                            notify::Error::Generic(err_msg) => {
                                if err_msg == STOP_WATCHER_ERROR.to_string() {
                                    debug!("Watcher thread stopped");
                                    return;
                                } else {
                                    error!("Watcher error for {:?}: {}", path, err_msg);
                                }
                            }
                            notify::Error::Io(err) => {
                                error!("Watcher IO error for {:?}: {}", path, err);
                            }
                            notify::Error::PathNotFound => {
                                error!("Watcher path not found error for {:?}", path);
                            }
                            notify::Error::WatchNotFound => {
                                error!("Watcher watch not found error for {:?}", path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn on_file_change( backup_file_path: PathBuf, settings: &Settings, ui_thread_tx: app::Sender<UiMessage> ) {
    let file_has_backup = match live_file_has_backup(settings.clone(), backup_file_path.clone()) {
        Ok(has_backup) => has_backup,
        Err(err) => {
            handle_error(&ui_thread_tx, &err);
            return;
        }
    };
    if !file_has_backup {
        if let Err(err) = back_up_live_file(settings.clone(), backup_file_path.clone()) {
            handle_error(&ui_thread_tx, &err);
        }
        if let Err(err) = delete_old_backups(settings.clone()) {
            handle_error(&ui_thread_tx, &err);
        }
        ui_thread_tx.send(UiMessage::RefreshFilesLists);
    }
}

fn handle_error(ui_thread_tx: &app::Sender<UiMessage>, err: &Error) {
    if let Some(file_err) = err.downcast_ref::<FileError>() {
        match file_err {
            FWarning(errs) => {
                errs.iter().for_each(|err_msg| ui_thread_tx.send(UiMessage::Alert(err_msg.clone())));
            }
            FError(errs) => {
                errs.iter().for_each(|err_msg| ui_thread_tx.send(UiMessage::AlertQuit(err_msg.clone())));
            }
        }
    } else {
        ui_thread_tx.send(UiMessage::AlertQuit(err.to_string()));
    }
}
