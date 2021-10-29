use fltk::app;
use fltk::app::Sender;
use fltk::browser::MultiBrowser;
use fltk::button::Button;
use fltk::enums::{Align, Event, FrameType};
use fltk::frame::Frame;
use fltk::group::{Group, Pack, PackType};
use fltk::prelude::{BrowserExt, GroupExt, WidgetBase, WidgetExt};
use fltk::widget::Widget;
use fltk::window::Window;

use crate::Message;
use crate::Message::SettingsOk;
use crate::settings::Settings;
use crate::win_common::{column_headers, make_list_browser, make_section_header};

pub struct SettingsWindow {
    pub wind: Window,
    pub backup_files: MultiBrowser,
    pub new_backup_button: Button,
    pub edit_backup_button: Button,
    pub delete_backup_button: Button,
    pub redirect_folders: MultiBrowser,
    pub new_redirect_button: Button,
    pub edit_redirect_button: Button,
    pub delete_redirect_button: Button,
    pub ok_button: Button,
}

pub fn make_settings_window(sender: Sender<Message>) -> SettingsWindow {
    static WINDOW_SIZE: (i32, i32) = (800, 440);
    static CONTENT_SIZE: (i32, i32) = (WINDOW_SIZE.0 - 20, WINDOW_SIZE.1 - 20);

    let mut wind = Window::default().with_label("Settings");
    wind.set_size(WINDOW_SIZE.0, WINDOW_SIZE.1);

    let mut content = Pack::default()
        .with_pos(10, 10);
    content.set_spacing(5);
    static BACKUP_LIST_COLUMN_WIDTHS: [i32; 2] = [CONTENT_SIZE.0 - 100, 100];

    // Live Files
    make_section_header("Files to Back up", true);
    column_headers(
        &vec!("Folder", "File Pattern"),
        &BACKUP_LIST_COLUMN_WIDTHS);
    let mut backup_files = make_list_browser(&BACKUP_LIST_COLUMN_WIDTHS, 100);

    backup_files.add(r"C:\Foo\Bar|*.db");

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

    static REDIRECT_LIST_COLUMN_WIDTHS: [i32; 2] = [CONTENT_SIZE.0 / 2, CONTENT_SIZE.0 / 2];

    // Live Files
    make_section_header("Folders to redirect", true);
    column_headers(
        &vec!("Folder", "Redirect To"),
        &REDIRECT_LIST_COLUMN_WIDTHS);
    let mut redirect_folders = make_list_browser(&REDIRECT_LIST_COLUMN_WIDTHS, 100);

    redirect_folders.add(r"C:\Foo\Bar|R:\");

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

    let mut ok_button_group = Group::default();

    content.end();

    let mut ok_button = Button::default()
        .with_label("Ok");
    let text_size = ok_button.measure_label();
    ok_button.set_size(text_size.0 + 50, text_size.1 + 14);
    ok_button.set_pos(CONTENT_SIZE.0 - ok_button.width() - 5, 0);

    ok_button.emit(sender, SettingsOk);

    ok_button_group.set_size(CONTENT_SIZE.0, ok_button.height());
    ok_button_group.add(&ok_button);

    wind.end();

    wind.set_callback(|_wind| {
        if app::event() == Event::Close {
            // Disables Escape key closes window behavior
        }
    });

    SettingsWindow {
        wind,
        backup_files,
        new_backup_button,
        edit_backup_button,
        delete_backup_button,
        redirect_folders,
        new_redirect_button,
        edit_redirect_button,
        delete_redirect_button,
        ok_button
    }
}

impl SettingsWindow {
    pub fn connect_widgets(&mut self, sender: &Sender<Message>) {
        self.new_backup_button
            .set_callback(|button| println!("new"));
    }
}