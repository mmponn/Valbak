use std::cmp::max;
use std::path::PathBuf;
use std::process::exit;

use chrono::{DateTime, Local};
use fltk::{app::*, app, browser::*, button::*, enums::*, group::*, input::*, prelude::*, window::*};
use fltk::frame::Frame;
use fltk::menu::{MenuBar, MenuFlag};
use fltk::misc::Tooltip;
use fltk::tree::TreeItemDrawMode::LabelAndWidget;

use crate::{UiMessage, win_common};
use crate::UiMessage::{AppQuit, MenuAbout, MenuDocumentation, MenuQuit, MenuSettings};
use crate::watcher::stop_backup_thread;

pub struct MainWindow {
    pub wind: DoubleWindow,
    status_frame: Frame,
    status_stack: Vec<String>,
    live_files: MultiBrowser,
    backed_up_files: MultiBrowser,
    redirect_list: MultiBrowser,
}

impl MainWindow {

    pub fn new(ui_thread_tx: Sender<UiMessage>) -> MainWindow {
        static WINDOW_SIZE: (i32, i32) = (1024, 715);
        static CONTENT_SIZE: (i32, i32) = (WINDOW_SIZE.0 - 20, WINDOW_SIZE.1 - 20);

        let mut wind = Window::default().with_label("Valbak");
        wind.set_size(WINDOW_SIZE.0, WINDOW_SIZE.1);

        let mut menu = MenuBar::default()
            .with_label("Menu");
        let text_size = menu.measure_label();
        menu.set_size(WINDOW_SIZE.0, text_size.1 + 10);
        let sender_copy = ui_thread_tx.clone();
        menu.add("File/Settings", Shortcut::None, MenuFlag::Normal,
            move |_menu_bar| sender_copy.send(MenuSettings));
        let sender_copy = ui_thread_tx.clone();
        menu.add("File/Quit", Shortcut::None, MenuFlag::Normal,
            move |_menu_bar| sender_copy.send(MenuQuit));
        let sender_copy = ui_thread_tx.clone();
        menu.add("Help/Documentation", Shortcut::None, MenuFlag::Normal,
            move |_menu_bar| sender_copy.send(MenuDocumentation));
        let sender_copy = ui_thread_tx.clone();
        menu.add("Help/About", Shortcut::None, MenuFlag::Normal,
            move |_menu_bar| sender_copy.send(MenuAbout));

        let mut live_files;
        let mut backed_up_files;
        let redirect_list;

        let mut content = Pack::default()
            .with_size(CONTENT_SIZE.0, CONTENT_SIZE.1)
            .with_pos(10, 10 + 20);
        content.set_spacing(5);

        win_common::make_section_header("Status", true);
        let mut status_frame = Frame::default();
        status_frame.set_align(Align::Inside | Align::Left);
        status_frame.set_label("Unknown");
        let text_size = status_frame.measure_label();
        status_frame.set_size(text_size.0, text_size.1);

        static FILE_LIST_COLUMN_WIDTHS: [i32; 3] = [CONTENT_SIZE.0 - 300, 200, 100];
        let file_header_texts: Vec<&str> = vec!["File", "File Date", "File Size"];

        // Live Files
        win_common::make_section_header("Live Files", true);
        win_common::column_headers(&file_header_texts, &FILE_LIST_COLUMN_WIDTHS);
        live_files = win_common::make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 100);

        live_files.set_selection_color(Color::White);

