use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvError};
use std::thread::JoinHandle;
use std::time::Duration;

use fltk::prelude::WidgetExt;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

use crate::{MainState};
use crate::settings::Settings;

#[derive(Debug)]
pub enum BackupMessage {
    Run { settings: Settings },
    Stop,
}

#[derive(Error, Debug)]
pub enum BackupError {
    Unknown(String),
}

impl Display for BackupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub type BackupResult = Result<(), BackupError>;

pub fn start_backup_thread(main_state: &mut MainState) {
    println!("Starting backup thread");
    assert!(main_state.settings.is_some(), "illegal state");
    assert!(main_state.backup_thread.is_none(), "illegal state");

    let (backup_message_tx, backup_message_rx) = mpsc::channel();
    let (backup_result_tx, backup_result_rx) = mpsc::channel();
    main_state.backup_thread_tx = Some(backup_message_tx.clone());
    main_state.backup_thread_rx = Some(backup_result_rx);
    main_state.backup_thread = Some(
        std::thread::spawn(move || backup_thread_main(backup_result_tx, backup_message_rx))
    );

    main_state.backup_thread_tx.as_ref().unwrap().send(
        BackupMessage::Run {
            settings: main_state.settings.clone().unwrap()
        });
}

pub fn stop_backup_thread(main_state: &mut MainState) {
    println!("Stopping backup thread");
    assert!(main_state.backup_thread.is_some(), "illegal state");
    assert!(main_state.backup_thread_tx.is_some(), "illegal state");

    main_state.backup_thread_tx.as_ref().unwrap().send(BackupMessage::Stop);
    let mut backup_thread = None;
    std::mem::swap(&mut backup_thread, &mut main_state.backup_thread);
    backup_thread.unwrap().join();
    println!("Backup thread stopped");
}

fn backup_thread_main(backup_result_tx: mpsc::Sender<BackupResult>, backup_message_rx: mpsc::Receiver<BackupMessage>) {
    let mut current_watcher = None;
    loop {
        match backup_message_rx.recv() {
            Err(err) => {
                println!("Backup thread done");
                // Drops current_watcher if it exists, which will drop notify_tx, which will return an error
                // from notify_rx.recv(), which will cause watcher_thread_main to return
                return;
            }
            Ok(msg) => {
                println!("Thread received message: {:?}", msg);
                match msg {
                    BackupMessage::Run { settings } => {
                        assert!(current_watcher.is_none(), "illegal state");

                        let (notify_tx, notify_rx) = mpsc::channel();

                        let new_watcher = Watcher::new(
                            notify_tx, Duration::from_secs(settings.backup_delay_sec as u64));
                        if let Err(err) = new_watcher {
                            let send_result = backup_result_tx.send(
                                Err(BackupError::Unknown(err.to_string())));
                            if let Err(err) = send_result {
                                panic!("Failed to send error to main thread: {}", err);
                            }
                            return;
                        }
                        let mut new_watcher: RecommendedWatcher = new_watcher.unwrap();

                        for backup_file_pattern in &settings.backup_paths {
                            new_watcher.watch(&backup_file_pattern.source_dir, RecursiveMode::NonRecursive);
                        }

                        std::thread::spawn(move || watcher_thread_main(settings, notify_rx));

                        current_watcher = Some(new_watcher);
                    }
                    BackupMessage::Stop => {
                        // Drops current_watcher if it exists, which will drop notify_tx, which will return an error
                        // from notify_rx.recv(), which will cause watcher_thread_main to return
                        return;
                    }
                }
            }
        }
    }
}

fn watcher_thread_main(settings: Settings, notify_rx: Receiver<DebouncedEvent>) {
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
                        backup_file(settings.clone(), file_path);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn backup_file(settings: Settings, file_path: PathBuf) {
    println!("Backing up {}", file_path.to_str().unwrap());
}