use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use fltk::app;
use fltk::prelude::WidgetExt;
use glob::{GlobResult, Paths, Pattern, PatternError};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

use crate::{MainState, UiMessage};
use crate::file::backup_file;
use crate::settings::Settings;

#[derive(Debug)]
pub enum BackupMessage {
    Run { settings: Settings },
    Stop { update_status: bool },
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

pub fn start_backup_thread(main_state: &mut MainState) {
    println!("Starting backup thread");
    assert!(main_state.settings.is_some(), "illegal state");
    assert!(main_state.backup_thread.is_none(), "illegal state");

    let (backup_message_tx, backup_message_rx) = mpsc::channel();
    main_state.backup_thread_tx = Some(backup_message_tx.clone());
    let ui_thread_tx = main_state.ui_thread_tx.clone();
    main_state.backup_thread = Some(
        std::thread::spawn(
            move || backup_thread_main(backup_message_rx, ui_thread_tx))
    );

    main_state.backup_thread_tx.as_ref().unwrap().send(
        BackupMessage::Run {
            settings: main_state.settings.clone().unwrap()
        });
}

pub fn stop_backup_thread(main_state: &mut MainState, update_status: bool) {
    println!("Stopping backup thread");
    assert!(main_state.backup_thread.is_some(), "illegal state");
    assert!(main_state.backup_thread_tx.is_some(), "illegal state");

    main_state.backup_thread_tx.as_ref().unwrap().send(BackupMessage::Stop { update_status });
    let mut backup_thread = None;
    std::mem::swap(&mut backup_thread, &mut main_state.backup_thread);
    backup_thread.unwrap().join();
    println!("Backup thread stopped");
}

fn backup_thread_main(
    backup_message_rx: mpsc::Receiver<BackupMessage>,
    ui_thread_tx: app::Sender<UiMessage>
) {
    ui_thread_tx.send(UiMessage::SetStatus("Waiting".to_string()));
    let mut current_watcher = None;
    loop {
        match backup_message_rx.recv() {
            Err(err) => {
                ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
                // Drops current_watcher if it exists, which will drop notify_tx, which will return an error
                // from notify_rx.recv(), which will cause watcher_thread_main to return
                return;
            }
            Ok(msg) => {
                match msg {
                    BackupMessage::Run { settings } => {
                        assert!(current_watcher.is_none(), "illegal state");

                        let (notify_tx, notify_rx) = mpsc::channel();

                        let new_watcher = Watcher::new(
                            notify_tx, Duration::from_secs(settings.backup_delay_sec as u64));
                        if let Err(err) = new_watcher {
                            ui_thread_tx.send(UiMessage::SetStatus(format!("Error: {}", err)));
                            return;
                        }
                        let mut new_watcher: RecommendedWatcher = new_watcher.unwrap();

                        for backup_file_pattern in &settings.backup_paths {
                            new_watcher.watch(&backup_file_pattern.source_dir, RecursiveMode::NonRecursive);
                        }

                        std::thread::spawn(move || watcher_thread_main(settings, notify_rx));

                        current_watcher = Some(new_watcher);
                        ui_thread_tx.send(UiMessage::SetStatus("Running".to_string()));
                    }
                    BackupMessage::Stop { update_status } => {
                        println!(">>1");
                        if update_status {
                            ui_thread_tx.send(UiMessage::SetStatus("Stopped".to_string()));
                        }
                        println!(">>2");
                        // Drops current_watcher if it exists, which will drop notify_tx, which will return an error
                        // from notify_rx.recv(), which will cause watcher_thread_main to return
                        return;
                    }
                }
            }
        }
    }
}

fn watcher_thread_main(settings: Settings, notify_rx: mpsc::Receiver<DebouncedEvent>) {
    loop {
        match notify_rx.recv() {
            Err(err) => {
                println!("Watcher thread done");
                return;
            }
            Ok(file_event) => {
                match file_event {
                    DebouncedEvent::Create(file_path)
                    | DebouncedEvent::Write(file_path) => {
                        for backup_file_pattern in &settings.backup_paths {
                            match Pattern::new(backup_file_pattern.file_pattern.as_str()) {
                                Err(err) =>
                                    // This should have been caught previously
                                    println!("internal error: {}", err),
                                Ok(file_pattern) =>
                                    if file_pattern.matches_path(&file_path) {
                                        backup_file(settings.clone(), file_path.clone());
                                    }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}