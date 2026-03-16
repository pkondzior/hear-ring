use std::f32::consts::TAU;

use gpui::{
    canvas, div, point, prelude::*, px, rgba, App, Bounds, Context, MouseButton, Path, PathBuilder,
    Pixels, Point, ReadGlobal, Render, Rgba, Window, WindowControlArea,
};

use crate::{
    runtime::RadarRuntime,
    types::{Direction, DirectionFrame},
};

const SECTOR_HALF_SPAN: f32 = TAU / 16.0;
const RING_STEPS: usize = 18;

#[derive(Clone, Copy)]
struct RingLayout {
    center: Point<Pixels>,
    outer_radius: Pixels,
    inner_radius: Pixels,
}

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
        let paint_frame = runtime.latest_frame.clone();

        div()
            .relative()
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
                canvas(
                    move |bounds, _, _| ring_layout(bounds),
                    move |bounds, layout, window, _| {
                        paint_radar_ring(bounds, layout, &paint_frame, window);
                    },
                )
                .absolute()
                .size_full(),
            )
    }
}

fn ring_layout(bounds: Bounds<Pixels>) -> RingLayout {
    let center = bounds.center();
    let radius = px(f32::from(bounds.size.width).min(f32::from(bounds.size.height)) * 0.49);

    RingLayout {
        center,
        outer_radius: radius,
        inner_radius: px(f32::from(radius) * 0.30),
    }
}

fn paint_radar_ring(
    _bounds: Bounds<Pixels>,
    layout: RingLayout,
    frame: &DirectionFrame,
    window: &mut Window,
) {
    for direction in Direction::ALL {
        let fill_strength = frame.scores[direction].clamp(0.0, 1.0);
        let start = direction.angle() - SECTOR_HALF_SPAN;
        let end = direction.angle() + SECTOR_HALF_SPAN;

        if let Some(path) = ring_segment_path(layout, start, end) {
            window.paint_path(path, segment_fill(fill_strength));
        }

        if let Some(path) = ring_segment_stroke_path(layout, start, end) {
            window.paint_path(path, rgba(0x5a5a5acc));
        }
    }
}

fn segment_fill(strength: f32) -> Rgba {
    if strength > 0.01 {
        let base = (40.0 + 180.0 * strength) / 255.0;
        Rgba {
            r: base,
            g: (base * 0.85).min(1.0),
            b: 240.0 / 255.0,
            a: 0.95,
        }
    } else {
        rgba(0x23232366)
    }
}

fn ring_segment_path(layout: RingLayout, start_angle: f32, end_angle: f32) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::fill();

    for i in 0..=RING_STEPS {
        let t = i as f32 / RING_STEPS as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        let point = polar_point(layout.center, angle, layout.outer_radius);

        if i == 0 {
            builder.move_to(point);
        } else {
            builder.line_to(point);
        }
    }

    for i in (0..=RING_STEPS).rev() {
        let t = i as f32 / RING_STEPS as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        builder.line_to(polar_point(layout.center, angle, layout.inner_radius));
    }

    builder.build().ok()
}

fn ring_segment_stroke_path(
    layout: RingLayout,
    start_angle: f32,
    end_angle: f32,
) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::stroke(px(1.5));

    for i in 0..=RING_STEPS {
        let t = i as f32 / RING_STEPS as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        let point = polar_point(layout.center, angle, layout.outer_radius);

        if i == 0 {
            builder.move_to(point);
        } else {
            builder.line_to(point);
        }
    }

    for i in 0..=RING_STEPS {
        let t = i as f32 / RING_STEPS as f32;
        let angle = end_angle - (end_angle - start_angle) * t;
        builder.line_to(polar_point(layout.center, angle, layout.inner_radius));
    }

    builder.line_to(polar_point(layout.center, start_angle, layout.outer_radius));
    builder.build().ok()
}

fn polar_point(center: Point<Pixels>, angle: f32, radius: Pixels) -> Point<Pixels> {
    point(
        px(f32::from(center.x) + angle.cos() * f32::from(radius)),
        px(f32::from(center.y) + angle.sin() * f32::from(radius)),
    )
}
