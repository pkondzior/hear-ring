use gpui::{
    div, prelude::*, px, relative, rgb, App, Context, Entity, ReadGlobal, Render, SharedString,
    UpdateGlobal, Window,
};
use gpui_component::slider::{Slider, SliderEvent, SliderState, SliderValue};

use crate::{
    runtime::{RadarRuntime, SourceMode},
    source::AudioSourceState,
    types::{ChannelLayout, Direction, EnergyChannel},
    ui::overlay_window::OverlayWindow,
};

pub struct OptionsView;

impl OptionsView {
    pub fn new(cx: &mut App) -> gpui::Entity<Self> {
        cx.new(|cx| {
            cx.observe_global::<RadarRuntime>(|_, cx| cx.notify())
                .detach();
            cx.observe_global::<OverlayWindow>(|_, cx| cx.notify())
                .detach();
            Self
        })
    }
}

fn action_button(
    id: impl Into<gpui::ElementId>,
    label: &str,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_sm()
        .bg(rgb(0x2b5cff))
        .hover(|style| style.bg(rgb(0x4f77ff)))
        .active(|style| style.bg(rgb(0x244fd6)))
        .text_color(rgb(0xffffff))
        .cursor_pointer()
        .child(label.to_owned())
        .on_click(move |_, window, cx| on_click(window, cx))
}

fn chip(
    id: impl Into<gpui::ElementId>,
    label: &str,
    selected: bool,
    disabled: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let background = if selected {
        rgb(0x2b5cff)
    } else {
        rgb(0x262626)
    };
    let text = if selected {
        rgb(0xffffff)
    } else {
        rgb(0xe5e5e5)
    };

    let mut element = div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_sm()
        .bg(background)
        .text_color(text)
        .border_1()
        .border_color(rgb(0x3a3a3a))
        .child(label.to_owned());

    if disabled {
        element = element.opacity(0.45);
    } else {
        element = element
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0x333333)))
            .active(|style| style.bg(rgb(0x404040)))
            .on_click(move |_, window, cx| on_click(window, cx));
    }

    element
}

fn info_row(label: &str, value: impl Into<String>) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .gap_3()
        .child(div().text_color(rgb(0xbdbdbd)).child(label.to_owned()))
        .child(div().text_color(rgb(0xf4f4f4)).child(value.into()))
}

fn source_state_message(state: &AudioSourceState) -> String {
    match state.detail() {
        Some(detail) => format!("{}: {}", state.label(), detail),
        None => state.label().to_owned(),
    }
}

fn source_help_text(mode: SourceMode, state: &AudioSourceState) -> &'static str {
    match (mode, state) {
        (SourceMode::Demo, _) => {
            "Demo source generates synthetic directional energy so the UI and estimator path can be tested."
        }
        (SourceMode::SystemAudio, AudioSourceState::Running) => {
            "System audio capture is active. ScreenCaptureKit is feeding the stereo analysis pipeline."
        }
        (SourceMode::SystemAudio, AudioSourceState::Starting) => {
            "Starting ScreenCaptureKit audio capture."
        }
        (SourceMode::SystemAudio, AudioSourceState::PermissionDenied) => {
            "Grant Screen Recording permission in Privacy & Security and restart the app."
        }
        (SourceMode::SystemAudio, AudioSourceState::UnsupportedPlatform) => {
            "System audio capture is only available on macOS."
        }
        (SourceMode::SystemAudio, AudioSourceState::Error(_)) => {
            "ScreenCaptureKit failed to start. Check permissions and try again."
        }
    }
}

fn section(title: &str, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_lg()
        .bg(rgb(0x1f1f1f))
        .border_1()
        .border_color(rgb(0x2d2d2d))
        .child(
            div()
                .text_base()
                .text_color(rgb(0xf4f4f4))
                .child(title.to_owned()),
        )
        .child(body)
}

fn meter_row(id: &str, label: &str, value: f32) -> impl IntoElement {
    let fill = value.clamp(0.0, 1.0);

    div()
        .id(SharedString::from(id.to_owned()))
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(rgb(0xbdbdbd))
                .child(label.to_owned()),
        )
        .child(
            div()
                .w(px(100.))
                .h(px(12.))
                .rounded_sm()
                .bg(rgb(0x101010))
                .border_1()
                .border_color(rgb(0x2d2d2d))
                .child(
                    div()
                        .h_full()
                        .w(relative(fill))
                        .rounded_sm()
                        .bg(rgb(0x2b90ff)),
                ),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0xe5e5e5))
                .child(format!("{:>3}%", (fill * 100.0).round() as i32)),
        )
}

struct TuningSliderState {
    slider: Entity<SliderState>,
    _subscription: gpui::Subscription,
}

