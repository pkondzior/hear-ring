use std::f32::consts::TAU;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Shape, Stroke, Vec2};

use crate::types::{DirectionFrame, ORDERED_SECTORS};

pub fn draw_direction_ring(painter: &egui::Painter, rect: egui::Rect, frame: &DirectionFrame) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.46;
    let inner_radius = radius * 0.56;
    let stroke = Stroke::new(1.5, Color32::from_gray(90));

    for sector in ORDERED_SECTORS {
        let idx = sector.index();
        let fill_strength = frame.scores[idx].clamp(0.0, 1.0);
        let start = sector.angle() - TAU / 16.0;
        let end = sector.angle() + TAU / 16.0;

        let base = 40.0 + 180.0 * fill_strength;
        let color = if fill_strength > 0.01 {
            Color32::from_rgb(base as u8, (base * 0.85) as u8, 240)
        } else {
            Color32::from_gray(35)
        };

        let shape = ring_segment(center, inner_radius, radius, start, end, color, stroke);
        painter.add(shape);

        let label_pos = center + Vec2::angled(sector.angle()) * (radius + 22.0);
        painter.text(
            label_pos,
            Align2::CENTER_CENTER,
            sector.label(),
            FontId::proportional(18.0),
            Color32::WHITE,
        );
    }

    painter.circle_filled(center, inner_radius - 8.0, Color32::from_gray(20));
    painter.text(
        center,
        Align2::CENTER_CENTER,
        format!(
            "{}\nconf {:.2}",
            frame
                .dominant_sector()
                .map(|sector| sector.label())
                .unwrap_or("-"),
            frame.confidence
        ),
        FontId::proportional(22.0),
        Color32::WHITE,
    );
}

pub fn energy_bar(ui: &mut egui::Ui, label: &str, value: f32) {
    ui.horizontal(|ui| {
        ui.label(format!("{label:>8}"));
        let bar = egui::ProgressBar::new(value.clamp(0.0, 1.0))
            .desired_width(180.0)
            .show_percentage();
        ui.add(bar);
    });
}

fn ring_segment(
    center: Pos2,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    end_angle: f32,
    fill: Color32,
    stroke: Stroke,
) -> Shape {
    let steps = 18;
    let mut points = Vec::with_capacity((steps + 1) * 2);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        points.push(center + Vec2::angled(angle) * outer_radius);
    }

    for i in (0..=steps).rev() {
        let t = i as f32 / steps as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        points.push(center + Vec2::angled(angle) * inner_radius);
    }

    Shape::convex_polygon(points, fill, stroke)
}
