use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::Metadata;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, mpsc, Mutex, MutexGuard};
use std::sync::mpsc::RecvTimeoutError;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local};
use fltk::app;
use fltk::prelude::WidgetExt;
use glob::{GlobResult, Paths, Pattern, PatternError};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

use crate::{AlertQuit, MainState, PushStatus, SetStatus, UiMessage};
use crate::file::{backup_file, clean_backups, file_has_backup, get_backed_up_file_paths, get_file_metadata};
use crate::settings::{BackupFilePattern, Settings};

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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub fn start_backup_thread(state: &mut MainState) {
    println!("Starting backup thread");
    assert!(state.settings.is_some(), "illegal state");
    assert!(state.backup_thread.is_none(), "illegal state");

    let (backup_message_tx, backup_message_rx) = mpsc::channel();
    state.backup_thread_tx = Some(backup_message_tx.clone());
    let ui_thread_tx_copy = state.ui_thread_tx.clone();
    state.backup_thread = Some(
        std::thread::spawn(
            move || backup_thread_main(backup_message_rx, ui_thread_tx_copy))
    );

    state.backup_thread_tx.as_ref().unwrap().send(
        BackupMessage::Run {
            settings: state.settings.clone().unwrap()
        });
}

pub fn stop_backup_thread(state: &mut MainState) -> JoinHandle<()> {
    println!("Signaling backup thread to stop");
    assert!(state.backup_thread.is_some(), "illegal state");
    assert!(state.backup_thread_tx.is_some(), "illegal state");

    state.backup_thread_tx.as_ref().unwrap().send(BackupMessage::Stop {});
    let mut backup_thread = None;
    std::mem::swap(&mut backup_thread, &mut state.backup_thread);
    backup_thread.unwrap()
}

fn backup_thread_main(
    backup_thread_rx: mpsc::Receiver<BackupMessage>,
    ui_thread_tx: app::Sender<UiMessage>
) {
    let mut current_watcher = None;
    let mut current_watcher_thread: Option<JoinHandle<()>> = None;
    let mut current_watcher_thread_tx: Option<mpsc::Sender<DebouncedEvent>> = None;

    loop {
        match backup_thread_rx.recv() {
            Err(err) => {
                ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
                println!("Backup thread stopped");
                // Drops current_watcher if it exists, which will drop watcher_thread_tx, which will return an error
                // from watcher_thread_rx.recv(), which will cause watcher_thread_main to return
                return;
            }
            Ok(msg) => {
                match msg {
                    BackupMessage::Stop {} => {
                        println!("Stopping backup thread");
                        if current_watcher_thread_tx.is_some() {
                            current_watcher_thread_tx.unwrap().send(
                                DebouncedEvent::Error(
                                    notify::Error::Generic(STOP_WATCHER_ERROR.to_string()),
                                    None));
                        }
                        if current_watcher_thread.is_some() {
                            current_watcher_thread.unwrap().join();
                        }
                        ui_thread_tx.send(UiMessage::SetStatus("Stopped".to_string()));
                        println!("Backup thread stopped");
                        return;
                    }
                    BackupMessage::Run { settings } => {
                        assert!(current_watcher.is_none(), "illegal state");

                        let (watcher_thread_tx, watcher_thread_rx) = mpsc::channel();

                        current_watcher_thread_tx = Some(watcher_thread_tx.clone());

                        let new_watcher = Watcher::new(
                            watcher_thread_tx, Duration::from_secs(settings.backup_delay_sec as u64));

                        if let Err(err) = new_watcher {
                            ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
                            println!("Backup thread stopped");
                            // Drops current_watcher if it exists, which will drop watcher_thread_tx, which will return
                            // an error from watcher_thread_rx.recv(), which will cause watcher_thread_main to return
                            return;
                        }
                        let mut new_watcher: RecommendedWatcher = new_watcher.unwrap();

                        //TODO dedup directories - multiple patterns will use the same source dir
                        for backup_file_pattern in &settings.backup_paths {
                            new_watcher.watch(&backup_file_pattern.source_dir, RecursiveMode::NonRecursive);
                            println!("Watching: {}", backup_file_pattern.source_dir.to_str().unwrap());
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
    println!("Watcher thread started");
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
                                    println!("Watcher thread stopped");
                                    return;
                                } else {
                                    println!("Watcher error for {:?}: {}", path, err_msg);
                                }
                            }
                            notify::Error::Io(err) => {
                                println!("Watcher IO error for {:?}: {}", path, err);
                            }
                            notify::Error::PathNotFound => {
                                println!("Watcher path not found error for {:?}", path);
                            }
                            notify::Error::WatchNotFound => {
                                println!("Watcher watch not found error for {:?}", path);
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
    let file_has_backup = match file_has_backup(settings.clone(), backup_file_path.clone()) {
        Ok(has_backup) => has_backup,
        Err(err) => {
            println!("{}: {}", backup_file_path.to_str().unwrap(), err);
            return;
        }
    };
    if !file_has_backup {
        backup_file(settings.clone(), backup_file_path.clone());
        clean_backups(settings.clone());
        ui_thread_tx.send(UiMessage::RefreshFilesLists);
    }
}