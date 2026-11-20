//! Deferred layer compositing. Compatible adjustments remain in the same GPU
//! command chain; unsupported operators materialize the image and retain the
//! reference renderer's semantics. The pending input budget is independent of
//! the playback cache and prevents large scenes retaining every layer in RAM.
use crate::post::Effect;
enum Operation {
    Layer(Vec<u8>, u32, u32),
    Effect(Effect, f32),
}

#[test]
#[ignore = "requires a GPU; deferred padded layer blending and inline effects"]
fn gpu_composition_preserves_layer_order_alpha_and_effect_order() {
    use crate::gpu_lens::{GpuLens, block_on, use_on_worker};
    let instance = eframe::wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = std::sync::Arc::new(GpuLens::new(device, queue));
    let (w, h) = (320, 240);
    let bottom = [20, 50, 70, 120].repeat(w * h);
    let top = [90, 30, 10, 140].repeat((w + 4) * (h + 4));
    for channels in [3, 4] {
        use_on_worker(None);
        let mut expected = Composition::new(w as u32, h as u32, channels);
        expected.composite(bottom.clone(), w as u32, 0);
        expected.composite(top.clone(), w as u32 + 4, 2);
        let expected = expected.finish();
        use_on_worker(Some(gpu.clone()));
        let mut actual = Composition::new(w as u32, h as u32, channels);
        actual.composite(bottom.clone(), w as u32, 0);
        actual.composite(top.clone(), w as u32 + 4, 2);
        let actual = actual.finish();
        use_on_worker(None);
        assert_eq!(expected, actual);
    }
    let effect = Effect::new(crate::post::Kind::Exposure);
    use_on_worker(None);
    let mut expected = Composition::new(w as u32, h as u32, 4);
    expected.composite(bottom.clone(), w as u32, 0);
    crate::post::apply_cpu(
        expected.pixels(),
        w as u32,
        h as u32,
        std::slice::from_ref(&effect),
        1.,
    );
    expected.composite(top.clone(), w as u32 + 4, 2);
    let expected = expected.finish();
    use_on_worker(Some(gpu));
    let mut actual = Composition::new(w as u32, h as u32, 4);
    actual.composite(bottom, w as u32, 0);
    assert!(actual.effects(&[effect], 1.));
    actual.composite(top, w as u32 + 4, 2);
    let actual = actual.finish();
    use_on_worker(None);
    assert_eq!(expected, actual);
}
pub(crate) struct Composition {
    pixels: Vec<u8>,
    size: [u32; 2],
    channels: usize,
    output_channels: usize,
    operations: Vec<Operation>,
    bytes: usize,
    gpu: bool,
}
impl Composition {
    pub fn new(w: u32, h: u32, channels: usize) -> Self {
        let gpu = crate::gpu_lens::available_on_worker();
        let internal = if gpu { 4 } else { channels };
        Self {
            pixels: vec![if channels == 3 { 255 } else { 0 }; w as usize * h as usize * internal],
            size: [w, h],
            channels: internal,
            output_channels: channels,
            operations: Vec::new(),
            bytes: 0,
            gpu,
        }
    }
    pub fn composite(&mut self, pixels: Vec<u8>, source_width: u32, padding: u32) {
        const LIMIT: usize = 32 * 1024 * 1024;
        if self.bytes.saturating_add(pixels.len()) > LIMIT || self.operations.len() >= 32 {
            self.flush();
        }
        if self.gpu && pixels.len() <= LIMIT {
            self.bytes += pixels.len();
            self.operations
                .push(Operation::Layer(pixels, source_width, padding));
        } else {
            Self::blend(
                &mut self.pixels,
                self.size,
                self.channels,
                &pixels,
                source_width,
                padding,
            );
        }
    }
    fn blend(
        target: &mut [u8],
        size: [u32; 2],
        channels: usize,
        source: &[u8],
        source_width: u32,
        padding: u32,
    ) {
        for y in 0..size[1] as usize {
            let source_start =
                ((y + padding as usize) * source_width as usize + padding as usize) * 4;
            let target_start = y * size[0] as usize * channels;
            for (dst, src) in target[target_start..target_start + size[0] as usize * channels]
                .chunks_exact_mut(channels)
                .zip(source[source_start..source_start + size[0] as usize * 4].chunks_exact(4))
            {
                let alpha = f32::from(src[3]) / 255.;
                for c in 0..channels {
                    dst[c] = (f32::from(src[c]) + f32::from(dst[c]) * (1. - alpha))
                        .min(255.)
                        .round() as u8;
                }
            }
        }
    }
    pub fn effects(&mut self, effects: &[Effect], scale: f32) -> bool {
        if !self.gpu
            || effects
                .iter()
                .filter(|e| crate::post::has_pixel_effect(e))
                .any(|e| crate::post::gpu_config(e, self.size[0], self.size[1], scale).is_none())
        {
            return false;
        }
        for effect in effects.iter().filter(|e| crate::post::has_pixel_effect(e)) {
            if self.operations.len() >= 32 {
                self.flush();
            }
            self.operations
                .push(Operation::Effect(effect.clone(), scale));
        }
        true
    }
    pub fn pixels(&mut self) -> &mut Vec<u8> {
        self.flush();
        &mut self.pixels
    }
    fn flush(&mut self) {
        if self.operations.is_empty() {
            return;
        }
        let _span = crate::render_profile::span("GPU composition chain");
        let configs: Vec<_> = self
            .operations
            .iter()
            .map(|op| match op {
                Operation::Layer(pixels, width, pad) => {
                    let mut config = vec![0u32; 16];
                    config[..6].copy_from_slice(&[self.size[0], self.size[1], 12, 0, *width, *pad]);
                    config.extend(
                        pixels
                            .chunks_exact(4)
                            .map(|p| u32::from_le_bytes(p.try_into().unwrap())),
                    );
                    config
                }
                Operation::Effect(effect, scale) => {
                    crate::post::gpu_config(effect, self.size[0], self.size[1], *scale).unwrap()
                }
            })
            .collect();
        if !crate::gpu_lens::post_on_worker(&mut self.pixels, &configs) {
            for op in &self.operations {
                match op {
                    Operation::Layer(pixels, width, pad) => Self::blend(
                        &mut self.pixels,
                        self.size,
                        self.channels,
                        pixels,
                        *width,
                        *pad,
                    ),
                    Operation::Effect(effect, scale) => crate::post::apply_cpu(
                        &mut self.pixels,
                        self.size[0],
                        self.size[1],
                        std::slice::from_ref(effect),
                        *scale,
                    ),
                }
            }
        }
        self.operations.clear();
        self.bytes = 0;
    }
    pub fn finish(mut self) -> Vec<u8> {
        self.flush();
        if self.channels == self.output_channels {
            return self.pixels;
        }
        self.pixels
            .chunks_exact(4)
            .flat_map(|p| p[..3].iter().copied())
            .collect()
    }
}
