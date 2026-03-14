use gpui::{
    div, prelude::*, rgb, rgba, App, Context, MouseButton, ReadGlobal, Render, Window,
    WindowControlArea,
};

use crate::runtime::RadarRuntime;

pub struct OverlayView;

impl OverlayView {
    pub fn new(cx: &mut App) -> gpui::Entity<Self> {
        cx.new(|cx| {
            cx.observe_global::<RadarRuntime>(|_, cx| cx.notify())
                .detach();
            Self
        })
    }
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime = RadarRuntime::global(cx).snapshot();
        let dominant_sector = runtime
            .latest_frame
            .dominant_sector()
            .map(|sector| sector.label())
            .unwrap_or("-");

        div()
            .window_control_area(WindowControlArea::Drag)
            .on_mouse_down(MouseButton::Left, |_event, window, _cx| {
                window.start_window_move();
            })
            .size_full()
            .bg(gpui::transparent_black())
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .items_center()
                    .px_6()
                    .py_5()
                    .rounded_xl()
                    .bg(rgba(0x111827cc))
                    .border_1()
                    .border_color(rgba(0xffffff22))
                    .text_color(rgb(0xf9fafb))
                    .child(div().text_lg().child("Radar overlay"))
                    .child(format!("Sector: {dominant_sector}"))
                    .child(format!(
                        "Confidence: {:.2}",
                        runtime.latest_frame.confidence
                    ))
                    .child(format!("Intensity: {:.2}", runtime.latest_frame.intensity)),
            )
    }
}
