use gpui::{
    div, prelude::*, px, relative, rgb, App, Context, ReadGlobal, Render, SharedString,
    UpdateGlobal, Window,
};

use crate::{
    runtime::{RadarRuntime, SourceMode},
    source::AudioSourceState,
    types::{ChannelLayout, Sector8, ORDERED_SECTORS},
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
        .gap_2()
        .child(
            div()
                .w(px(64.))
                .text_sm()
                .text_color(rgb(0xbdbdbd))
                .child(label.to_owned()),
        )
        .child(
            div()
                .w(px(180.))
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
                .w(px(44.))
                .text_sm()
                .text_color(rgb(0xe5e5e5))
                .child(format!("{:>3}%", (fill * 100.0).round() as i32)),
        )
}

fn tune_row(
    base_id: &'static str,
    label: &str,
    value: f32,
    minus: impl Fn(&mut Window, &mut App) + 'static,
    plus: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(px(160.))
                .text_color(rgb(0xbdbdbd))
                .child(label.to_owned()),
        )
        .child(action_button((base_id, 0usize), "-", minus))
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
        .child(action_button((base_id, 1usize), "+", plus))
}

impl Render for OptionsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime = RadarRuntime::global(cx).snapshot();
        let tuning = RadarRuntime::global(cx).tuning();
        let overlay_visible = OverlayWindow::global(cx).is_visible();
        let click_through = OverlayWindow::global(cx).click_through();
        let layout_locked = matches!(runtime.source_mode, SourceMode::SystemAudio);

        div().size_full().bg(rgb(0x171717)).child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .id("options-scroll")
                .overflow_y_scroll()
                .text_color(rgb(0xf4f4f4))
                .child(div().text_xl().child("Sound Radar"))
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
                                            runtime.set_source_mode(SourceMode::SystemAudio);
                                        });
                                    },
                                )),
                        )
                        .child(info_row(
                            "State",
                            source_state_message(&runtime.source_state),
                        ))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xbdbdbd))
                                .child(source_help_text(
                                    runtime.source_mode,
                                    &runtime.source_state,
                                )),
                        )
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
                    "Overlay",
                    div().flex().flex_col().gap_2().child(
                        div()
                            .flex()
                            .gap_2()
                            .child(action_button(
                                "toggle-overlay",
                                if overlay_visible {
                                    "Hide overlay"
                                } else {
                                    "Show overlay"
                                },
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
                            )),
                    ),
                ))
                .child(section(
                    "Tuning",
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(tune_row(
                            "stereo-min-energy",
                            "Stereo min energy",
                            tuning.stereo.min_energy,
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_stereo_min_energy(-0.01);
                                });
                            },
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_stereo_min_energy(0.01);
                                });
                            },
                        ))
                        .child(tune_row(
                            "stereo-max-energy",
                            "Stereo max energy",
                            tuning.stereo.max_energy,
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_stereo_max_energy(-0.05);
                                });
                            },
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_stereo_max_energy(0.05);
                                });
                            },
                        ))
                        .child(tune_row(
                            "stereo-pan-gain",
                            "Stereo pan gain",
                            tuning.stereo.pan_gain,
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_stereo_pan_gain(-0.05);
                                });
                            },
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_stereo_pan_gain(0.05);
                                });
                            },
                        ))
                        .child(tune_row(
                            "smoother-attack",
                            "Attack",
                            tuning.smoother.attack_alpha,
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_attack(-0.01);
                                });
                            },
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_attack(0.01);
                                });
                            },
                        ))
                        .child(tune_row(
                            "smoother-decay",
                            "Decay",
                            tuning.smoother.decay_alpha,
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_decay(-0.01);
                                });
                            },
                            |_window, cx| {
                                RadarRuntime::update_global(cx, |runtime, _| {
                                    runtime.adjust_decay(0.01);
                                });
                            },
                        )),
                ))
                .child(section(
                    "Diagnostics",
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(info_row("Input mode", runtime.layout.label()))
                        .child(meter_row(
                            "energy-fl",
                            "FL / L",
                            runtime.latest_energies.fl.max(runtime.latest_energies.sl),
                        ))
                        .child(meter_row(
                            "energy-fr",
                            "FR / R",
                            runtime.latest_energies.fr.max(runtime.latest_energies.sr),
                        ))
                        .child(meter_row("energy-c", "C", runtime.latest_energies.c))
                        .child(meter_row("energy-sl", "SL", runtime.latest_energies.sl))
                        .child(meter_row("energy-sr", "SR", runtime.latest_energies.sr))
                        .child(meter_row("energy-rl", "RL", runtime.latest_energies.rl))
                        .child(meter_row("energy-rr", "RR", runtime.latest_energies.rr))
                        .child(meter_row("energy-lfe", "LFE", runtime.latest_energies.lfe))
                        .children(matches!(runtime.layout, ChannelLayout::Stereo).then(|| {
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(info_row(
                                    "Pan",
                                    format!("{:+.2}", runtime.latest_energies.stereo_pan),
                                ))
                                .child(info_row(
                                    "Smoothed pan",
                                    format!("{:+.2}", runtime.stereo_smoothed_pan),
                                ))
                                .child(info_row(
                                    "Width",
                                    format!("{:.2}", runtime.latest_energies.stereo_width),
                                ))
                        }))
                        .child(info_row(
                            "Dominant sector",
                            runtime
                                .latest_frame
                                .dominant_sector()
                                .map(Sector8::label)
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
                ))
                .child(section(
                    "Sector scores",
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(ORDERED_SECTORS.into_iter().map(|sector| {
                            meter_row(
                                &format!("score-{}", sector.label()),
                                sector.label(),
                                runtime.latest_frame.scores[sector.index()],
                            )
                        })),
                )),
        )
    }
}
