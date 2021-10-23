use fltk::prelude::WidgetExt;
use fltk::window::Window;
use crate::settings::Settings;

pub fn make_settings_window(settings: Settings) -> Window {
    static WINDOW_SIZE: (i32, i32) = (800, 665);
    static CONTENT_SIZE: (i32, i32) = (WINDOW_SIZE.0 - 20, WINDOW_SIZE.1 - 20);

    let mut wind = Window::default().with_label("Settings");
    wind.set_size(WINDOW_SIZE.0, WINDOW_SIZE.1);

    wind
}