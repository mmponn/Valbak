use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use fltk::app;
use fltk::browser::MultiBrowser;
use fltk::button::Button;
use fltk::dialog::{FileChooser, FileChooserType};
use fltk::enums::{Align, Event, FrameType};
use fltk::frame::Frame;
use fltk::group::{Group, Pack, PackType};
use fltk::input::{FileInput, Input};
use fltk::prelude::{BrowserExt, GroupExt, InputExt, WidgetBase, WidgetExt, WindowExt};
use fltk::widget::Widget;
use fltk::window::Window;
use thiserror::Error;

use UiMessage::SettingsBackupDestChoose;

use crate::settings::{BackupFilePattern, Settings, SETTINGS_VERSION, SettingsError};
use crate::UiMessage;
use crate::UiMessage::{SettingsOk, SettingsQuit};
use crate::win_common::{column_headers, make_list_browser, make_section_header};

#[derive(Error, Debug)]
pub enum SettingsWinError {
    SwWarning(String),
    SwError(String)
}

impl Display for SettingsWinError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct SettingsWindow {
    pub wind: Window,
    backup_files_browser: MultiBrowser,
    backup_dest_input: Input,
    backup_count_input: Input,
    backup_delay_input: Input
}

impl SettingsWindow {

    pub fn new(sender: app::Sender<UiMessage>) -> SettingsWindow {
        static WINDOW_SIZE: (i32, i32) = (800, 420);
        static CONTENT_SIZE: (i32, i32) = (WINDOW_SIZE.0 - 20, WINDOW_SIZE.1 - 20);

        let mut wind = Window::default().with_label("Settings");
        wind.make_modal(true);
        wind.set_size(WINDOW_SIZE.0, WINDOW_SIZE.1);

        let mut content = Pack::default()
            .with_pos(10, 10);
        content.set_spacing(5);
        static BACKUP_LIST_COLUMN_WIDTHS: [i32; 2] = [CONTENT_SIZE.0 - 100, 100];

        // Live Files
        make_section_header("Files to Backup", true);
        column_headers(
            &vec!["Folder", "File Pattern"],
            &BACKUP_LIST_COLUMN_WIDTHS);
        let mut backup_files_browser = make_list_browser(&BACKUP_LIST_COLUMN_WIDTHS, 100);

        let mut backup_files_buttons = Pack::default()
            .with_type(PackType::Horizontal);
        backup_files_buttons.set_spacing(5);

        let mut new_backup_button = Button::default()
            .with_label("New");
        let text_size = new_backup_button.measure_label();
        new_backup_button.set_size(text_size.0 + 15, text_size.1 + 10);
        let mut edit_backup_button = Button::default()
            .with_label("Edit");
        let text_size = edit_backup_button.measure_label();
        edit_backup_button.set_size(text_size.0 + 15, text_size.1 + 10);
        let mut delete_backup_button = Button::default()
            .with_label("Delete");
        let text_size = delete_backup_button.measure_label();
        delete_backup_button.set_size(text_size.0 + 15, text_size.1 + 10);

        backup_files_buttons.set_size(0, text_size.1 + 10);

        backup_files_buttons.end();

        make_section_header("Backup to folder", true);

        let mut backup_dest_fields = Pack::default()
            .with_type(PackType::Horizontal);
        backup_dest_fields.set_spacing(5);

        let mut backup_dest_input = Input::default();
        backup_dest_input.set_size(0, backup_dest_input.text_size() + 12);

        let mut backup_dest_select_button = Button::default()
            .with_label("...");
        let text_size = backup_dest_select_button.measure_label();
        backup_dest_select_button.set_size(text_size.0 + 15, text_size.1 + 10);
        backup_dest_select_button.set_pos( CONTENT_SIZE.0 - backup_dest_select_button.width(), 0);
        backup_dest_select_button.emit(sender.clone(), SettingsBackupDestChoose);

        backup_dest_input.set_size(
            CONTENT_SIZE.0 - backup_dest_select_button.width() - 5, backup_dest_input.height());

        backup_dest_fields.set_size(0, backup_dest_select_button.height());
        backup_dest_fields.end();

        make_section_header("Maximum number of backups per file", true);

        let mut backup_count_input = Input::default();
        backup_count_input.set_size(0, backup_count_input.text_size() + 12);

        make_section_header("File backup delay in seconds", true);

        let mut backup_delay_input = Input::default();
        backup_delay_input.set_size(0, backup_delay_input.text_size() + 12);

        content.set_size(CONTENT_SIZE.0, backup_delay_input.y() + backup_delay_input.height());

        let mut bottom_button_group = Group::default();

        content.end();

        let mut quit_button = Button::default()
            .with_label("Quit");
        let text_size = quit_button.measure_label();
        quit_button.set_size(text_size.0 + 50, text_size.1 + 14);
        quit_button.set_pos(CONTENT_SIZE.0 - quit_button.width() - 5, 0);
        quit_button.emit(sender.clone(), SettingsQuit);

        let mut ok_button = Button::default()
            .with_label("Ok");
        let text_size = ok_button.measure_label();
        ok_button.set_size(text_size.0 + 50, text_size.1 + 14);
        ok_button.set_pos(quit_button.x() - ok_button.width() - 5, 0);
        ok_button.emit(sender.clone(), SettingsOk);

        bottom_button_group.set_size(CONTENT_SIZE.0, ok_button.height());
        bottom_button_group.add(&ok_button);
        bottom_button_group.add(&quit_button);

        wind.end();

        wind.set_callback(|_wind| {
            if app::event() == Event::Close {
                // Disables Escape key closes window behavior
            }
        });

        SettingsWindow {
            wind,
            backup_files_browser,
            backup_dest_input,
            backup_count_input,
            backup_delay_input
        }
    }

