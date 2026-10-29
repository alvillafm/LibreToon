use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Mark {
    pub points: Vec<[f32; 2]>,
    pub color: [u8; 4],
    pub width: f32,
    pub rectangle: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub frames: Vec<Vec<Mark>>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub fps: u32,
    pub layers: Vec<Layer>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            fps: 12,
            layers: vec![Layer {
                name: "Capa 1".into(),
                visible: true,
                frames: vec![vec![]; 24],
            }],
        }
    }
}
impl Document {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !(1..=60).contains(&self.fps)
            || self.layers.is_empty()
            || self.layers.len() > 100
        {
            return Err("Formato de proyecto no compatible".into());
        }
        let count = self.layers[0].frames.len();
        if count == 0 || count > 10000 {
            return Err("Número de fotogramas no válido".into());
        }
        for layer in &self.layers {
            if layer.frames.len() != count {
                return Err("Capas con duraciones diferentes".into());
            }
            for mark in layer.frames.iter().flatten() {
                if !mark.width.is_finite()
                    || mark.width <= 0.0
                    || mark.width > 100.0
                    || mark
                        .points
                        .iter()
                        .flatten()
                        .any(|v| !v.is_finite() || v.abs() > 1e7)
                {
                    return Err("Trazo no válido".into());
                }
            }
        }
        Ok(())
    }
    pub fn svg(&self, frame: usize) -> String {
        let mut out = String::from(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\" height=\"540\" viewBox=\"0 0 960 540\"><rect width=\"960\" height=\"540\" fill=\"white\"/>",
        );
        for layer in self.layers.iter().filter(|l| l.visible) {
            for m in &layer.frames[frame] {
                let c = format!("rgb({},{},{})", m.color[0], m.color[1], m.color[2]);
                let style = format!(
                    "fill=\"none\" stroke=\"{c}\" stroke-opacity=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
                    m.color[3] as f32 / 255.0,
                    m.width
                );
                if m.rectangle && m.points.len() >= 2 {
                    let a = m.points[0];
                    let b = *m.points.last().unwrap();
                    out += &format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {style}/>",
                        a[0].min(b[0]),
                        a[1].min(b[1]),
                        (a[0] - b[0]).abs(),
                        (a[1] - b[1]).abs()
                    );
                } else if m.points.len() == 1 {
                    out += &format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{c}\" fill-opacity=\"{}\"/>",
                        m.points[0][0],
                        m.points[0][1],
                        m.width / 2.0,
                        m.color[3] as f32 / 255.0
                    );
                } else {
                    let points = m
                        .points
                        .iter()
                        .map(|p| format!("{},{}", p[0], p[1]))
                        .collect::<Vec<_>>()
                        .join(" ");
                    out += &format!("<polyline points=\"{points}\" {style}/>");
                }
            }
        }
        out + "</svg>"
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_round_trip_and_validation() {
        let mut d = Document::default();
        d.layers[0].frames[0].push(Mark {
            points: vec![[20., 30.]],
            color: [0, 0, 0, 255],
            width: 4.,
            rectangle: false,
        });
        let restored: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert!(restored.validate().is_ok());
        assert!(restored.svg(0).contains("<circle"));
        d.layers[0].frames.clear();
        assert!(d.validate().is_err());
    }
    #[test]
    fn reject_mismatched_timeline() {
        let mut d = Document::default();
        let mut l = d.layers[0].clone();
        l.frames.pop();
        d.layers.push(l);
        assert!(d.validate().is_err());
    }
}
