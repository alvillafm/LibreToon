//! Editable mouth poses and local audio-to-pose suggestions.
use crate::{
    Editor,
    design::{self, Icon},
    i18n::tr,
    shared::SharedVec,
};
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Shape {
    Rest,
    Closed,
    Open,
    Wide,
    Round,
    Fricative,
}

impl Shape {
    const ALL: [(Self, &'static str, &'static [&'static str]); 6] = [
        (Self::Rest, "Rest", &["rest", "reposo", "silence"]),
        (
            Self::Closed,
            "Closed",
            &["m", "p", "b", "closed", "cerrada"],
        ),
        (Self::Open, "Open", &["a", "open", "abierta"]),
        (Self::Wide, "Wide", &["e", "i", "ñ", "wide", "estirada"]),
        (Self::Round, "Round", &["o", "u", "eau", "round", "redonda"]),
        (Self::Fricative, "Fricative", &["s", "f", "v"]),
    ];
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Pose {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shape: Option<Shape>,
}
impl From<&str> for Pose {
    fn from(name: &str) -> Self {
        Self {
            name: name.into(),
            shape: None,
        }
    }
}
impl From<String> for Pose {
    fn from(name: String) -> Self {
        Self { name, shape: None }
    }
}
const DEFAULT_POSES: [&str; 9] = ["Rest", "M/P/B", "A", "E", "I", "O", "U", "F/V", "S"];

pub(crate) fn default_poses(frame_count: usize) -> SharedVec<Pose> {
    (0..frame_count.max(DEFAULT_POSES.len()))
        .map(|frame| {
            DEFAULT_POSES
                .get(frame)
                .map_or_else(|| format!("Pose {}", frame + 1).into(), |&name| name.into())
        })
        .collect()
}

impl Pose {
    pub fn valid(&self) -> bool {
        !self.name.is_empty() && self.name.len() <= 48
    }

    fn shape(&self) -> Option<Shape> {
        self.shape.or_else(|| {
            let name = self.name.to_lowercase();
            Shape::ALL
                .iter()
                .find(|(_, _, aliases)| {
                    name.split(|c: char| !c.is_alphabetic())
                        .any(|word| aliases.contains(&word))
                })
                .map(|(shape, _, _)| *shape)
        })
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub frame: usize,
    pub pose: usize,
    pub manual: bool,
}

pub fn pose_at(keys: &[Key], frame: usize) -> Option<usize> {
    keys.get(
        keys.partition_point(|key| key.frame <= frame)
            .checked_sub(1)?,
    )
    .map(|key| key.pose)
}

#[derive(Clone)]
pub(crate) struct Audio {
    pub name: String,
    pub data: SharedVec<u8>,
    pub start: usize,
    pub offset: usize,
    pub length: usize,
}

type Analysis = Result<Vec<Key>, &'static str>;

struct Job {
    layer: usize,
    mark: usize,
    frame: usize,
    audio_start: usize,
    audio_end: usize,
    owner: SharedVec<crate::document::Layer>,
    media: SharedVec<crate::media::Track>,
    symbols: SharedVec<crate::features::Symbol>,
    receiver: std::sync::mpsc::Receiver<Analysis>,
}

#[derive(Default)]
pub(crate) struct State {
    pose_name: String,
    audio: usize,
    job: Option<Job>,
}

impl Editor {
    fn lip_symbol_context(&self) -> Option<usize> {
        let id = self.feature.symbol_stack.last()?.3;
        self.doc
            .extras
            .symbols
            .iter()
            .position(|symbol| symbol.id == id && !symbol.mouth_poses.is_empty())
    }

    pub(crate) fn lip_frame_label(&self, frame: usize) -> Option<String> {
        let symbol = &self.doc.extras.symbols[self.lip_symbol_context()?];
        symbol
            .mouth_poses
            .get(frame)
            .map(|pose| tr(&pose.name).to_owned())
    }

    pub(crate) fn enable_lip_symbol_on_selection(&mut self) -> bool {
        let [(layer, mark)] = self.canvas.selected.as_slice() else {
            return false;
        };
        let Some(instance) = self.doc.layers[*layer].frames[self.frame]
            .get(*mark)
            .and_then(|mark| mark.style.instance.as_ref())
        else {
            return false;
        };
        let id = instance.symbol;
        let Some(index) = self
            .doc
            .extras
            .symbols
            .iter()
            .position(|symbol| symbol.id == id)
        else {
            return false;
        };
        let count = self.doc.extras.symbols[index].layers[0].frames.len();
        if count > 128 {
            self.status = tr("Lip Sync symbols support up to 128 poses.").into();
            return true;
        }
        if self.doc.extras.symbols[index].mouth_poses.is_empty() {
            self.checkpoint();
            let symbol = &mut self.doc.extras.symbols[index];
            symbol.mouth_poses = default_poses(count);
            for layer in &mut symbol.layers {
                layer.init_keys();
                layer
                    .frames
                    .resize_with(symbol.mouth_poses.len(), Default::default);
                layer.options.keys.resize(symbol.mouth_poses.len(), true);
            }
            self.recover();
        }
        true
    }

    pub(crate) fn lip_pose_properties(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.lip_symbol_context() else {
            return;
        };
        design::category(ui, "Lip Sync poses");
        if let Some(label) = self.lip_frame_label(self.frame) {
            ui.label(label);
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.lip.pose_name)
                    .desired_width(115.)
                    .hint_text(tr("Pose or sound")),
            );
            ui.menu_button(tr("Common poses"), |ui| {
                for pose in DEFAULT_POSES.into_iter().chain(["Ñ", "EAU"]) {
                    if ui.button(tr(pose)).clicked() {
                        self.lip.pose_name = pose.into();
                        ui.close();
                    }
                }
            });
        });
        let name: String = self
            .lip
            .pose_name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(12)
            .collect();
        let count = self.doc.layers[0].frames.len();
        let mut add = false;
        let mut rename = false;
        let original = self.doc.extras.symbols[index].mouth_poses.get(self.frame);
        let mut shape = original.and_then(|pose| pose.shape);
        let inferred = original.and_then(Pose::shape);
        let selected = shape
            .and_then(|value| {
                Shape::ALL
                    .iter()
                    .find(|(s, _, _)| *s == value)
                    .map(|(_, label, _)| *label)
            })
            .unwrap_or("From pose name");
        if original.is_some() {
            ui.label(tr("Automatic mouth shape"));
            egui::ComboBox::from_id_salt("lip-pose-shape")
                .selected_text(tr(selected))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut shape, None, tr("From pose name"));
                    for (value, label, _) in Shape::ALL {
                        ui.selectable_value(&mut shape, Some(value), tr(label));
                    }
                });
            if shape.is_none()
                && let Some((_, label, _)) =
                    Shape::ALL.iter().find(|(s, _, _)| Some(*s) == inferred)
            {
                ui.small(tr(label));
            }
        }
        let update_shape = shape != original.and_then(|pose| pose.shape);
        ui.horizontal_wrapped(|ui| {
            add = design::button(
                ui,
                Icon::Plus,
                "Add pose",
                "",
                false,
                !name.is_empty() && count < 128,
            )
            .clicked();
            rename = design::button(
                ui,
                Icon::Rename,
                "Name this pose",
                "",
                false,
                !name.is_empty() && self.frame < 128,
            )
            .clicked();
        });
        if add || rename || update_shape {
            self.finish_pending_pen();
            self.checkpoint();
            if add {
                for layer in &mut self.doc.layers {
                    layer.init_keys();
                    layer.frames.push(Default::default());
                    layer.options.keys.push(true);
                }
                self.frame = count;
            }
            let poses = &mut self.doc.extras.symbols[index].mouth_poses;
            let length = poses.len().max(self.frame + 1);
            poses.resize_with(length, || "Rest".into());
            if add || rename {
                poses[self.frame] = Pose {
                    name,
                    shape: if add { None } else { shape },
                };
                self.lip.pose_name.clear();
            } else {
                poses[self.frame].shape = shape;
            }
            self.canvas.clear_selection();
            self.recover();
        }
        ui.separator();
    }

    pub(crate) fn lip_instance_properties(
        &mut self,
        ui: &mut egui::Ui,
        l: usize,
        m: usize,
        symbol: u64,
    ) -> bool {
        let Some(poses) = self
            .doc
            .extras
            .symbols
            .iter()
            .find(|s| s.id == symbol)
            .filter(|s| !s.mouth_poses.is_empty())
            .map(|s| s.mouth_poses.clone())
        else {
            return false;
        };
        let Some(mark) = self.doc.layers[l].frames[self.frame].get(m) else {
            return false;
        };
        let current = pose_at(&mark.style.mouth_keys, self.frame).unwrap_or_else(|| {
            mark.style
                .instance
                .as_ref()
                .map_or(0, |instance| instance.first)
        });
        let has_key = mark
            .style
            .mouth_keys
            .binary_search_by_key(&self.frame, |k| k.frame)
            .is_ok();
        let unlocked = !self.doc.layers[l].options.locked;
        design::category(ui, "LIPSYNC");
        if design::button(ui, Icon::Node, "Editar símbolo", "", false, true).clicked() {
            self.enter_symbol(symbol);
            return true;
        }
        let previous = mark
            .style
            .mouth_keys
            .iter()
            .rev()
            .find(|key| key.frame < self.frame)
            .map(|key| key.frame);
        let next = mark
            .style
            .mouth_keys
            .iter()
            .find(|key| key.frame > self.frame && key.frame < self.doc.layers[0].frames.len())
            .map(|key| key.frame);
        let mut jump = None;
        ui.horizontal(|ui| {
            for (target, icon, tip) in [
                (previous, Icon::Previous, "Previous mouth key"),
                (next, Icon::Next, "Next mouth key"),
            ] {
                if design::button(ui, icon, "", tip, false, target.is_some()).clicked()
                    && let Some(target) = target
                {
                    jump = Some(target);
                }
            }
        });
        if let Some(target) = jump {
            self.frame = target;
            self.playing = false;
            self.scrub_media_audio(ui.ctx());
            return true;
        }
        ui.add_enabled_ui(unlocked, |ui| {
            ui.small(tr("Edit the symbol to add more mouth poses."));
            ui.horizontal_wrapped(|ui| {
                for (pose, drawing) in poses.iter().enumerate() {
                    if design::button(ui, Icon::Mouth, &drawing.name, "", current == pose, true)
                        .clicked()
                    {
                        let frame = self.frame;
                        self.change_mouth_keys(l, m, frame, |keys| {
                            keys.retain(|key| key.frame != frame);
                            keys.push(Key {
                                frame,
                                pose,
                                manual: true,
                            });
                        });
                    }
                }
                if design::button(ui, Icon::Minus, "", "Delete mouth key", false, has_key).clicked()
                {
                    let frame = self.frame;
                    self.change_mouth_keys(l, m, frame, |keys| {
                        keys.retain(|key| key.frame != frame)
                    });
                }
            });
            let audio = self.doc.extras.media.dialogue_clips();
            design::category(ui, "AUTOMATED SYNC");
            ui.small(tr(
                "Right-click a WAV clip to sync it with the selected Lip Sync symbol.",
            ));
            if audio.is_empty() {
                ui.small(tr("Add an audio clip to this scene."));
            } else {
                self.lip.audio = self.lip.audio.min(audio.len() - 1);
                egui::ComboBox::from_id_salt("lip-audio")
                    .selected_text(&audio[self.lip.audio].name)
                    .show_ui(ui, |ui| {
                        for (i, clip) in audio.iter().enumerate() {
                            ui.selectable_value(&mut self.lip.audio, i, &clip.name);
                        }
                    });
                if design::button(
                    ui,
                    Icon::Audio,
                    "Generate mouth keys",
                    "",
                    false,
                    self.lip.job.is_none(),
                )
                .clicked()
                {
                    self.start_lip_analysis(
                        ui.ctx(),
                        audio[self.lip.audio].clone(),
                        l,
                        m,
                        poses.clone(),
                    );
                }
                if self.lip.job.is_some() {
                    ui.spinner();
                }
            }
        });
        true
    }

    fn change_mouth_keys(
        &mut self,
        layer: usize,
        mark: usize,
        frame: usize,
        edit: impl FnOnce(&mut Vec<Key>),
    ) {
        let old = &self.doc.layers[layer].frames[frame][mark];
        let Some(symbol) = old.style.instance.as_ref().map(|instance| instance.symbol) else {
            return;
        };
        let track = old.style.track;
        let mouth_track = if track == 0 {
            crate::features::id()
        } else {
            track
        };
        let exposure = self.doc.layers[layer].exposure(frame);
        let mut keys = old.style.mouth_keys.to_vec();
        edit(&mut keys);
        keys.sort_by_key(|key| key.frame);
        let keys: SharedVec<_> = keys.into();
        self.checkpoint();
        let layer = &mut self.doc.layers[layer];
        let mut edited = std::collections::HashMap::new();
        for (f, cel) in layer.frames.iter_mut().enumerate() {
            if track == 0 && !exposure.contains(&f) {
                continue;
            }
            let next = edited.entry(cel.identity()).or_insert_with(|| {
                let mut updated = cel.clone();
                if track == 0 {
                    if let Some(mouth) = updated.get_mut(mark) {
                        mouth.style.track = mouth_track;
                        mouth.style.mouth_keys = keys.clone();
                    }
                } else if cel.iter().any(|m| {
                    m.style.track == track
                        && m.style
                            .instance
                            .as_ref()
                            .is_some_and(|i| i.symbol == symbol)
                }) {
                    for mouth in &mut updated {
                        if mouth.style.track == track
                            && mouth
                                .style
                                .instance
                                .as_ref()
                                .is_some_and(|i| i.symbol == symbol)
                        {
                            mouth.style.mouth_keys = keys.clone();
                        }
                    }
                }
                updated
            });
            *cel = next.clone();
        }
        self.recover();
    }

    pub(crate) fn start_lip_analysis(
        &mut self,
        ctx: &egui::Context,
        mut audio: Audio,
        layer: usize,
        mark: usize,
        poses: SharedVec<Pose>,
    ) {
        if self.lip.job.is_some() {
            return;
        }
        if let Some((_, a, b)) = self.feature.frame_selection {
            let start = audio.start.max(a.min(b));
            let end = (audio.start + audio.length).min(a.max(b) + 1);
            if end > start {
                audio.offset += start - audio.start;
                audio.start = start;
                audio.length = end - start;
            }
        }
        let (tx, receiver) = std::sync::mpsc::channel();
        let fps = self.doc.fps;
        let frame_count = self.doc.layers[0].frames.len();
        let ctx = ctx.clone();
        self.lip.job = Some(Job {
            layer,
            mark,
            frame: self.frame,
            audio_start: audio.start,
            audio_end: (audio.start + audio.length).min(frame_count),
            owner: self.doc.layers.clone(),
            media: self.doc.extras.media.tracks.clone(),
            symbols: self.doc.extras.symbols.clone(),
            receiver,
        });
        std::thread::spawn(move || {
            let result = analyze(&audio, fps, frame_count, &poses);
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    pub(crate) fn poll_lip_analysis(&mut self) {
        let Some(job) = self.lip.job.as_ref() else {
            return;
        };
        let result = match job.receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(_) => Err("Lip Sync analysis failed."),
        };
        let job = self.lip.job.take().unwrap();
        if self.doc.layers.identity() != job.owner.identity()
            || self.doc.extras.media.tracks.identity() != job.media.identity()
            || self.doc.extras.symbols.identity() != job.symbols.identity()
        {
            self.status = tr("The target changed. Run Lip Sync again.").into();
            return;
        }
        match result {
            Ok(suggestions) => {
                self.change_mouth_keys(job.layer, job.mark, job.frame, |keys| {
                    let resume = pose_at(keys, job.audio_end).unwrap_or(0);
                    keys.retain(|key| {
                        key.manual || !(job.audio_start..job.audio_end).contains(&key.frame)
                    });
                    let retained: std::collections::HashSet<_> =
                        keys.iter().map(|key| key.frame).collect();
                    keys.extend(
                        suggestions
                            .into_iter()
                            .filter(|key| !retained.contains(&key.frame)),
                    );
                    if job.audio_end < 10_000 && !keys.iter().any(|key| key.frame == job.audio_end)
                    {
                        keys.push(Key {
                            frame: job.audio_end,
                            pose: resume,
                            manual: false,
                        });
                    }
                });
                self.status = tr("Lip Sync keys generated.").into();
            }
            Err(error) => self.status = tr(error).into(),
        }
    }
}

pub(crate) struct Wave<'a> {
    bytes: &'a [u8],
    pub(crate) rate: usize,
    pub(crate) channels: usize,
    width: usize,
    float: bool,
}

