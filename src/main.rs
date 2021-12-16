use std::borrow::Borrow;
use std::cell::RefCell;
use std::error::Error;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::{Arc, mpsc, Mutex, MutexGuard, TryLockResult};
use std::sync::mpsc::{Receiver, RecvError};
use std::thread::JoinHandle;
use std::time::Duration;

use fltk::app;
use fltk::dialog::{alert_default, FileChooser, FileChooserType, message_default};
use fltk::enums::Event;
use fltk::prelude::{WidgetExt, WindowExt};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

use main_win::MainWindow;
use settings_win::SettingsWindow;
use SettingsError::{SError, SWarning};
use UiMessage::*;

use crate::file::{backup_all_changed_files, clean_backups, get_backed_up_files, get_live_files};
use crate::settings::{get_default_settings, get_settings, Settings, SettingsError, write_settings};
use crate::settings_win::SettingsWinError;
use crate::watcher::{BackupMessage, BackupStatus, start_backup_thread, stop_backup_thread};

mod settings;
mod main_win;
mod settings_win;
mod win_common;
mod watcher;
mod file;

pub enum UiMessage {
    AlertQuit(String),
    AppQuit,
    MenuSettings,
    MenuQuit,
    MenuDocumentation,
    MenuAbout,
    SettingsBackupDestChoose,
    SettingsOk,
    SettingsQuit,
    RestoreBackup,
    DeleteBackup,
    ActivateRedirect,
    DeactivateRedirect,
    PushStatus(String),
    PopStatus,
    SetStatus(String),
    RefreshFilesLists,
}

impl Clone for UiMessage {
    fn clone(&self) -> Self {
        match self {
            AlertQuit(alert_msg) => AlertQuit(alert_msg.clone()),
            AppQuit => AppQuit,
            MenuSettings => MenuSettings,
            MenuQuit => MenuQuit,
            MenuDocumentation => MenuDocumentation,
            MenuAbout => MenuAbout,
            SettingsBackupDestChoose => SettingsBackupDestChoose,
            SettingsOk => SettingsOk,
            SettingsQuit => SettingsQuit,
            RestoreBackup => RestoreBackup,
            DeleteBackup => DeleteBackup,
            ActivateRedirect => ActivateRedirect,
            DeactivateRedirect => DeactivateRedirect,
            SetStatus(status) => SetStatus(status.clone()),
            PushStatus(status) => PushStatus(status.clone()),
            PopStatus => PopStatus,
            RefreshFilesLists => RefreshFilesLists,
        }
    }
}

pub struct MainState {
    main_win: MainWindow,
    settings_win: Option<SettingsWindow>,
    settings: Option<Settings>,
    backup_thread: Option<JoinHandle<()>>,
    backup_thread_tx: Option<mpsc::Sender<BackupMessage>>,
    backup_thread_rx: Option<mpsc::Receiver<BackupStatus>>,
    ui_thread_tx: app::Sender<UiMessage>,
}

