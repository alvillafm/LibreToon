use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Curve {
    gamma: f32,
}
impl Default for Curve {
    fn default() -> Self {
        Self { gamma: 1. }
    }
}
impl Curve {
    pub(crate) fn valid(self) -> bool {
        self.gamma.is_finite() && (0.25..=4.).contains(&self.gamma)
    }
    pub(crate) fn sample(self, pressure: f32) -> f32 {
        let gamma = if self.valid() { self.gamma } else { 1. };
        pressure.clamp(0., 1.).powf(gamma)
    }
    pub(crate) fn ui(&mut self, ui: &mut eframe::egui::Ui) -> bool {
        use eframe::egui::{self, Pos2, Sense, Stroke, Vec2};
        let mut changed = false;
        ui.collapsing(crate::i18n::tr("Pressure response"), |ui| {
            changed |= ui
                .add(
                    egui::Slider::new(&mut self.gamma, 0.25..=4.)
                        .logarithmic(true)
                        .text(crate::i18n::tr("Soft ← response → firm")),
                )
                .changed();
            let (r, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width().min(220.), 70.),
                Sense::hover(),
            );
            ui.painter()
                .rect_filled(r, 3., crate::themes::palette().canvas);
            let points: Vec<_> = (0..=48)
                .map(|i| {
                    let p = i as f32 / 48.;
                    Pos2::new(
                        r.left() + p * r.width(),
                        r.bottom() - self.sample(p) * r.height(),
                    )
                })
                .collect();
            ui.painter().add(egui::Shape::line(
                points,
                Stroke::new(2., crate::themes::accent()),
            ));
            if ui.button(crate::i18n::tr("Linear response")).clicked() {
                *self = Self::default();
                changed = true;
            }
        });
        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressure_curve_is_monotone_bounded_and_mouse_is_unchanged() {
        for gamma in [0.25, 0.5, 1., 2., 4.] {
            let curve = Curve { gamma };
            let mut previous = 0.;
            for i in 0..=100 {
                let p = curve.sample(i as f32 / 100.);
                assert!(p >= previous && p <= 1.);
                previous = p;
            }
            assert_eq!(curve.sample(1.), 1.);
        }
        assert_eq!(Curve::default().sample(0.4), 0.4);
    }
}