    pub fn get_settings_from_win(&self) -> Result<Settings, SettingsWinError> {
        let mut backup_settings = vec![];
        for i in 1..=self.backup_files_browser.size() {
            let text = self.backup_files_browser.text(i);
            let backup_files_line = text.unwrap();
            let backup_files_parts: Vec<&str> = backup_files_line.split("|").collect();
            let backup_source_path = backup_files_parts[0];
            let backup_files_glob = backup_files_parts[1];
            backup_settings.push(BackupFilePattern {
                source_dir: PathBuf::from(backup_source_path),
                file_pattern: backup_files_glob.to_string()
            });
        }

        let backup_dest_path = self.backup_dest_input.value();

        let backup_count = self.backup_count_input.value();
        let backup_count = match backup_count.parse::<u8>() {
            Ok(count) =>
                count,
            Err(err) =>
                return Err(
                    SettingsWinError::SwWarning(format!("Invalid backup count: {}", backup_count))
                )
        };

        let backup_delay_sec = self.backup_delay_input.value();
        let backup_delay_sec = match backup_delay_sec.parse::<u8>() {
            Ok(delay_sec) =>
                delay_sec,
            Err(err) =>
                return Err(
                    SettingsWinError::SwWarning(format!("Invalid delay seconds: {}", backup_delay_sec))
                )
        };

        Ok(Settings {
                settings_version: SETTINGS_VERSION.to_string(),
                backup_paths: backup_settings,
                backup_dest_path: PathBuf::from(backup_dest_path),
                backup_count,
                backup_delay_sec
        })
    }

    pub fn set_settings_to_win(&mut self, settings: Settings) {
        self.clear_win();
        for backup_file_pattern in settings.backup_paths {
            let backup_file_line = format!("{}|{}",
                backup_file_pattern.source_dir.to_str().unwrap(),
                backup_file_pattern.file_pattern
            );
            self.backup_files_browser.add(&backup_file_line);
        }

        self.backup_dest_input.set_value(settings.backup_dest_path.to_str().unwrap());

        self.backup_count_input.set_value(&settings.backup_count.to_string());

        self.backup_delay_input.set_value(&settings.backup_delay_sec.to_string());
    }

    fn clear_win(&mut self) {
        for i in (1..=self.backup_files_browser.size()).rev() {
            self.backup_files_browser.remove(i);
        }
        self.backup_dest_input.set_value("");
    }

    pub fn choose_backup_dest_dir(&mut self, mut settings: Settings) {
        let mut file_chooser =
            FileChooser::new(settings.backup_dest_path.to_str().unwrap(),
                             "",
                             FileChooserType::Single | FileChooserType::Directory,
                             "Choose backup destination folder");
        file_chooser.set_preview(false);
        file_chooser.preview_button().unwrap().hide();
        file_chooser.new_button().unwrap().activate();
        file_chooser.show();
        while file_chooser.shown() {
            app::wait();
        }
        if let Some(mut dir) = file_chooser.directory() {
            // FLTK File Chooser apparently always uses forward slashes
            if std::path::MAIN_SEPARATOR != '/' {
                dir = dir.replace("/", &std::path::MAIN_SEPARATOR.to_string());
            }
            settings.backup_dest_path = PathBuf::from(dir);
            self.set_settings_to_win(settings);
        }
    }
}