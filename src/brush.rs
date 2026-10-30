use eframe::egui::{Pos2, Vec2};
/// Original batch filter retained as a regression and benchmark oracle.
#[cfg(test)]
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

/// Arc-length resampling plus the original finite-window filter and Catmull-Rom
/// interpolation. Only the provisional tail changes when a sample arrives.
pub struct StrokeSmoother {
    sampled: Vec<Pos2>,
    filtered: Vec<Pos2>,
    segment_starts: Vec<usize>,
    last: Pos2,
    carry: f32,
    count: usize,
    amount: f32,
    #[cfg(test)]
    filtered_work: usize,
}
impl StrokeSmoother {
    pub fn new(first: [f32; 2], amount: f32) -> Self {
        let first = Pos2::new(first[0], first[1]);
        Self {
            sampled: vec![first],
            filtered: vec![],
            segment_starts: vec![],
            last: first,
            carry: 0.,
            count: 1,
            amount: amount.clamp(0., 1.),
            #[cfg(test)]
            filtered_work: 0,
        }
    }
    pub fn last(&self) -> Pos2 {
        self.last
    }
    pub fn push(&mut self, point: [f32; 2], out: &mut Vec<[f32; 2]>) {
        let end = Pos2::new(point[0], point[1]);
        self.count += 1;
        if self.amount <= 0. {
            out.push(point);
            self.last = end;
            return;
        }
        let old_stable = self.sampled.len();
        let mut a = self.last;
        let mut length = a.distance(end);
        while length + self.carry >= 2. {
            let step = 2. - self.carry;
            a += (end - a) * (step / length.max(0.00001));
            self.sampled.push(a);
            length = a.distance(end);
            self.carry = 0.;
        }
        self.carry += length;
        self.last = end;
        let stable = self.sampled.len();
        if self.sampled.last() != Some(&end) {
            self.sampled.push(end);
        }
        if self.count < 3 || self.sampled.len() < 3 {
            out.push(point);
            self.sampled.truncate(stable);
            return;
        }
        let radius = (1. + self.amount * 4.).round() as usize;
        let changed = if self.segment_starts.is_empty() {
            0
        } else {
            old_stable.saturating_sub(radius + 1)
        };
        self.filtered.resize(self.sampled.len(), Pos2::ZERO);
        for i in changed..self.sampled.len() {
            #[cfg(test)]
            {
                self.filtered_work += 1;
            }
            self.filtered[i] = if i == 0 || i + 1 == self.sampled.len() {
                self.sampled[i]
            } else {
                let mut sum = Vec2::ZERO;
                let mut weight = 0.;
                for j in i.saturating_sub(radius)..=(i + radius).min(self.sampled.len() - 1) {
                    let w = (radius + 1 - i.abs_diff(j)) as f32;
                    sum += self.sampled[j].to_vec2() * w;
                    weight += w;
                }
                self.sampled[i].lerp(Pos2::ZERO + sum / weight, self.amount)
            };
        }
        let first_segment = changed.saturating_sub(2);
        if first_segment == 0 {
            out.clear();
            out.push([self.filtered[0].x, self.filtered[0].y]);
        } else {
            out.truncate(self.segment_starts[first_segment]);
        }
        self.segment_starts.truncate(first_segment);
        for i in first_segment..self.filtered.len() - 1 {
            self.segment_starts.push(out.len());
            let a = self.filtered[i.saturating_sub(1)];
            let b = self.filtered[i];
            let c = self.filtered[i + 1];
            let d = self.filtered[(i + 2).min(self.filtered.len() - 1)];
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
        *out.last_mut().unwrap() = point;
        self.sampled.truncate(stable);
    }
}

#[cfg(test)]
mod incremental_tests {
    use super::*;
    #[test]
    fn every_prefix_matches_original_filter() {
        let raw: Vec<_> = (0..160)
            .map(|i| {
                let x = i as f32;
                [x * 1.73, (x * 0.63).sin() * 14. + (x * 0.17).cos() * 5.]
            })
            .collect();
        for amount in [0., 0.2, 0.7, 1.] {
            let mut smoother = StrokeSmoother::new(raw[0], amount);
            let mut output = vec![raw[0]];
            for i in 1..raw.len() {
                smoother.push(raw[i], &mut output);
                assert_eq!(
                    output,
                    smooth(&raw[..=i], amount),
                    "prefix={i}, amount={amount}"
                );
            }
        }
    }
    #[test]
    fn duplicates_short_segments_and_long_jumps_match_original() {
        for raw in [
            vec![
                [0., 0.],
                [0., 0.],
                [0.1, 0.],
                [0.2, 0.],
                [500., 300.],
                [500., 300.],
            ],
            vec![[0., 0.], [2., 0.], [4., 0.], [6., 0.], [8., 0.]],
        ] {
            let mut smoother = StrokeSmoother::new(raw[0], 0.7);
            let mut output = vec![raw[0]];
            for i in 1..raw.len() {
                smoother.push(raw[i], &mut output);
                assert_eq!(output, smooth(&raw[..=i], 0.7));
            }
        }
    }
    #[test]
    fn long_stroke_filter_work_grows_linearly() {
        let mut smoother = StrokeSmoother::new([0., 0.], 0.7);
        let mut output = vec![[0., 0.]];
        for i in 1..10_000 {
            smoother.push([i as f32 * 2., 0.], &mut output);
        }
        assert!(
            smoother.filtered_work < 100_000,
            "work={}",
            smoother.filtered_work
        );
        assert_eq!(output.last(), Some(&[19_998., 0.]));
    }
    #[test]
    #[ignore = "manual release benchmark"]
    fn benchmark_long_stroke() {
        let raw: Vec<_> = (0..5000)
            .map(|i| [i as f32 * 0.8, (i as f32 * 0.1).sin() * 10.])
            .collect();
        let start = std::time::Instant::now();
        let mut smoother = StrokeSmoother::new(raw[0], 0.7);
        let mut output = vec![raw[0]];
        for point in &raw[1..] {
            smoother.push(*point, &mut output);
        }
        let incremental = start.elapsed();
        let start = std::time::Instant::now();
        for i in 2..=raw.len() {
            std::hint::black_box(smooth(&raw[..i], 0.7));
        }
        let baseline = start.elapsed();
        assert_eq!(output, smooth(&raw, 0.7));
        eprintln!(
            "5000 samples: original={baseline:?}, incremental={incremental:?}, ratio={:.1}x",
            baseline.as_secs_f64() / incremental.as_secs_f64()
        );
    }
}
