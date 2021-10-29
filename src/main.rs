
// use serde::{Deserialize, Serialize};
// use serde_json::Result;

// use crate::settings::{Backup, RedirectPath, Settings};

use std::process::exit;

use fltk::app;
use fltk::app::{App, Receiver, Sender};
use fltk::dialog::{alert_default, message_default};
use fltk::enums::Event;
use fltk::prelude::{WidgetExt, WindowExt};

use Message::*;

use crate::settings::{default_settings, get_settings, SettingsError};

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
    SettingsOk,
    RestoreBackup,
    DeleteBackup,
    ActivateRedirect,
    DeactivateRedirect,
}

fn main() {
    let app = App::default();

    let (sender, receiver) = app::channel::<Message>();

    let mut main_win = main_win::make_main_window(sender);
    main_win.wind.show();

    let mut settings_win = settings_win::make_settings_window(sender);
    settings_win.wind.make_modal(true);

    match get_settings() {
        Ok(settings) => {},
        Err(SettingsError::SError(msg)) => {
            let msg = msg + "\nFatal error - Valbak must close";
            alert_default(&msg);
            app::quit();
            exit(1);
        },
        Err(SettingsError::SWarning(Some(settings), msg)) => {
            settings_win.wind.show();
            message_default(&msg);
        },
        Err(SettingsError::SNotFound(Some(settings))) => {
            settings_win.wind.show();
        }
        _ =>
            panic!("illegal state")
    }

    settings_win.connect_widgets(&sender);

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
                SettingsOk => {
                    settings_win.wind.hide();
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