fn tune_slider_row(
    slider_id: &'static str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    window: &mut Window,
    cx: &mut Context<OptionsView>,
    on_change: fn(&mut RadarRuntime, f32),
) -> impl IntoElement {
    let slider = {
        let on_change = on_change;
        let state = window
            .use_keyed_state(slider_id, cx, |_, cx| {
                let slider = cx.new(|_| {
                    SliderState::new()
                        .min(min)
                        .max(max)
                        .step(step)
                        .default_value(value)
                });
                let _subscription = cx.subscribe(&slider, move |_, _, event: &SliderEvent, cx| {
                    if let SliderEvent::Change(SliderValue::Single(value)) = event {
                        RadarRuntime::update_global(cx, |runtime, _| on_change(runtime, *value));
                    }
                });

                TuningSliderState {
                    slider,
                    _subscription,
                }
            })
            .read(cx);

        state.slider.clone()
    };

    slider.update(cx, |state, cx| {
        if state.value() != SliderValue::Single(value) {
            state.set_value(value, window, cx);
        }
    });

    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(160.))
                .text_color(rgb(0xbdbdbd))
                .child(label.to_owned()),
        )
        .child(Slider::new(&slider).w(px(220.)).h(px(24.)))
        .child(
            div()
                .w(px(72.))
                .px_2()
                .py_2()
                .rounded_sm()
                .bg(rgb(0x101010))
                .border_1()
                .border_color(rgb(0x2d2d2d))
                .text_color(rgb(0xf4f4f4))
                .child(format!("{value:.3}")),
        )
}