impl<'a> Wave<'a> {
    pub(crate) fn read(bytes: &'a [u8]) -> Option<Self> {
        if bytes.get(..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
            return None;
        }
        let mut data = None;
        let mut format = None;
        let mut cursor = 12;
        while let Some(header) = bytes.get(cursor..cursor + 8) {
            let len = u32::from_le_bytes(header[4..8].try_into().ok()?) as usize;
            let end = cursor.checked_add(8)?.checked_add(len)?;
            let chunk = bytes.get(cursor + 8..end)?;
            match &header[..4] {
                b"fmt " => format = Some(chunk),
                b"data" => data = Some(chunk),
                _ => {}
            }
            cursor = end + len % 2;
        }
        let fmt = format?;
        let u16_at = |i| Some(u16::from_le_bytes(fmt.get(i..i + 2)?.try_into().ok()?));
        let mut kind = u16_at(0)?;
        if kind == 0xfffe
            && fmt.get(26..40)? == [0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]
        {
            kind = u16_at(24)?;
        }
        let channels = usize::from(u16_at(2)?);
        let rate = u32::from_le_bytes(fmt.get(4..8)?.try_into().ok()?) as usize;
        let bits = u16_at(14)?;
        let width = usize::from(bits / 8);
        if !(1..=32).contains(&channels)
            || !(4000..=384000).contains(&rate)
            || !(kind == 1 && matches!(bits, 8 | 16 | 24 | 32) || kind == 3 && bits == 32)
            || usize::from(u16_at(12)?) != width * channels
        {
            return None;
        }
        Some(Self {
            bytes: data?,
            rate,
            channels,
            width,
            float: kind == 3,
        })
    }

