use objc2::rc::Retained;
use objc2_app_kit::{
    NSView, NSWindow, NSWindowCollectionBehavior, NSWindowLevel, NSWindowStyleMask,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::platform::WindowExt;

fn get_ns_window(window: &gpui::Window) -> Retained<NSWindow> {
    let handle = HasWindowHandle::window_handle(window).unwrap().as_raw();
    let ns_view: Retained<NSView> = match handle {
        RawWindowHandle::AppKit(handle) => unsafe {
            Retained::retain(handle.ns_view.as_ptr().cast()).expect("Failed to get NSView")
        },
        _ => unreachable!(),
    };

    ns_view
        .window()
        .expect("NSView is not attached to an NSWindow")
}

const OVERLAY_WINDOW_LEVEL: NSWindowLevel = objc2_app_kit::NSPopUpMenuWindowLevel + 1;

impl WindowExt for gpui::Window {
    fn setup_options_window(&self) {
        get_ns_window(self).setLevel(OVERLAY_WINDOW_LEVEL + 1);
    }

    fn setup_overlay_window(&self) {
        let ns_window = get_ns_window(self);

        ns_window.setLevel(OVERLAY_WINDOW_LEVEL);
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllApplications
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        ns_window.setHasShadow(false);
        ns_window.setStyleMask(
            NSWindowStyleMask::Borderless
                | NSWindowStyleMask::NonactivatingPanel
                | NSWindowStyleMask::FullSizeContentView,
        );

        let screen_frame = ns_window.screen().expect("Failed to get screen").frame();
        let mut window_pos = screen_frame.origin;
        window_pos.y += screen_frame.size.height;
        ns_window.setFrameTopLeftPoint(window_pos);
    }

    fn set_hidden(&self, hidden: bool) {
        let ns_window = get_ns_window(self);

        if hidden {
            ns_window.orderOut(Some(&ns_window));
        } else {
            ns_window.makeKeyAndOrderFront(Some(&ns_window));
        }
    }

    fn set_ignore_cursor_events(&self, ignore: bool) {
        get_ns_window(self).setIgnoresMouseEvents(ignore);
    }

    fn set_window_draggable(&self, draggable: bool) {
        get_ns_window(self).setMovableByWindowBackground(draggable);
    }
}
