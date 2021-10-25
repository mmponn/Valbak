
// use serde::{Deserialize, Serialize};
// use serde_json::Result;

// use crate::settings::{Backup, RedirectPath, Settings};

use std::process::exit;
use fltk::app;
use fltk::app::App;
use fltk::dialog::{alert_default, message_default};
use fltk::prelude::{WidgetExt, WindowExt};
use Message::{Bar, Foo};
use crate::main_win::connect_widgets;
use crate::settings::{default_settings, get_settings, SettingsError};

mod settings;
mod main_win;
mod settings_win;
mod win;

pub enum Message {
    Foo,
    Bar
}

fn main() {
    let app = App::default();
    let (sender, receiver) = app::channel::<Message>();

    let mut main_win = main_win::make_main_window();
    main_win.wind.show();

    main_win::connect_widgets(&main_win, &sender);

    let mut settings_win;

    match get_settings() {
        Ok(settings) => {
            settings_win = settings_win::make_settings_window(settings);
            settings_win.wind.make_modal(true);
        },
        Err(SettingsError::SError(msg)) => {
            let msg = msg + "\nFatal error - Valbak must close";
            alert_default(&msg);
            exit(1);
        },
        Err(SettingsError::SWarning(Some(settings), msg)) => {
            settings_win = settings_win::make_settings_window(settings);
            settings_win.wind.make_modal(true);
            settings_win.wind.show();
            message_default(&msg);
        },
        Err(SettingsError::SNotFound(Some(settings))) => {
            settings_win = settings_win::make_settings_window(settings);
            settings_win.wind.make_modal(true);
            settings_win.wind.show();
        }
        _ =>
            panic!("illegal state")
    }

    settings_win::connect_widgets(&settings_win, &sender);

    while app.wait() {
        if let Some(msg) = receiver.recv() {
            match msg {
                Foo => {}
                Bar => {}
            }
        }

    }
}