    pub(crate) fn frames(&self) -> usize {
        self.bytes.len() / (self.width * self.channels)
    }

    pub(crate) fn channel(&self, index: usize, channel: usize) -> f32 {
        let start = (index * self.channels + channel.min(self.channels - 1)) * self.width;
        let Some(raw) = self.bytes.get(start..start + self.width) else {
            return 0.;
        };
        let sample = match raw {
            [a] => (f32::from(*a) - 128.) / 128.,
            [a, b] => i16::from_le_bytes([*a, *b]) as f32 / 32768.,
            [a, b, c] => i32::from_le_bytes([0, *a, *b, *c]) as f32 / 2147483648.,
            [a, b, c, d] if self.float => f32::from_le_bytes([*a, *b, *c, *d]),
            [a, b, c, d] => i32::from_le_bytes([*a, *b, *c, *d]) as f32 / 2147483648.,
            _ => 0.,
        };
        if sample.is_finite() {
            sample.clamp(-1., 1.)
        } else {
            0.
        }
    }

    pub(crate) fn sample(&self, index: usize) -> f32 {
        (0..self.channels)
            .map(|channel| self.channel(index, channel))
            .sum::<f32>()
            / self.channels as f32
    }
}

fn analyze(audio: &Audio, fps: u32, count: usize, poses: &[Pose]) -> Analysis {
    let wave =
        Wave::read(&audio.data).ok_or("Automatic Lip Sync requires a PCM or float WAV clip.")?;
    let find = |shape| {
        poses
            .iter()
            .position(|pose| pose.shape == Some(shape))
            .or_else(|| poses.iter().position(|pose| pose.shape() == Some(shape)))
    };
    let rest = find(Shape::Rest).unwrap_or(0);
    let open = find(Shape::Open).ok_or("Add an A or Open pose before generating Lip Sync.")?;
    let closed = find(Shape::Closed).unwrap_or(rest);
    let round = find(Shape::Round).unwrap_or(open);
    let wide = find(Shape::Wide).unwrap_or(open);
    let fricative = find(Shape::Fricative).unwrap_or(wide);
    let end = (audio.start + audio.length).min(count);
    if end <= audio.start {
        return Err("The audio clip is outside the scene.");
    }
    let step = (wave.rate / 8000).max(1);
    let sample_rate = wave.rate as f32 / step as f32;
    let bands = [450., 800., 1250., 2100., 3100.]
        .map(|hz| 2. * (std::f32::consts::TAU * hz / sample_rate).cos());
    let mut windows = Vec::with_capacity(end - audio.start);
    for f in audio.start..end {
        let source = audio.offset + f - audio.start;
        let start = source * wave.rate / fps as usize;
        let stop = (source + 1) * wave.rate / fps as usize;
        let mut power = 0.;
        let mut states = [(0f32, 0f32); 5];
        let mut previous = 0.;
        let mut difference = 0.;
        for i in (start..stop).step_by(step) {
            let value = wave.sample(i);
            power += value * value;
            difference += (value - previous).powi(2);
            previous = value;
            for ((last, older), coeff) in states.iter_mut().zip(bands) {
                (*last, *older) = (value + coeff * *last - *older, *last);
            }
        }
        let energy = (power / ((stop - start) / step).max(1) as f32).sqrt();
        let spectrum: [f32; 5] = std::array::from_fn(|i| {
            let (a, b) = states[i];
            (a * a + b * b - bands[i] * a * b).max(0.)
        });
        let pose = if difference > power * 1.2 {
            fricative
        } else if spectrum[1] + spectrum[2] > 1.5 * (spectrum[3] + spectrum[4]).max(1e-9) {
            round
        } else if spectrum[3] > (spectrum[1] + spectrum[2]) * 1.4 {
            wide
        } else {
            open
        };
        windows.push((energy, pose));
    }
    let peak = windows.iter().map(|(v, _)| *v).fold(0f32, f32::max);
    let gate = (peak * 0.075).max(0.004);
    let mut result: Vec<Key> = Vec::new();
    for (i, &(energy, voice)) in windows.iter().enumerate() {
        let next_voice = windows
            .get(i + 1)
            .is_some_and(|(energy, _)| *energy >= gate);
        let prev_voice = i > 0 && windows[i - 1].0 >= gate;
        let pose = if energy < gate {
            if next_voice && (prev_voice || energy > gate * 0.35) {
                closed
            } else {
                rest
            }
        } else {
            voice
        };
        if result.last().is_none_or(|key| key.pose != pose) {
            result.push(Key {
                frame: audio.start + i,
                pose,
                manual: false,
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimmed_wav_analysis_emits_keys_at_scene_frames() {
        let mut wav = crate::media::test_wav();
        for i in 2000..4000 {
            let sample = ((i as f32 * 0.35).sin() * 12000.) as i16;
            wav[44 + i * 2..46 + i * 2].copy_from_slice(&sample.to_le_bytes());
        }
        let audio = Audio {
            name: "dialogue".into(),
            data: wav.into(),
            start: 10,
            offset: 4,
            length: 10,
        };
        let keys = analyze(&audio, 24, 24, &["Rest".into(), "A".into()]).unwrap();
        assert_eq!(pose_at(&keys, 10), Some(0));
        assert_eq!(pose_at(&keys, 13), Some(1));
        assert_eq!(pose_at(&keys, 19), Some(0));
        assert!(
            keys.iter()
                .all(|k| (10..20).contains(&k.frame) && !k.manual)
        );
    }

    #[test]
    fn shared_symbol_mouths_keep_independent_keys_and_roundtrip() {
        let mut editor = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        editor.doc.layers[0].frames[0].push(crate::Mark {
            points: vec![[0., 0.], [10., 10.]].into(),
            rectangle: true,
            ..Default::default()
        });
        editor.canvas.selected = vec![(0, 0)];
        editor.convert_symbol_kind(true);
        let symbol = &mut editor.doc.extras.symbols[0];
        let layer = &mut symbol.layers[0];
        assert_eq!(symbol.mouth_poses.len(), DEFAULT_POSES.len());
        assert_eq!(symbol.mouth_poses[2].name, "A");
        assert!(layer.frames.iter().skip(1).all(|frame| frame.is_empty()));
        assert!((0..layer.frames.len()).all(|frame| layer.key(frame)));
        let mut open = layer.frames[0].clone();
        open[0].points[1][1] = 20.;
        layer.frames[2] = open;
        let second = editor.doc.layers[0].frames[0][0].clone();
        editor.doc.layers[0].frames[0].push(second);
        editor.change_mouth_keys(0, 0, 0, |keys| {
            keys.push(Key {
                frame: 0,
                pose: 2,
                manual: true,
            })
        });
        let data = crate::native_project::encode(&editor.doc).unwrap();
        let decoded = crate::native_project::decode_if_native(&data)
            .unwrap()
            .unwrap();
        let resolved = crate::features::resolve(&decoded, 0, false);
        assert_eq!(crate::vector::control_bounds(&resolved[0].2).height(), 20.);
        assert_eq!(crate::vector::control_bounds(&resolved[1].2).height(), 10.);
    }
}
