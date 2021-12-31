use std::path::Path;
use std::process::exit;
use std::sync::{Arc, mpsc, Mutex};
use std::thread::JoinHandle;

use anyhow::Error;
use file_rotate::{ContentLimit, FileRotate, suffix::CountSuffix};
use file_rotate::compression::Compression;
use fltk::app;
use fltk::dialog::{alert_default, choice_default, message_default};
use fltk::prelude::{WidgetExt, WindowExt};
use log::*;
use simplelog::{ColorChoice, CombinedLogger, ConfigBuilder, LevelFilter, TerminalMode, TermLogger, WriteLogger};

use FileError::{FError, FWarning};
use main_win::MainWindow;
use settings_win::SettingsWindow;
use SettingsError::{SError, SNotFound, SWarning};
use UiMessage::*;

use crate::file::{backup_all_changed_files, delete_backed_up_files, delete_old_backups, FileError, get_backed_up_files, get_live_files, PathExt, restore_backed_up_files};
use crate::settings::{get_settings, get_settings_file_path, Settings, SettingsError, write_settings};
use crate::settings_win::SettingsWinError;
use crate::watcher::{BackupMessage, BackupStatus, start_backup_thread, stop_backup_thread};

mod settings;
mod main_win;
mod settings_win;
mod win_common;
mod watcher;
mod file;

pub enum UiMessage {
    Alert(String),
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
    PushStatus(String),
    PopStatus,
    SetStatus(String),
    RefreshFilesLists,
}

impl Clone for UiMessage {
    fn clone(&self) -> Self {
        match self {
            Alert(alert_msg) => Alert(alert_msg.clone()),
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
            SetStatus(status) => SetStatus(status.clone()),
            PushStatus(status) => PushStatus(status.clone()),
            PopStatus => PopStatus,
            RefreshFilesLists => RefreshFilesLists,
        }
    }
}

impl ToString for UiMessage {
    fn to_string(&self) -> String {
        match self {
            Alert(alert_msg)         => format!("Alert({})", alert_msg),
            AlertQuit(alert_msg)     => format!("AlertQuit({})", alert_msg),
            AppQuit                  => "AppQuit".to_string(),
            MenuSettings             => "MenuSettings".to_string(),
            MenuQuit                 => "MenuQuit".to_string(),
            MenuDocumentation        => "MenuDocumentation".to_string(),
            MenuAbout                => "MenuAbout".to_string(),
            SettingsBackupDestChoose => "SettingsBackupDestChoose".to_string(),
            SettingsOk               => "SettingsOk".to_string(),
            SettingsQuit             => "SettingsQuit".to_string(),
            RestoreBackup            => "RestoreBackup".to_string(),
            DeleteBackup             => "DeleteBackup".to_string(),
            PushStatus(status)       => format!("PushStatus({})", status),
            PopStatus                => "PopStatus".to_string(),
            SetStatus(status)        => format!("SetStatus({})", status),
            RefreshFilesLists        => "RefreshFilesLists".to_string()
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

    let main_state = Arc::new(Mutex::new(
        MainState {
            main_win: MainWindow::new(ui_thread_tx.clone()),
            settings_win: None,
            settings: None,
            backup_thread: None,
            backup_thread_tx: None,
            backup_thread_rx: None,
            ui_thread_tx: ui_thread_tx.clone(),
        }));

    let settings_file_path = match get_settings_file_path() {
        Ok(path) => path,
        Err(err) =>
            fatal_error(main_state.clone(), err.to_string())
    };
    let settings_folder_path = settings_file_path.parent().unwrap();

    init_logging(main_state.clone(), settings_folder_path);

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
            // Settings loaded with a user recoverable error
            state.settings = Some(settings.clone());
            let mut settings_win = SettingsWindow::new(state.ui_thread_tx.clone());
            settings_win.set_settings_to_win(settings);
            settings_win.wind.show();
            state.settings_win = Some(settings_win);
            if !warn_msg.is_empty() {
                message_default(&warn_msg);
            }
        }
        Err(SNotFound(Some(settings))) => {
            // A settings file was just created with defaults and needs to be validated and adjusted by the user
            state.settings = Some(settings.clone());
            let mut settings_win = SettingsWindow::new(state.ui_thread_tx.clone());
            settings_win.set_settings_to_win(settings);
            settings_win.wind.show();
            state.settings_win = Some(settings_win);
        }
        _ =>
            panic!("illegal state")
    }

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
                        warn!("Quitting - and ignoring message {}", ui_msg.to_string());
                        continue;
                    }
                }
                info!("Quitting - and allowing message {}", ui_msg.to_string());
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
                                    match write_settings(settings) {
                                        Err(err) => {
                                            drop(state);
                                            fatal_error(main_state, err.to_string());
                                        }
                                        Ok(settings) => {
                                            state.settings_win.as_mut().unwrap().wind.hide();
                                            state.settings_win = None;
                                            start_backup_thread(&mut state);
                                            drop(state);
                                            if let Err(err) = backup_all_changed_files(settings.clone()) {
                                                handle_file_error(main_state.clone(), &err);
                                            };
                                            if let Err(err) = delete_old_backups(settings) {
                                                handle_file_error(main_state.clone(), &err);
                                            }
                                            internal_message_queue.push(UiMessage::RefreshFilesLists);
                                        }
                                    }
                                }
                                Err(err) => {
                                    match err {
                                        SWarning(_settings, err_msg) => {
                                            if !err_msg.is_empty() {
                                                warn!("{}", err_msg);
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
                                    warn!("{}", err_msg);
                                    alert_default(&err_msg);
                                }
                                SettingsWinError::SwError(err_msg) => {
                                    drop(state);
                                    fatal_error(main_state.clone(), err_msg);
                                }
                            }
                        }
                    };
                }
                Alert(alert_msg) => {
                    alert_default(&alert_msg);
                }
                AlertQuit(alert_msg) => {
                    fatal_error(main_state.clone(), alert_msg);
                }
                AppQuit
                | MenuQuit
                | SettingsQuit => {
                    quitting = true;
                    start_graceful_quit(main_state.clone(), 0);
                }
                RestoreBackup => {
                    let state = main_state.lock().unwrap();
                    let selected_backup_paths = state.main_win.get_selected_backed_up_paths();
                    if !selected_backup_paths.is_empty() {
                        //TODO show confirmation dialog
                        assert!(state.settings.is_some(), "illegal state");
                        if let Err(err) = restore_backed_up_files(state.settings.as_ref().unwrap().clone(), selected_backup_paths) {
                            drop(state);
                            handle_file_error(main_state.clone(), &err);
                        }
                    }
                    internal_message_queue.push(UiMessage::RefreshFilesLists);
                }
                DeleteBackup => {
                    let state = main_state.lock().unwrap();
                    let selected_backup_paths = state.main_win.get_selected_backed_up_paths();
                    if !selected_backup_paths.is_empty() {
                        match choice_default(
                            format!("Delete {} backup files?", selected_backup_paths.len()).as_str(),
                            "Yes", "Cancel", ""
                        ) {
                            0 => {  // Yes
                                if let Err(err) = delete_backed_up_files(selected_backup_paths) {
                                    drop(state);
                                    handle_file_error(main_state.clone(), &err);
                                }
                            }
                            _ => ()
                        }
                    }
                    internal_message_queue.push(UiMessage::RefreshFilesLists);
                }
                PushStatus(status) => {
                    debug!("Pushing status message to: {}", &status);
                    let mut state = main_state.lock().unwrap();
                    state.main_win.push_status(status);
                }
                PopStatus => {
                    debug!("Popping status message");
                    let mut state = main_state.lock().unwrap();
                    state.main_win.pop_status();
                }
                SetStatus(status) => {
                    debug!("Setting status message to: {}", &status);
                    let mut state = main_state.lock().unwrap();
                    state.main_win.set_status(status);
                },
                RefreshFilesLists => {
                    let mut state = main_state.lock().unwrap();
                    match get_live_files(state.settings.as_ref().unwrap().clone()) {
                        Ok(live_files) => {
                            state.main_win.set_live_files_to_win(live_files);
                            match get_backed_up_files(state.settings.as_ref().unwrap().clone()) {
                                Ok(backed_up_files) => {
                                    state.main_win.set_backed_up_files_to_win(backed_up_files);
                                }
                                Err(err) => {
                                    drop(state);
                                    handle_file_error(main_state.clone(), &err);
                                }
                            }
                        },
                        Err(err) => {
                            drop(state);
                            handle_file_error(main_state.clone(), &err);
                        }
                    }
                }
            }
        }
    }
}

