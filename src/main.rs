use std::error::Error;
use std::path::PathBuf;
use std::process::exit;
use std::sync::mpsc;
use std::sync::mpsc::RecvError;
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
use UiMessage::*;

use crate::settings::{get_default_settings, get_settings, Settings, SettingsError, write_settings};
use crate::watcher::{BackupMessage, BackupStatus, start_backup_thread, stop_backup_thread};

mod settings;
mod main_win;
mod settings_win;
mod win_common;
mod watcher;
mod file;

pub enum UiMessage {
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
    SetStatus(String),
}

impl Clone for UiMessage {
    fn clone(&self) -> Self {
        match self {
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
        }
    }
}

pub struct MainState {
    main_win: MainWindow,
    settings_win: SettingsWindow,
    settings: Option<Settings>,
    backup_thread: Option<JoinHandle<()>>,
    backup_thread_tx: Option<mpsc::Sender<BackupMessage>>,
    backup_thread_rx: Option<mpsc::Receiver<BackupStatus>>,
    ui_thread_tx: app::Sender<UiMessage>,
}

fn main() {
    let app = app::App::default();

    let (ui_thread_tx, ui_thread_rx) = app::channel::<UiMessage>();

    let mut state = MainState {
        main_win: MainWindow::new(ui_thread_tx.clone()),
        settings_win: SettingsWindow::new(ui_thread_tx.clone()),
        settings: None,
        backup_thread: None,
        backup_thread_tx: None,
        backup_thread_rx: None,
        ui_thread_tx,
    };

    state.main_win.wind.show();

    match get_settings() {
        Ok(settings) => {
            state.settings = Some(settings.clone());
            //? state.settings_win.set_settings_to_win(settings);
            start_backup_thread(&mut state);
        }
        Err(SettingsError::SError(err_msg)) => {
            fatal_error(err_msg);
        }
        Err(SettingsError::SWarning(settings, msg)) => {
            state.settings = Some(settings.clone());
            state.settings_win.set_settings_to_win(settings);
            state.settings_win.wind.show();
            message_default(&msg);
        }
        Err(SettingsError::SNotFound(Some(settings))) => {
            // A settings file was just created with defaults and needs to be adjusted by the user
            state.settings = Some(settings.clone());
            state.settings_win.set_settings_to_win(settings);
            state.settings_win.wind.show();
        }
        _ =>
            panic!("illegal state")
    }

    while app.wait() {
        if let Some(msg) = ui_thread_rx.recv() {
            match msg {
                MenuSettings => {
                    stop_backup_thread(&mut state, false);
                    state.settings_win.wind.make_modal(true);
                    state.settings_win.wind.show();
                }
                AppQuit
                | MenuQuit => {
                    stop_backup_thread(&mut state, false);
                    app::quit();
                    exit(0);
                }
                MenuDocumentation => {
                    todo!();
                }
                MenuAbout => {
                    todo!();
                }
                SettingsBackupDestChoose => {
                    state.settings_win.choose_backup_dest_dir();
                }
                SettingsOk => {
                    let settings = state.settings_win.get_settings_from_win();
                    match settings::validate_settings(settings) {
                        Ok(settings) => {
                            write_settings(settings);
                            state.settings_win.wind.hide();
                            start_backup_thread(&mut state);
                        }
                        Err(err) => {
                            match err {
                                SettingsError::SWarning(settings, err_msg) => {
                                    alert_default(&err_msg);
                                }
                                SettingsError::SError(err_msg) => {
                                    fatal_error(err_msg);
                                }
                                _ =>
                                    panic!("illegal state")
                            }
                        }
                    }
                }
                SettingsQuit => {
                    stop_backup_thread(&mut state, false);
                    app::quit();
                    exit(0);
                }
                RestoreBackup => {
                    println!("Restore Backup");
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
                SetStatus(status) => {
                    state.main_win.set_status(status);
                }
            }
        }
    }
}

fn fatal_error(err_msg: String) {
    let err_msg = err_msg + "\nFatal error - Valbak must close";
    alert_default(&err_msg);
    app::quit();
    exit(1);
}