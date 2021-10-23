use std::cmp::max;
use fltk::{
    app::*,
    browser::*,
    button::*,
    enums::*,
    input::*,
    prelude::*,
    window::*,
    group::*
};
use fltk::frame::Frame;
use fltk::menu::{MenuBar, MenuFlag};
use fltk::misc::Tooltip;

pub fn make_main_window() -> Window {
    static WINDOW_SIZE: (i32, i32) = (800, 665);
    static CONTENT_SIZE: (i32, i32) = (WINDOW_SIZE.0 - 20, WINDOW_SIZE.1 - 20);

    let mut wind = Window::default().with_label("Valbak");
    wind.set_size(WINDOW_SIZE.0, WINDOW_SIZE.1);

    let mut menu = MenuBar::default()
        .with_label("Menu");
    let text_size = menu.measure_label();
    menu.set_size(WINDOW_SIZE.0, text_size.1 + 10);
    menu.add("File/Settings", Shortcut::None, MenuFlag::Normal, |_menu_bar| println!("Callback!"));
    menu.add("File/Quit", Shortcut::None, MenuFlag::Normal, |_menu_bar| println!("Callback!"));
    menu.add("Help/Documentation", Shortcut::None, MenuFlag::Normal, |_menu_bar| println!("Callback!"));
    menu.add("Help/About", Shortcut::None, MenuFlag::Normal, |_menu_bar| println!("Callback!"));

    let mut live_files;
    let mut backed_up_files;
    let redirect_list;

    let mut content = Pack::default()
        // .with_size(window_size.0 - 20, window_size.1 - 20)
        .with_size(CONTENT_SIZE.0, CONTENT_SIZE.1)
        .with_pos(10, 10 + 20);
    content.set_spacing(5);
    {
        static FILE_LIST_COLUMN_WIDTHS: [i32; 3] = [CONTENT_SIZE.0 - 300, 200, 100];
        let file_header_texts: Vec<&str> = vec!("File", "File Date", "File Size");

        // Live Files
        make_section_header("Live Files", true);
        column_headers(&file_header_texts, &FILE_LIST_COLUMN_WIDTHS);
        live_files = make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 100);

        live_files.add("File #1|2021-10-17 12:22pm|102kb");
        live_files.add("File #2|2021-10-15 2:22pm|233kb");
        live_files.add("File #3|2021-10-14 8:22pm|12kb");

        live_files.set_selection_color(Color::White);

        // Backed-Up Files
        make_section_header("Backed-Up Files", true);
        column_headers(&file_header_texts, &FILE_LIST_COLUMN_WIDTHS);
        backed_up_files = make_list_browser(&FILE_LIST_COLUMN_WIDTHS, 200);

        backed_up_files.add("File #1|2021-10-17 12:22pm|102kb");
        backed_up_files.add("File #2|2021-10-15 2:22pm|233kb");
        backed_up_files.add("File #3|2021-10-14 8:22pm|12kb");

        let mut backed_up_files_buttons = Pack::default()
            .with_type(PackType::Horizontal);
        backed_up_files_buttons.set_spacing(5);
        {
            let mut restore_backups_button = Button::default()
                .with_label("Restore");
            let text_size = restore_backups_button.measure_label();
            restore_backups_button.set_size(text_size.0 + 15, text_size.1 + 10);
            let mut delete_backups_button = Button::default()
                .with_label("Delete");
            let text_size = delete_backups_button.measure_label();
            delete_backups_button.set_size(text_size.0 + 15, text_size.1 + 10);

            backed_up_files_buttons.set_size(0, text_size.1 + 10);
        }
        backed_up_files_buttons.end();

        // Redirects
        static REDIRECT_COLUMN_WIDTHS: [i32; 3] = [(CONTENT_SIZE.0 - 50) / 2, (CONTENT_SIZE.0 - 50) / 2, 50];
        let redirect_header_texts = vec!("Source Directory", "Destination Directory", "Active");

        make_section_header("Redirects", true);
        column_headers(&redirect_header_texts, &REDIRECT_COLUMN_WIDTHS);
        redirect_list = make_list_browser(&REDIRECT_COLUMN_WIDTHS, 100);

        let mut redirect_list_buttons = Pack::default()
            .with_type(PackType::Horizontal);
        redirect_list_buttons.set_spacing(5);
        {
            let mut activate_redirect_button = Button::default()
                .with_label("Activate");
            let text_size = activate_redirect_button.measure_label();
            activate_redirect_button.set_size(text_size.0 + 15, text_size.1 + 10);
            let mut deactivate_redirect_button = Button::default()
                .with_label("Deactivate");
            let text_size = deactivate_redirect_button.measure_label();
            deactivate_redirect_button.set_size(text_size.0 + 15, text_size.1 + 10);

            redirect_list_buttons.set_size(0, text_size.1 + 10);
        }
        redirect_list_buttons.end();
    }
    content.end();

    wind.end();
    // wind.make_resizable(true);
    wind
}

fn make_section_header(header_text: &str, space_before: bool) {
    if space_before {
        Frame::default().with_size(0, 5);
    }

    let mut section_label_pack = Pack::default()
        .size_of_parent()
        .with_type(PackType::Horizontal);
    {
        let mut section_label = Frame::default()
            .with_label(header_text);
        section_label.set_label_font(Font::HelveticaBold);
        section_label.set_label_size(12);
        let text_size = section_label.measure_label();
        section_label.set_size(text_size.0, text_size.1);
        section_label_pack.set_size(text_size.0, text_size.1);
    }
    section_label_pack.end();
}

type WidgetWidth = i32;
type WidgetHeight = i32;

fn column_headers(column_header_texts: &Vec<&str>, column_header_widths: &'static[i32]) {
    let column_header_widths = Vec::from(column_header_widths);
    let mut max_header_width = 0;
    let mut max_header_height = 0;

    let mut column_headers_pack = Pack::default()
        .size_of_parent()
        .with_type(PackType::Horizontal);
    {
        for (header_text, header_width) in column_header_texts.iter().zip(column_header_widths) {
            let mut adjusted_header_text = String::from(" ");
            adjusted_header_text.push_str(header_text);

            let mut label_frame = Frame::default()
                .with_label(&*adjusted_header_text);
            label_frame.set_label_size(12);
            let text_size = label_frame.measure_label();
            label_frame.set_size(text_size.0, text_size.1);

            Frame::default()
                .with_size(header_width - text_size.0, text_size.1);

            max_header_width = max(text_size.0, max_header_width);
            max_header_height = max(text_size.1, max_header_height);
        }
        column_headers_pack.set_size(max_header_width, max_header_height);
    }
    column_headers_pack.end();
}

fn make_list_browser(column_widths: &'static[i32], list_height: i32) -> MultiBrowser {
    let mut list = MultiBrowser::default()
        .with_size(0, list_height)
        .with_pos(10, 10);
    list.set_column_char('|');
    list.set_column_widths(column_widths);
    list
}
