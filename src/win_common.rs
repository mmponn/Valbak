/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::max;

use fltk::browser::MultiBrowser;
use fltk::enums::Font;
use fltk::frame::Frame;
use fltk::group::{Pack, PackType};
use fltk::prelude::{BrowserExt, GroupExt, WidgetExt};

pub fn make_section_header(header_text: &str, space_before: bool) {
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

pub fn column_headers(column_header_texts: &Vec<&str>, column_header_widths: &'static[i32]) {
    let column_header_widths = Vec::from(column_header_widths);
    let mut max_header_width = 0;
    let mut max_header_height = 0;

    let mut column_headers_pack = Pack::default()
        .size_of_parent()
        .with_type(PackType::Horizontal);

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

    column_headers_pack.end();
}

pub fn make_list_browser(column_widths: &'static[i32], list_height: i32) -> MultiBrowser {
    let mut list = MultiBrowser::default()
        .with_size(0, list_height)
        .with_pos(10, 10);
    list.set_column_char('|');
    list.set_column_widths(column_widths);
    list
}