//! Hand-drawn rail icons, so the app needs no emoji or image assets.

use eframe::egui::{
    self, Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Write,
    Corkboard,
    Outline,
    Research,
    Search,
    Database,
    History,
    Project,
    Compile,
    Agent,
    Settings,
}

pub fn draw(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = Stroke::new(1.55, color);
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.32;
    let p = |x: f32, y: f32| Pos2::new(c.x + x * r, c.y + y * r);
    let line = |a: Pos2, b: Pos2| painter.line_segment([a, b], s);
    match icon {
        Icon::Write => {
            let page = Rect::from_center_size(c + Vec2::new(-1.0, 1.0), Vec2::splat(r * 1.7));
            painter.rect_stroke(page, CornerRadius::same(2), s, StrokeKind::Inside);
            line(p(-0.55, -0.35), p(0.35, -0.35));
            line(p(-0.55, 0.0), p(0.2, 0.0));
            line(p(-0.55, 0.35), p(0.0, 0.35));
            line(p(0.15, 0.75), p(0.85, 0.05));
            line(p(0.72, -0.08), p(0.92, 0.12));
        }
        Icon::Corkboard => {
            for (x, y) in [(-0.7, -0.7), (0.1, -0.7), (-0.7, 0.1), (0.1, 0.1)] {
                let card = Rect::from_min_size(p(x, y), Vec2::splat(r * 0.7));
                painter.rect_filled(card, CornerRadius::same(2), color.gamma_multiply(0.18));
                painter.rect_stroke(card, CornerRadius::same(2), s, StrokeKind::Inside);
            }
        }
        Icon::Outline => {
            for (i, y) in [-0.7, 0.0, 0.7].iter().enumerate() {
                let x = if i == 1 { -0.35 } else { -0.6 };
                painter.circle_filled(p(x, *y), r * 0.09, color);
                line(p(x + 0.3, *y), p(0.8 - i as f32 * 0.12, *y));
            }
        }
        Icon::Research => {
            // An open book.
            line(p(0.0, -0.6), p(0.0, 0.75));
            painter.add(egui::Shape::line(vec![p(0.0, -0.6), p(-0.55, -0.8), p(-0.95, -0.6), p(-0.95, 0.6), p(-0.55, 0.4), p(0.0, 0.75)], s));
            painter.add(egui::Shape::line(vec![p(0.0, -0.6), p(0.55, -0.8), p(0.95, -0.6), p(0.95, 0.6), p(0.55, 0.4), p(0.0, 0.75)], s));
        }
        Icon::Search => {
            painter.circle_stroke(p(-0.15, -0.15), r * 0.75, s);
            line(p(0.4, 0.4), p(0.95, 0.95));
        }
        Icon::Database => {
            let db = Rect::from_min_max(p(-0.9, -0.75), p(0.9, 0.75));
            painter.rect_stroke(db, CornerRadius::same(2), s, StrokeKind::Inside);
            line(p(-0.9, -0.25), p(0.9, -0.25));
            line(p(-0.9, 0.25), p(0.9, 0.25));
            line(p(-0.3, -0.75), p(-0.3, 0.75));
        }
        Icon::History => {
            let circle = Rect::from_center_size(c, Vec2::splat(r * 1.6));
            painter.add(egui::Shape::line(
                vec![
                    p(0.75, 0.0),
                    p(0.53, -0.53),
                    p(0.0, -0.75),
                    p(-0.53, -0.53),
                    p(-0.75, 0.0),
                    p(-0.53, 0.53),
                    p(0.0, 0.75),
                ],
                s,
            ));
            painter.add(egui::Shape::convex_polygon(
                vec![p(0.2, -0.75), p(0.85, -0.55), p(0.55, 0.1)],
                color,
                Stroke::NONE,
            ));
            let _ = circle;
        }
        Icon::Project => {
            // A title card: a sheet with a heading rule and two lines of detail.
            let sheet = Rect::from_center_size(c, Vec2::new(r * 1.7, r * 2.0));
            painter.rect_stroke(sheet, CornerRadius::same(2), s, StrokeKind::Inside);
            line(p(-0.5, -0.45), p(0.5, -0.45));
            line(p(-0.5, 0.15), p(0.3, 0.15));
            line(p(-0.5, 0.5), p(0.1, 0.5));
        }
        Icon::Compile => {
            line(p(0.0, -0.9), p(0.0, 0.25));
            painter.add(egui::Shape::convex_polygon(
                vec![p(-0.42, 0.0), p(0.42, 0.0), p(0.0, 0.55)],
                color,
                Stroke::NONE,
            ));
            line(p(-0.9, 0.45), p(-0.9, 0.9));
            line(p(-0.9, 0.9), p(0.9, 0.9));
            line(p(0.9, 0.45), p(0.9, 0.9));
        }
        Icon::Agent => {
            let box_rect = Rect::from_center_size(c, Vec2::new(r * 1.8, r * 1.5));
            painter.rect_stroke(box_rect, CornerRadius::same(2), s, StrokeKind::Inside);
            line(p(-0.55, -0.3), p(-0.15, 0.0));
            line(p(-0.55, 0.3), p(-0.15, 0.0));
            line(p(0.05, 0.35), p(0.6, 0.35));
        }
        Icon::Settings => {
            painter.circle_stroke(c, r * 0.55, s);
            for tick in 0..6 {
                let angle = std::f32::consts::TAU * tick as f32 / 6.0;
                let dir = Vec2::new(angle.cos(), angle.sin());
                line(c + dir * (r * 0.78), c + dir * (r * 1.12));
            }
        }
    }
}