fn main() {
    let app = app::App::default();

    let (ui_thread_tx, ui_thread_rx) = app::channel::<UiMessage>();

    let mut main_state = Arc::new(Mutex::new(
        MainState {
            main_win: MainWindow::new(ui_thread_tx.clone()),
            settings_win: None,
            settings: None,
            backup_thread: None,
            backup_thread_tx: None,
            backup_thread_rx: None,
            ui_thread_tx: ui_thread_tx.clone(),
        }));
    let mut state = main_state.lock().unwrap();

    state.main_win.wind.show();

    match get_settings() {
        Ok(settings) => {
            // Settings loaded without error
            state.settings = Some(settings);
            start_backup_thread(&mut state);
        }
        Err(SError(err_msg)) => {
            // Settings could not be loaded
            drop(state);
            fatal_error(main_state.clone(), err_msg);
        }
        Err(SWarning(settings, warn_msg)) => {
            // Settings loaded with error
            state.settings = Some(settings.clone());
            let mut settings_win = SettingsWindow::new(state.ui_thread_tx.clone());
            settings_win.set_settings_to_win(settings);
            settings_win.wind.show();
            state.settings_win = Some(settings_win);
            if !warn_msg.is_empty() {
                message_default(&warn_msg);
            }
        }
        Err(SettingsError::SNotFound(Some(settings))) => {
            // A settings file was just created with defaults and needs to be adjusted by the user
            state.settings = Some(settings.clone());
            let mut settings_win = SettingsWindow::new(state.ui_thread_tx.clone());
            settings_win.set_settings_to_win(settings);
            settings_win.wind.show();
            state.settings_win = Some(settings_win);
        }
        _ =>
            panic!("illegal state")
    }
    println!("Got settings");

    if state.settings.is_some() {
        ui_thread_tx.send(UiMessage::RefreshFilesLists);
    }

    // Release the lock
    drop(state);

    // Apparently sending UI messages from the main UI loop is unreliable
    let mut internal_message_queue = Vec::new();

    let mut quitting = false;
    // wait() blocks until a message is ready for ui_thread_rx.recv()
    while !internal_message_queue.is_empty() || app.wait() {
        let mut ui_msg = internal_message_queue.pop();
        if ui_msg.is_none() {
            if let Some(msg) = ui_thread_rx.recv() {
                ui_msg = Some(msg);
            }
        }
        if let Some(ui_msg) = ui_msg {
            if quitting {
                // Ignore most messages
                match ui_msg {
                    PushStatus(_) => {}
                    PopStatus => {}
                    SetStatus(_) => {}
                    _ => {
                        println!("Quitting - and ignoring message");
                        continue;
                    }
                }
                println!("Quitting - and allowing message");
            }
            match ui_msg {
                MenuSettings => {
                    let mut state = main_state.lock().unwrap();
                    assert!(state.settings.is_some(), "illegal state");
                    // non-blocking call
                    stop_backup_thread(&mut state);
                    let mut settings_win = SettingsWindow::new(state.ui_thread_tx.clone());
                    settings_win.set_settings_to_win(state.settings.as_ref().unwrap().clone());
                    settings_win.wind.make_modal(true);
                    // Note: Apparently only the UI thread can show windows
                    settings_win.wind.show();
                    state.settings_win = Some(settings_win);
                }
                MenuDocumentation => {
                    todo!();
                }
                MenuAbout => {
                    todo!();
                }
                SettingsBackupDestChoose => {
                    let mut state = main_state.lock().unwrap();
                    assert!(state.settings_win.is_some(), "illegal state");
                    assert!(state.settings.is_some(), "illegal state");
                    let settings = state.settings.as_ref().unwrap().clone();
                    // Shows a file chooser window/dialog and blocks
                    state.settings_win.as_mut().unwrap().choose_backup_dest_dir(settings);
                }
                SettingsOk => {
                    let mut state = main_state.lock().unwrap();
                    assert!(state.settings_win.is_some(), "illegal state");
                    match state.settings_win.as_ref().unwrap().get_settings_from_win() {
                        Ok(settings) => {
                            match settings::validate_settings(settings) {
                                Ok(settings) => {
                                    state.settings = Some(settings.clone());
                                    write_settings(settings.clone());
                                    state.settings_win.as_mut().unwrap().wind.hide();
                                    state.settings_win = None;
                                    start_backup_thread(&mut state);
                                    backup_all_changed_files(settings.clone());
                                    clean_backups(settings);
                                    internal_message_queue.push(UiMessage::RefreshFilesLists);
                                }
                                Err(err) => {
                                    match err {
                                        SWarning(_settings, err_msg) => {
                                            if !err_msg.is_empty() {
                                                alert_default(&err_msg);
                                            }
                                        }
                                        SError(err_msg) => {
                                            drop(state);
                                            fatal_error(main_state.clone(), err_msg);
                                        }
                                        _ =>
                                            panic!("illegal state")
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            match err {
                                SettingsWinError::SwWarning(err_msg) => {
                                    alert_default(&err_msg);
                                }
                                SettingsWinError::SwError(err_msg) => {
                                    fatal_error(main_state.clone(), err_msg);
                                }
                            }
                        }
                    };
                }
                AlertQuit(alert_msg) => {
                    quitting = true;
                    alert_default(&alert_msg);
                    start_graceful_quit(main_state.clone(), 1);
                }
                AppQuit
                | MenuQuit
                | SettingsQuit => {
                    quitting = true;
                    start_graceful_quit(main_state.clone(), 0);
                }
                RestoreBackup => {
                    let mut state = main_state.lock().unwrap();
                    let selected_backup_paths = state.main_win.get_selected_backed_up_paths();
                    if !selected_backup_paths.is_empty() {
                        //TODO show confirmation dialog
                        assert!(state.settings.is_some(), "illegal state");
                        file::restore_backed_up_files(state.settings.as_ref().unwrap().clone(), selected_backup_paths);
                    }
                    internal_message_queue.push(UiMessage::RefreshFilesLists);
                }
                DeleteBackup => {
                    println!("Delete Backup");
                }
                ActivateRedirect => {
                    println!("Activate Redirect");
                }
                DeactivateRedirect => {
                    println!("Deactivate Redirect");
                }
                PushStatus(status) => {
                    println!("Pushing status message to: {}", &status);
                    let mut state = main_state.lock().unwrap();
                    state.main_win.push_status(status);
                }
                PopStatus => {
                    println!("Popping status message");
                    let mut state = main_state.lock().unwrap();
                    state.main_win.pop_status();
                }
                SetStatus(status) => {
                    println!("Setting status message to: {}", &status);
                    let mut state = main_state.lock().unwrap();
                    state.main_win.set_status(status);
                },
                RefreshFilesLists => {
                    let mut state = main_state.lock().unwrap();
                    let live_files = get_live_files(state.settings.as_ref().unwrap().clone());
                    state.main_win.set_live_files_to_win(live_files);

                    let backed_up_files = get_backed_up_files(state.settings.as_ref().unwrap().clone());
                    state.main_win.set_backed_up_files_to_win(backed_up_files);
                }
            }
        }
    }
}

fn fatal_error(main_state: Arc<Mutex<MainState>>, err_msg: String) -> ! {
    let err_msg = err_msg + "\nFatal error - Valbak must close";
    // blocks until user dismisses the alert box
    alert_default(&err_msg);
    let exit_thread = start_graceful_quit(main_state, 1);
    exit_thread.join();
    // The exit thread should terminate the app before this occurs
    exit(1);
}

fn start_graceful_quit(mut main_state: Arc<Mutex<MainState>>, exit_code: i32) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut state = match main_state.try_lock() {
            Ok(lock) =>
                lock,
            Err(err) =>
                panic!("illegal state - main state lock not released")
        };
        if state.backup_thread.is_some() {
            let backup_thread = stop_backup_thread(&mut state);
            drop(state);
            backup_thread.join();
        }
        exit(exit_code);
    })
}