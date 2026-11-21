//! The standalone window stays mounted when hidden from the desktop.

use nsh_core::desktop::CloseReason;
use nsh_core::element::{
    ImageSrc,
    desktop::{MenuItem, current_window, on_quit_request, quit, set_tray, tray},
};
use nsh_core::platform::Os;
use nsh_core::prelude::*;

#[derive(Clone, Copy)]
struct QuitCheck(Stored<Option<Box<dyn Fn() -> bool>>>);

pub(crate) fn install_tray() {
    if !matches!(os(), Os::Windows | Os::MacOs) {
        return;
    }
    let check = QuitCheck(stored(None));
    provide_context(check);
    let window = current_window();
    let show = window.clone();
    let show_from_menu = window.clone();
    let on_close = window.clone();
    window.on_close_press(move |reason| {
        if reason == CloseReason::System {
            on_close.close();
        } else {
            on_close.hide();
        }
    });
    let quit_from = window.clone();
    let request_quit = move || {
        if check
            .0
            .with(|check| check.as_ref().is_none_or(|can_quit| can_quit()))
        {
            quit();
        } else {
            // Let the user see the retained draft and the save error.
            quit_from.focus();
        }
    };
    on_quit_request(request_quit.clone());
    set_tray(
        tray()
            .icon(ImageSrc::svg_markup(include_str!("../assets/tray.svg")))
            .tooltip("Kanon")
            .on_click(move |_| show.focus())
            .menu([
                MenuItem::new("显示 Kanon").on_select(move || show_from_menu.focus()),
                MenuItem::separator(),
                MenuItem::new("退出 Kanon").on_select(request_quit),
            ]),
    );
}

/// Only the mounted editor can have a pending draft; it owns this callback.
pub(crate) fn check_before_quit(can_quit: impl Fn() -> bool + 'static) {
    if let Some(check) = use_context::<QuitCheck>() {
        check.0.replace(Some(Box::new(can_quit)));
        cleanup(move || {
            check.0.try_with(|callback| *callback = None);
        });
    }
}
