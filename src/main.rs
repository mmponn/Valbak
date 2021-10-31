
// use serde::{Deserialize, Serialize};
// use serde_json::Result;

// use crate::settings::{Backup, RedirectPath, Settings};

use std::path::PathBuf;
use std::process::exit;

use fltk::app;
use fltk::app::{App, Receiver, Sender};
use fltk::dialog::{alert_default, FileChooser, FileChooserType, message_default};
use fltk::enums::Event;
use fltk::prelude::{WidgetExt, WindowExt};
use main_win::MainWindow;

use Message::*;
use settings_win::SettingsWindow;

use crate::settings::{get_default_settings, get_settings, Settings, SettingsError, write_settings};

mod settings;
mod main_win;
mod settings_win;
mod win_common;

#[derive(Copy, Clone)]
pub enum Message {
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
}

fn main() {
    let app = App::default();

    let (sender, receiver) = app::channel::<Message>();

    let mut main_win = MainWindow::new(sender);
    main_win.wind.show();

    let mut settings_win = SettingsWindow::new(sender);

    match get_settings() {
        Ok(settings) => {
            settings_win.set_settings_to_win(settings);
        }
        Err(SettingsError::SError(err_msg)) => {
            fatal_error(err_msg);
        }
        Err(SettingsError::SWarning(settings, msg)) => {
            settings_win.set_settings_to_win(settings);
            settings_win.wind.show();
            message_default(&msg);
        }
        Err(SettingsError::SNotFound(Some(settings))) => {
            // A settings file was just created with defaults and needs to be adjusted by the user
            settings_win.set_settings_to_win(settings);
            settings_win.wind.show();
        }
        _ =>
            panic!("illegal state")
    }

    while app.wait() {
        if let Some(msg) = receiver.recv() {
            match msg {
                MenuSettings => {
                    println!("Menu Settings");
                    settings_win.wind.make_modal(true);
                    settings_win.wind.show()
                }
                MenuQuit => {
                    app::quit();
                    exit(0);
                }
                MenuDocumentation => {}
                MenuAbout => {}
                SettingsBackupDestChoose => {
                    settings_win.choose_backup_dest_dir();
                }
                SettingsOk => {
                    let settings = settings_win.get_settings_from_win();
                    match settings::validate_settings(settings) {
                        Ok(settings) => {
                            write_settings(settings);
                            settings_win.wind.hide();
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