
// use serde::{Deserialize, Serialize};
// use serde_json::Result;

// use crate::settings::{Backup, RedirectPath, Settings};

use std::process::exit;
use fltk::app::App;
use fltk::dialog::{alert_default, message_default};
use fltk::prelude::{WidgetExt, WindowExt};
use crate::settings::{get_settings, SettingsError};

mod settings;
mod main_win;
mod settings_win;

fn main() {
    let mut main_win = main_win::make_main_window();
    main_win.show();

    match get_settings() {
        Ok(_) =>
            (),
        Err(SettingsError::SError(msg)) => {
            let msg = msg + "\nFatal error - Valbak must close";
            alert_default(&msg);
            exit(1);
        },
        Err(SettingsError::SWarning(Some(settings), msg)) => {
            let mut settings_win = settings_win::make_settings_window(settings);
            settings_win.make_modal(true);
            settings_win.show();
            message_default(&msg);
        },
        Err(SettingsError::SNotFound(Some(settings))) => {
            let mut settings_win = settings_win::make_settings_window(settings);
            settings_win.make_modal(true);
            settings_win.show();
        }
        _ =>
            panic!("illegal state")
    }

    let app = App::default();
    app.run().unwrap();
}