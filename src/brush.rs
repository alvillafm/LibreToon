use eframe::egui::{Pos2, Vec2};
/// Smooth from the original input each time: avoids progressive shrinkage.
pub fn smooth(raw: &[[f32; 2]], amount: f32) -> Vec<[f32; 2]> {
    if raw.len() < 3 || amount <= 0. {
        return raw.to_vec();
    }
    let amount = amount.clamp(0., 1.);
    let mut points: Vec<Pos2> = vec![Pos2::new(raw[0][0], raw[0][1])];
    // Uniform arc-length sampling makes the filter independent of mouse speed.
    let mut carry = 0.;
    let spacing = 2.;
    for w in raw.windows(2) {
        let mut a = Pos2::new(w[0][0], w[0][1]);
        let b = Pos2::new(w[1][0], w[1][1]);
        let mut length = a.distance(b);
        while length + carry >= spacing {
            let step = spacing - carry;
            a += (b - a) * (step / length.max(0.00001));
            points.push(a);
            length = a.distance(b);
            carry = 0.;
        }
        carry += length;
    }
    let end = Pos2::new(raw.last().unwrap()[0], raw.last().unwrap()[1]);
    if points.last() != Some(&end) {
        points.push(end);
    }
    if points.len() < 3 {
        return raw.to_vec();
    }
    let original = points.clone();
    let radius = (1. + amount * 4.).round() as usize;
    for i in 1..points.len() - 1 {
        let mut sum = Vec2::ZERO;
        let mut weight = 0.;
        for j in i.saturating_sub(radius)..=(i + radius).min(points.len() - 1) {
            let w = (radius + 1 - i.abs_diff(j)) as f32;
            sum += original[j].to_vec2() * w;
            weight += w;
        }
        let target = Pos2::ZERO + sum / weight;
        points[i] = original[i].lerp(target, amount);
    }
    let mut out = vec![[points[0].x, points[0].y]];
    for i in 0..points.len() - 1 {
        let a = points[i.saturating_sub(1)];
        let b = points[i];
        let c = points[i + 1];
        let d = points[(i + 2).min(points.len() - 1)];
        let steps = (b.distance(c) / 0.8).ceil().clamp(2., 24.) as usize;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let value = (b.to_vec2() * 2.
                + (c - a) * t
                + (a.to_vec2() * 2. - b.to_vec2() * 5. + c.to_vec2() * 4. - d.to_vec2()) * t2
                + (-a.to_vec2() + b.to_vec2() * 3. - c.to_vec2() * 3. + d.to_vec2()) * t3)
                * 0.5;
            out.push([value.x, value.y]);
        }
    }
    *out.last_mut().unwrap() = *raw.last().unwrap();
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smoothing_reduces_jitter_and_preserves_endpoints() {
        let raw: Vec<_> = (0..80)
            .map(|i| [i as f32 * 2., if i % 2 == 0 { 4. } else { -4. }])
            .collect();
        let s = smooth(&raw, 0.85);
        assert_eq!(s.first(), raw.first());
        assert_eq!(s.last(), raw.last());
        let amplitude = s
            .iter()
            .filter(|p| p[0] > 20. && p[0] < 130.)
            .map(|p| p[1].abs())
            .sum::<f32>()
            / s.iter().filter(|p| p[0] > 20. && p[0] < 130.).count() as f32;
        assert!(amplitude < 1.5);
    }
    #[test]
    fn smoothing_zero_and_taps_are_unchanged() {
        let p = vec![[1., 2.], [9., 12.], [20., 1.]];
        assert_eq!(smooth(&p, 0.), p);
        assert_eq!(smooth(&p[..1], 1.), p[..1]);
    }
}
