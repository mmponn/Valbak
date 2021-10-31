use std::cmp::max;
use std::process::exit;

use fltk::{app::*, app, browser::*, button::*, enums::*, group::*, input::*, prelude::*, window::*};
use fltk::frame::Frame;
use fltk::menu::{MenuBar, MenuFlag};
use fltk::misc::Tooltip;

use crate::{Message, win_common};
use crate::Message::{MenuAbout, MenuDocumentation, MenuQuit, MenuSettings};

pub struct MainWindow {
    pub wind: DoubleWindow,
    pub live_files: MultiBrowser,
    pub backed_up_files: MultiBrowser,
    pub restore_backups_button: Button,
    pub delete_backups_button: Button,
    pub redirect_list: MultiBrowser,
    pub activate_redirect_button: Button,
    pub deactivate_redirect_button: Button,
}

impl MainWindow {

    pub fn new(sender: Sender<Message>) -> MainWindow {
        static WINDOW_SIZE: (i32, i32) = (800, 665);
        static CONTENT_SIZE: (i32, i32) = (WINDOW_SIZE.0 - 20, WINDOW_SIZE.1 - 20);

        let mut wind = Window::default().with_label("Valbak");
        wind.set_size(WINDOW_SIZE.0, WINDOW_SIZE.1);

        let mut menu = MenuBar::default()
            .with_label("Menu");
        let text_size = menu.measure_label();
        menu.set_size(WINDOW_SIZE.0, text_size.1 + 10);
        menu.add("File/Settings", Shortcut::None, MenuFlag::Normal, move |_menu_bar| sender.send(MenuSettings));
        menu.add("File/Quit", Shortcut::None, MenuFlag::Normal, move |_menu_bar| sender.send(MenuQuit));
        menu.add("Help/Documentation", Shortcut::None, MenuFlag::Normal, move |_menu_bar| sender.send(MenuDocumentation));
        menu.add("Help/About", Shortcut::None, MenuFlag::Normal, move |_menu_bar| sender.send(MenuAbout));

        let mut live_files;
        let mut backed_up_files;
        let redirect_list;

        let mut content = Pack::default()
            .with_size(CONTENT_SIZE.0, CONTENT_SIZE.1)
            .with_pos(10, 10 + 20);
        content.set_spacing(5);

        static FILE_LIST_COLUMN_WIDTHS: [i32; 3] = [CONTENT_SIZE.0 - 300, 200, 100];
        let file_header_texts: Vec<&str> = vec!["File", "File Date", "File Size"];

        // Live Files
        win_common::make_section_header("Live Files", true);
        win_common::column_headers(&file_header_texts, &FILE_LIST_COLUMN_WIDTHS);
        live_files = win_common::make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 100);

        live_files.add("File #1|2021-10-17 12:22pm|102kb");
        live_files.add("File #2|2021-10-15 2:22pm|233kb");
        live_files.add("File #3|2021-10-14 8:22pm|12kb");

        live_files.set_selection_color(Color::White);

        // Backed-Up Files
        win_common::make_section_header("Backed-Up Files", true);
        win_common::column_headers(&file_header_texts, &FILE_LIST_COLUMN_WIDTHS);
        backed_up_files = win_common::make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 200);

        backed_up_files.add("File #1|2021-10-17 12:22pm|102kb");
        backed_up_files.add("File #2|2021-10-15 2:22pm|233kb");
        backed_up_files.add("File #3|2021-10-14 8:22pm|12kb");

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
            .emit(sender, Message::RestoreBackup);
       delete_backups_button
            .emit(sender, Message::DeleteBackup);

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

        activate_redirect_button.emit(sender, Message::ActivateRedirect);
        deactivate_redirect_button.emit(sender, Message::DeactivateRedirect);

        redirect_list_buttons.set_size(0, text_size.1 + 10);

        redirect_list_buttons.end();

        content.end();

        wind.end();

        wind.set_callback(|_wind| {
            if app::event() == Event::Close {
                app::quit();
                exit(0);
            }
        });

        MainWindow {
            wind,
            live_files,
            backed_up_files,
            restore_backups_button,
            delete_backups_button,
            redirect_list,
            activate_redirect_button,
            deactivate_redirect_button
        }
    }

}