fn handle_file_error(main_state: Arc<Mutex<MainState>>, file_err: &Error) {
    let file_err = file_err.downcast_ref::<FileError>()
        .unwrap_or_else(|| fatal_error(main_state.clone(), file_err.to_string()));
    let summarize_errs = |errs: &Vec<String>| {
        let mut alert_err = errs.join("\n");
        if alert_err.len() > 100 {
            alert_err = alert_err[..100].to_string() + "...";
        }
        alert_err
    };
    match file_err {
        FWarning(errs) => {
            errs.iter().for_each(|err_msg| warn!("{}", err_msg));
            alert_default(&summarize_errs(errs));
        }
        FError(errs) => {
            errs.iter().for_each(|err_msg| error!("{}", err_msg));
            fatal_error(main_state.clone(), summarize_errs(errs));
        }
    }
}

fn init_logging(main_state: Arc<Mutex<MainState>>, settings_folder_path: &Path) {
    let log_file_path = settings_folder_path.join("valbak.log");
    let log_file_path = log_file_path.str();

    let rotating_log_writer =
        FileRotate::new(log_file_path, CountSuffix::new(2), ContentLimit::Lines(1000), Compression::None);

    let log_config = ConfigBuilder::default()
        .set_time_format("%Y-%m-%d %H:%M:%S%.3f".to_string())
        .set_time_to_local(true)
        .build();

    if let Err(err) = CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, log_config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Debug, log_config, rotating_log_writer)
        ],
    ) {
        fatal_error(main_state, format!("Error creating loggers: {}", err));
    }
}

fn fatal_error(main_state: Arc<Mutex<MainState>>, err_msg: String) -> ! {
    let err_msg = err_msg + "\nFatal error - Valbak must close";
    if log::logger().enabled(&Metadata::builder().level(Level::Error).build()) {
        error!("{}", err_msg);
    } else {
        println!("{}", err_msg);
    }
    // blocks until user dismisses the alert box
    alert_default(&err_msg);
    let exit_thread = start_graceful_quit(main_state, 1);
    if let Err(_) = exit_thread.join() {
        // ignore
    }
    // The exit thread should terminate the app before this occurs
    exit(1);
}

fn start_graceful_quit(main_state: Arc<Mutex<MainState>>, exit_code: i32) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut state = match main_state.try_lock() {
            Ok(lock) =>
                lock,
            Err(_) =>
                panic!("illegal state - main state lock not released")
        };
        if state.backup_thread.is_some() {
            let backup_thread = stop_backup_thread(&mut state);
            drop(state);
            if let Err(err) = backup_thread.join() {
                error!("Panic from backup thread: {:?}", err);
            }
        }
        exit(exit_code);
    })
}