        // Backed-Up Files
        win_common::make_section_header("Backed-Up Files", true);
        win_common::column_headers(&file_header_texts, &FILE_LIST_COLUMN_WIDTHS);
        backed_up_files = win_common::make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 200);

        let mut backed_up_files_buttons = Pack::default()
            .with_type(PackType::Horizontal);
        backed_up_files_buttons.set_spacing(5);

        let mut restore_backups_button = Button::default()
            .with_label("Restore");
        let text_size = restore_backups_button.measure_label();
        restore_backups_button.set_size(text_size.0 + 15, text_size.1 + 10);
        let mut delete_backups_button = Button::default()
            .with_label("Delete");
        let text_size = delete_backups_button.measure_label();
        delete_backups_button.set_size(text_size.0 + 15, text_size.1 + 10);

        restore_backups_button
            .emit(ui_thread_tx.clone(), UiMessage::RestoreBackup);
        delete_backups_button
            .emit(ui_thread_tx.clone(), UiMessage::DeleteBackup);

        backed_up_files_buttons.set_size(0, text_size.1 + 10);

        backed_up_files_buttons.end();

        // Redirects
        static REDIRECT_COLUMN_WIDTHS: [i32; 3] = [(CONTENT_SIZE.0 - 50) / 2, (CONTENT_SIZE.0 - 50) / 2, 50];
        let redirect_header_texts = vec!["Source Directory", "Destination Directory", "Active"];

        win_common::make_section_header("Redirects", true);
        win_common::column_headers(&redirect_header_texts, &REDIRECT_COLUMN_WIDTHS);
        redirect_list = win_common::make_list_browser(&REDIRECT_COLUMN_WIDTHS, 100);

        let mut redirect_list_buttons = Pack::default()
            .with_type(PackType::Horizontal);
        redirect_list_buttons.set_spacing(5);

        let mut activate_redirect_button = Button::default()
            .with_label("Activate");
        let text_size = activate_redirect_button.measure_label();
        activate_redirect_button.set_size(text_size.0 + 15, text_size.1 + 10);
        let mut deactivate_redirect_button = Button::default()
            .with_label("Deactivate");
        let text_size = deactivate_redirect_button.measure_label();
        deactivate_redirect_button.set_size(text_size.0 + 15, text_size.1 + 10);

        activate_redirect_button.emit(ui_thread_tx.clone(), UiMessage::ActivateRedirect);
        deactivate_redirect_button.emit(ui_thread_tx.clone(), UiMessage::DeactivateRedirect);

        redirect_list_buttons.set_size(0, text_size.1 + 10);

        redirect_list_buttons.end();

        content.end();

        wind.end();

        wind.set_callback(move |_| {
            if app::event() == Event::Close {
                ui_thread_tx.send(AppQuit);
            }
        });

        MainWindow {
            wind,
            status_frame,
            status_stack: Vec::new(),
            live_files,
            backed_up_files,
            redirect_list,
        }
    }

    pub fn push_status(&mut self, status: String) {
        self.status_frame.set_label(&status);
        self.status_stack.push(status);
    }

    pub fn pop_status(&mut self) {
        self.status_stack.pop();
        let status = match self.status_stack.last() {
            None => "",
            Some(status) => &status
        };
        self.status_frame.set_label(status);
    }

    pub fn set_status(&mut self, status: String) {
        self.status_frame.set_label(&status);
        self.status_stack.clear();
        self.status_stack.push(status);
    }

    pub fn set_live_files_to_win(&mut self, mut live_files: Vec<PathBuf>) {
        live_files.sort();
        self.live_files.clear();
        for live_file in live_files {
            let live_file_metadata = match live_file.metadata() {
                Err(err) => {
                    println!("Error reading file metadata for {}: {}", live_file.to_str().unwrap(), err);
                    continue;
                }
                Ok(metadata) =>
                    metadata
            };
            let live_file_modified = match live_file_metadata.modified() {
                Err(err) => {
                    println!("Error reading file modified time for {}: {}", live_file.to_str().unwrap(), err);
                    continue;
                }
                Ok(modified) =>
                    modified
            };
            let live_file_modified: DateTime<Local> = live_file_modified.into();
            let mut live_file_size_mb = live_file_metadata.len() / (1000 * 1000);
            let live_file_size;
            if live_file_size_mb > 0 {
                live_file_size = live_file_size_mb.to_string() + "mb";
            } else {
                live_file_size = (live_file_metadata.len() / 1000).to_string() + "kb";
            }
            let live_file_line = format!("{}|{}|{}",
                live_file.to_str().unwrap(),
                live_file_modified.format("%m/%d/%Y %T"),
                live_file_size
            );
            self.live_files.add(&live_file_line);
        }
    }

    pub fn set_backed_up_files_to_win(&mut self, mut backed_up_files: Vec<PathBuf>) {
        backed_up_files.sort();
        backed_up_files.reverse();
        self.backed_up_files.clear();
        for backed_up_file in backed_up_files {
            let backed_up_file_metadata = match backed_up_file.metadata() {
                Err(err) => {
                    println!("Error reading file metadata for {}: {}", backed_up_file.to_str().unwrap(), err);
                    continue;
                }
                Ok(metadata) =>
                    metadata
            };
            let backed_up_file_modified = match backed_up_file_metadata.modified() {
                Err(err) => {
                    println!("Error reading file modified time for {}: {}", backed_up_file.to_str().unwrap(), err);
                    continue;
                }
                Ok(modified) =>
                    modified
            };
            let backed_up_file_modified: DateTime<Local> = backed_up_file_modified.into();
            let mut backed_up_file_size_mb = backed_up_file_metadata.len() / (1000 * 1000);
            let backed_up_file_size;
            if backed_up_file_size_mb > 0 {
                backed_up_file_size = backed_up_file_size_mb.to_string() + "mb";
            } else {
                backed_up_file_size = (backed_up_file_metadata.len() / 1000).to_string() + "kb";
            }
            let backed_up_file_line = format!("{}|{}|{}",
                backed_up_file.to_str().unwrap(),
                backed_up_file_modified.format("%m/%d/%Y %T"),
                backed_up_file_size
            );
            self.backed_up_files.add(&backed_up_file_line);
        }
    }

    pub fn get_selected_backed_up_paths(&self) -> Vec<PathBuf> {
        let mut selected_backed_up_paths = Vec::new();
        for i in 1..=self.backed_up_files.size() {
            if self.backed_up_files.selected(i) {
                let selected_line = match self.backed_up_files.text(i) {
                    None =>
                        panic!("illegal state"),
                    Some(text) =>
                        text
                };
                let backed_up_path = match selected_line.split("|").next() {
                    None =>
                        panic!("illegal state"),
                    Some(backed_up_path) =>
                        backed_up_path
                };
                selected_backed_up_paths.push(PathBuf::from(backed_up_path));
            }
        }
        selected_backed_up_paths
    }
}