impl Render for OptionsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime = RadarRuntime::global(cx).snapshot();
        let tuning = RadarRuntime::global(cx).tuning();
        let overlay_visible = OverlayWindow::global(cx).is_visible();
        let click_through = OverlayWindow::global(cx).click_through();
        let overlay_always_on_top = OverlayWindow::global(cx).always_on_top();
        let layout_locked = matches!(runtime.source_mode, SourceMode::SystemAudio);

        div().size_full().bg(rgb(0x171717)).child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap_3()
                .px_4()
                .pb_4()
                .pt(px(30.))
                .text_color(rgb(0xf4f4f4))
                .child(div().text_xl().child("Sound Hearing Aid"))
                .child(
                    div()
                        .id("options-scroll")
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .overflow_y_scroll()
                        .child(section(
                            "Source",
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .items_center()
                                        .child(chip(
                                            "source-demo",
                                            "Demo",
                                            runtime.source_mode == SourceMode::Demo,
                                            false,
                                            |_window, cx| {
                                                RadarRuntime::update_global(cx, |runtime, _| {
                                                    runtime.set_source_mode(SourceMode::Demo);
                                                });
                                            },
                                        ))
                                        .child(chip(
                                            "source-system",
                                            "System Audio",
                                            runtime.source_mode == SourceMode::SystemAudio,
                                            false,
                                            |_window, cx| {
                                                RadarRuntime::update_global(cx, |runtime, _| {
                                                    runtime
                                                        .set_source_mode(SourceMode::SystemAudio);
                                                });
                                            },
                                        )),
                                )
                                .child(info_row(
                                    "State",
                                    source_state_message(&runtime.source_state),
                                ))
                                .child(div().text_sm().text_color(rgb(0xbdbdbd)).child(
                                    source_help_text(runtime.source_mode, &runtime.source_state),
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .items_center()
                                        .child(chip(
                                            "layout-stereo",
                                            "Stereo",
                                            runtime.layout == ChannelLayout::Stereo,
                                            layout_locked,
                                            |_window, cx| {
                                                RadarRuntime::update_global(cx, |runtime, _| {
                                                    runtime.set_layout(ChannelLayout::Stereo);
                                                });
                                            },
                                        ))
                                        .child(chip(
                                            "layout-surround",
                                            "7.1",
                                            runtime.layout == ChannelLayout::Surround71,
                                            layout_locked,
                                            |_window, cx| {
                                                RadarRuntime::update_global(cx, |runtime, _| {
                                                    runtime.set_layout(ChannelLayout::Surround71);
                                                });
                                            },
                                        ))
                                        .when(layout_locked, |this| {
                                            this.child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0x9ca3af))
                                                    .child("(fixed by source)"),
                                            )
                                        }),
                                ),
                        ))
                        .child(section(
                            "Hear Ring",
                            div()
                                .flex()
                                .gap_2()
                                .child(action_button(
                                    "toggle-hear-ring",
                                    if overlay_visible { "Hide" } else { "Show" },
                                    |_window, cx| {
                                        OverlayWindow::update_global(cx, |overlay, cx| {
                                            overlay.set_visible(cx, !overlay.is_visible());
                                        });
                                    },
                                ))
                                .child(action_button(
                                    "toggle-click-through",
                                    if click_through {
                                        "Draggable"
                                    } else {
                                        "Click-through"
                                    },
                                    |_window, cx| {
                                        OverlayWindow::update_global(cx, |overlay, cx| {
                                            overlay.set_click_through(cx, !overlay.click_through());
                                        });
                                    },
                                ))
                                .child(action_button(
                                    "toggle-topmost",
                                    if overlay_always_on_top {
                                        "Unpin"
                                    } else {
                                        "Pin"
                                    },
                                    |_window, cx| {
                                        OverlayWindow::update_global(cx, |overlay, cx| {
                                            overlay.set_always_on_top(cx, !overlay.always_on_top());
                                        });
                                    },
                                )),
                        ))
                        .child(section(
                            "Tuning",
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(tune_slider_row(
                                    "stereo-min-energy-slider",
                                    "Stereo min energy",
                                    tuning.stereo.min_energy,
                                    0.0,
                                    0.25,
                                    0.01,
                                    window,
                                    cx,
                                    RadarRuntime::set_stereo_min_energy,
                                ))
                                .child(tune_slider_row(
                                    "stereo-max-energy-slider",
                                    "Stereo max energy",
                                    tuning.stereo.max_energy,
                                    0.1,
                                    2.5,
                                    0.05,
                                    window,
                                    cx,
                                    RadarRuntime::set_stereo_max_energy,
                                ))
                                .child(tune_slider_row(
                                    "stereo-pan-gain-slider",
                                    "Stereo pan gain",
                                    tuning.stereo.pan_gain,
                                    0.5,
                                    4.0,
                                    0.05,
                                    window,
                                    cx,
                                    RadarRuntime::set_stereo_pan_gain,
                                ))
                                .child(tune_slider_row(
                                    "smoother-attack-slider",
                                    "Attack",
                                    tuning.smoother.attack_alpha,
                                    0.0,
                                    1.0,
                                    0.01,
                                    window,
                                    cx,
                                    RadarRuntime::set_attack_alpha,
                                ))
                                .child(tune_slider_row(
                                    "smoother-decay-slider",
                                    "Decay",
                                    tuning.smoother.decay_alpha,
                                    0.0,
                                    1.0,
                                    0.01,
                                    window,
                                    cx,
                                    RadarRuntime::set_decay_alpha,
                                )),
                        ))
                        .child(section(
                            "Diagnostics",
                            div()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(info_row("Input mode", runtime.layout.label()))
                                        .children(
                                            matches!(runtime.layout, ChannelLayout::Stereo).then(
                                                || {
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_1()
                                                        .child(info_row(
                                                            "Pan",
                                                            format!(
                                                                "{:+.2}",
                                                                runtime.latest_energies.stereo_pan
                                                            ),
                                                        ))
                                                        .child(info_row(
                                                            "Smoothed pan",
                                                            format!(
                                                                "{:+.2}",
                                                                runtime.stereo_smoothed_pan
                                                            ),
                                                        ))
                                                        .child(info_row(
                                                            "Width",
                                                            format!(
                                                                "{:.2}",
                                                                runtime
                                                                    .latest_energies
                                                                    .stereo_width
                                                            ),
                                                        ))
                                                },
                                            ),
                                        )
                                        .child(info_row(
                                            "Dominant direction",
                                            runtime
                                                .latest_frame
                                                .dominant_direction()
                                                .map(Direction::label)
                                                .unwrap_or("-"),
                                        ))
                                        .child(info_row(
                                            "Confidence",
                                            format!("{:.2}", runtime.latest_frame.confidence),
                                        ))
                                        .child(info_row(
                                            "Intensity",
                                            format!("{:.2}", runtime.latest_frame.intensity),
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex_1()
                                                .flex()
                                                .flex_col()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(rgb(0xf4f4f4))
                                                        .child("Energy levels"),
                                                )
                                                .children(EnergyChannel::ALL.into_iter().map(
                                                    |channel: EnergyChannel| {
                                                        meter_row(
                                                            channel.id(),
                                                            channel.label(),
                                                            channel.value(&runtime.latest_energies),
                                                        )
                                                    },
                                                )),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .flex()
                                                .flex_col()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(rgb(0xf4f4f4))
                                                        .child("Direction scores"),
                                                )
                                                .children(Direction::ALL.into_iter().map(
                                                    |direction| {
                                                        meter_row(
                                                            &format!("score-{}", direction.label()),
                                                            direction.label(),
                                                            runtime.latest_frame.scores[direction],
                                                        )
                                                    },
                                                )),
                                        ),
                                ),
                        )),
                ),
        )
    }
}
