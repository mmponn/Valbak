use std::path::PathBuf;
use fltk::app;
use fltk::app::Sender;
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
use Message::SettingsBackupDestChoose;

use crate::Message;
use crate::Message::{SettingsOk, SettingsQuit};
use crate::settings::{BackupFilePattern, RedirectFolder, Settings, SETTINGS_VERSION};
use crate::win_common::{column_headers, make_list_browser, make_section_header};

pub struct SettingsWindow {
    pub wind: Window,
    backup_files_browser: MultiBrowser,
    backup_dest_input: Input,
    redirect_folders_browser: MultiBrowser,
}

impl SettingsWindow {
}

impl SettingsWindow {

    pub fn new(sender: Sender<Message>) -> SettingsWindow {
        static WINDOW_SIZE: (i32, i32) = (800, 500);
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
        backup_dest_select_button.emit(sender, SettingsBackupDestChoose);

        backup_dest_input.set_size(
            CONTENT_SIZE.0 - backup_dest_select_button.width() - 5, backup_dest_input.height());

        backup_dest_fields.set_size(0, backup_dest_select_button.height());
        backup_dest_fields.end();

        static REDIRECT_LIST_COLUMN_WIDTHS: [i32; 2] = [CONTENT_SIZE.0 / 2, CONTENT_SIZE.0 / 2];

        // Live Files
        make_section_header("Folders to redirect", true);
        column_headers(
            &vec!["Folder", "Redirect To"],
            &REDIRECT_LIST_COLUMN_WIDTHS);
        let mut redirect_folders_browser = make_list_browser(&REDIRECT_LIST_COLUMN_WIDTHS, 100);

        let mut redirect_buttons = Pack::default()
            .with_type(PackType::Horizontal);
        redirect_buttons.set_spacing(5);

        let mut new_redirect_button = Button::default()
            .with_label("New");
        let text_size = new_redirect_button.measure_label();
        new_redirect_button.set_size(text_size.0 + 15, text_size.1 + 10);
        let mut edit_redirect_button = Button::default()
            .with_label("Edit");
        let text_size = edit_redirect_button.measure_label();
        edit_redirect_button.set_size(text_size.0 + 15, text_size.1 + 10);
        let mut delete_redirect_button = Button::default()
            .with_label("Delete");
        let text_size = delete_redirect_button.measure_label();
        delete_redirect_button.set_size(text_size.0 + 15, text_size.1 + 10);

        redirect_buttons.set_size(0, text_size.1 + 10);

        redirect_buttons.end();
        content.set_size(CONTENT_SIZE.0, redirect_buttons.y() + redirect_buttons.height());

        let frame = Frame::default()
            .with_size(0, 10);

        let mut bottom_button_group = Group::default();

        content.end();

        let mut quit_button = Button::default()
            .with_label("Quit");
        let text_size = quit_button.measure_label();
        quit_button.set_size(text_size.0 + 50, text_size.1 + 14);
        quit_button.set_pos(CONTENT_SIZE.0 - quit_button.width() - 5, 0);
        quit_button.emit(sender, SettingsQuit);

        let mut ok_button = Button::default()
            .with_label("Ok");
        let text_size = ok_button.measure_label();
        ok_button.set_size(text_size.0 + 50, text_size.1 + 14);
        ok_button.set_pos(quit_button.x() - ok_button.width() - 5, 0);
        ok_button.emit(sender, SettingsOk);

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
            redirect_folders_browser,
        }
    }

    pub fn get_settings_from_win(&self) -> Settings {
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

        let mut redirect_settings = vec![];
        for i in 1..=self.redirect_folders_browser.size() {
            let text = self.redirect_folders_browser.text(i);
            let redirect_folders_line = text.unwrap();
            let redirect_folders_parts: Vec<&str> = redirect_folders_line.split("|").collect();
            let redirect_source_path = redirect_folders_parts[0];
            let redirect_target_path = redirect_folders_parts[1];
            redirect_settings.push(RedirectFolder {
                from_dir: PathBuf::from(redirect_source_path),
                to_dir: PathBuf::from(redirect_target_path)
            });
        }

        Settings {
            settings_version: SETTINGS_VERSION.to_string(),
            backup_paths: backup_settings,
            backup_dest_path: PathBuf::from(backup_dest_path),
            redirect_folders: redirect_settings
        }
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
        for redirect_folder in settings.redirect_folders {
            let redirect_path_line = format!("{}|{}",
                redirect_folder.from_dir.to_str().unwrap(),
                redirect_folder.to_dir.to_str().unwrap()
            );
            self.redirect_folders_browser.add(&redirect_path_line);
        }
    }

    fn clear_win(&mut self) {
        for i in (1..=self.backup_files_browser.size()).rev() {
            self.backup_files_browser.remove(i);
        }
        self.backup_dest_input.set_value("");
        for i in (1..=self.redirect_folders_browser.size()).rev() {
            self.redirect_folders_browser.remove(i);
        }
    }

    pub fn choose_backup_dest_dir(&mut self) {
        let mut settings = self.get_settings_from_win();
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