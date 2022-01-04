/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Local};
use fltk::{app::*, app, browser::*, button::*, enums::*, group::*, prelude::*, window::*};
use fltk::frame::Frame;
use fltk::menu::{MenuBar, MenuFlag};
use log::error;

use FileError::{FError, FFatal};

use crate::{FileError, FWarning, UiMessage, win_common};
use crate::file::{get_backed_up_path, get_backed_up_version_number, get_file_metadata, PathExt};
use crate::UiMessage::{AppQuit, MenuAbout, MenuDocumentation, MenuQuit, MenuSettings};

pub struct MainWindow {
    pub wind: DoubleWindow,
    status_frame: Frame,
    status_stack: Vec<String>,
    live_files: MultiBrowser,
    backed_up_files: MultiBrowser,
}

impl MainWindow {

    pub fn new(ui_thread_tx: Sender<UiMessage>) -> MainWindow {
        static WINDOW_SIZE: (i32, i32) = (1024, 800);
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
        let backed_up_files;

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
        live_files = win_common::make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 242);

        live_files.set_selection_color(Color::White);

        // Backed-Up Files
        win_common::make_section_header("Backed-Up Files", true);
        win_common::column_headers(&file_header_texts, &FILE_LIST_COLUMN_WIDTHS);
        backed_up_files = win_common::make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 322);

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
                    error!("Error reading file metadata for {}: {}", live_file.str(), err);
                    continue;
                }
                Ok(metadata) =>
                    metadata
            };
            let live_file_modified = match live_file_metadata.modified() {
                Err(err) => {
                    error!("Error reading file modified time for {}: {}", live_file.str(), err);
                    continue;
                }
                Ok(modified) =>
                    modified
            };
            let live_file_modified: DateTime<Local> = live_file_modified.into();
            let live_file_size_mb = live_file_metadata.len() / (1000 * 1000);
            let live_file_size;
            if live_file_size_mb > 0 {
                live_file_size = live_file_size_mb.to_string() + "mb";
            } else {
                live_file_size = (live_file_metadata.len() / 1000).to_string() + "kb";
            }
            let live_file_line = format!("{}|{}|{}",
                live_file.str(),
                live_file_modified.format("%m/%d/%Y %T"),
                live_file_size
            );
            self.live_files.add(&live_file_line);
        }
    }

    pub fn set_backed_up_files_to_win(&mut self, mut backed_up_files: Vec<PathBuf>) -> Result<(), FileError> {
        let mut errors = vec![];

        // Sort the backed up files so they are ready to be displayed to the user
        backed_up_files.sort_by(|a, b| {
            let (_a_metadata, a_modified) = match get_file_metadata(a) {
                Ok(metadata) => metadata,
                Err(err) => {
                    match err {
                        FWarning(mut errs)
                        | FError(mut errs)
                        | FFatal(mut errs) =>
                            errs.drain(..).for_each(|err_msg| errors.push(err_msg))
                    }
                    return Ordering::Equal;
                }
            };
            let (_b_metadata, b_modified) = match get_file_metadata(b) {
                Ok(metadata) => metadata,
                Err(err) => {
                    match err {
                        FWarning(mut errs)
                        | FError(mut errs)
                        | FFatal(mut errs) =>
                            errs.drain(..).for_each(|err_msg| errors.push(err_msg))
                    }
                    return Ordering::Equal;
                }
            };

            // Reverse datetime sort
            match b_modified.cmp(&a_modified) {
                Ordering::Less =>
                    return Ordering::Less,
                Ordering::Greater =>
                    return Ordering::Greater,
                _ => {}
            }

            let filename_a = match get_backed_up_path(a) {
                Some(name) => name,
                None => {
                    errors.push(format!("Invalid backup file name {}", b.str()));
                    return Ordering::Equal;
                }
            };
            let filename_b = match get_backed_up_path(b) {
                Some(name) => name,
                None => {
                    errors.push(format!("Invalid backup file name {}", b.str()));
                    return Ordering::Equal;
                }
            };

            // Forward filename ordering
            match filename_a.cmp(filename_b) {
                Ordering::Less =>
                    return Ordering::Less,
                Ordering::Greater =>
                    return Ordering::Greater,
                _ => {}
            }

            let backup_number_a = match get_backed_up_version_number(a) {
                Some(n) => n,
                None => {
                    errors.push(format!("Invalid backup file name {}", a.str()));
                    return Ordering::Equal;
                }
            };
            let backup_number_b = match get_backed_up_version_number(b) {
                Some(n) => n,
                None => {
                    errors.push(format!("Invalid backup file name {}", b.str()));
                    return Ordering::Equal;
                }
            };

            // Reverse number ordering
            backup_number_b.cmp(&backup_number_a)
        });

        if !errors.is_empty() {
            return Err(FWarning(errors));
        }

        self.backed_up_files.clear();
        for backed_up_file in backed_up_files {
            let backed_up_file_metadata = match backed_up_file.metadata() {
                Err(err) => {
                    errors.push(format!("Error reading file metadata for {}: {}", backed_up_file.str(), err));
                    continue;
                }
                Ok(metadata) =>
                    metadata
            };
            let backed_up_file_modified = match backed_up_file_metadata.modified() {
                Err(err) => {
                    errors.push(format!("Error reading file modified time for {}: {}", backed_up_file.str(), err));
                    continue;
                }
                Ok(modified) =>
                    modified
            };
            let backed_up_file_modified: DateTime<Local> = backed_up_file_modified.into();
            let backed_up_file_size_mb = backed_up_file_metadata.len() / (1000 * 1000);
            let backed_up_file_size;
            if backed_up_file_size_mb > 0 {
                backed_up_file_size = backed_up_file_size_mb.to_string() + "mb";
            } else {
                backed_up_file_size = (backed_up_file_metadata.len() / 1000).to_string() + "kb";
            }
            let backed_up_file_line = format!("{}|{}|{}",
                backed_up_file.str(),
                backed_up_file_modified.format("%m/%d/%Y %T"),
                backed_up_file_size
            );
            self.backed_up_files.add(&backed_up_file_line);
        }

        if !errors.is_empty() {
            return Err(FWarning(errors));
        }
        Ok(())
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