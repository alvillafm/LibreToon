//! Opt-in frame instrumentation. No clocks or allocations on the normal path.
use std::{cell::RefCell, collections::BTreeMap, time::Instant};
thread_local! {static FRAME:RefCell<Option<(Instant,BTreeMap<&'static str,f64>)>>=const{RefCell::new(None)};}
pub(crate) struct Frame(bool);
pub(crate) fn frame() -> Frame {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let enabled = *ENABLED.get_or_init(|| std::env::var_os("LIBRETOON_RENDER_PROFILE").is_some());
    if enabled {
        FRAME.with(|frame| *frame.borrow_mut() = Some((Instant::now(), BTreeMap::new())));
    }
    Frame(enabled)
}
impl Drop for Frame {
    fn drop(&mut self) {
        if self.0 {
            FRAME.with(|frame| {
                if let Some((start, times)) = frame.borrow_mut().take() {
                    eprintln!(
                        "render profile: total {:.2} ms; {times:?}",
                        start.elapsed().as_secs_f64() * 1000.
                    );
                }
            });
        }
    }
}
pub(crate) struct Span(&'static str, Option<Instant>);
pub(crate) fn span(name: &'static str) -> Span {
    Span(
        name,
        FRAME
            .with(|frame| frame.borrow().is_some())
            .then(Instant::now),
    )
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.1 {
            FRAME.with(|frame| {
                if let Some((_, times)) = frame.borrow_mut().as_mut() {
                    *times.entry(self.0).or_default() += start.elapsed().as_secs_f64() * 1000.;
                }
            });
        }
    }
}

#[cfg(test)]
mod benchmarks {
    fn percentiles(values: &mut [f64]) -> [f64; 3] {
        values.sort_by(f64::total_cmp);
        [0.5, 0.95, 0.99].map(|p| values[((values.len() - 1) as f64 * p).ceil() as usize])
    }
    #[test]
    #[ignore = "manual release storage benchmark; writes only isolated target/qa files"]
    fn benchmark_native_storage() {
        use std::io::Write;
        let mut projects: Vec<_> = crate::examples::PROJECTS
            .iter()
            .map(|(name, key)| {
                (
                    name.to_string(),
                    crate::examples::project(key).unwrap().unwrap(),
                )
            })
            .collect();
        if let Some(path) = std::env::var_os("LIBRETOON_BENCH_PROJECT") {
            projects.push((
                std::path::Path::new(&path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                crate::documents::read_project(std::path::Path::new(&path)).unwrap(),
            ));
        }
        std::fs::create_dir_all("target/qa").unwrap();
        let path = std::path::PathBuf::from(format!(
            "target/qa/storage-benchmark-{}.ltoon",
            std::process::id()
        ));
        let mut report = Vec::new();
        for (name, doc) in projects {
            let mut save = Vec::new();
            let mut open = Vec::new();
            let mut sizes = Vec::new();
            let expected = (
                doc.width,
                doc.height,
                doc.fps,
                crate::animation::movie_count(&doc),
                doc.layers.len(),
                doc.extras.symbols.len(),
                doc.extras.media.assets.len(),
            );
            for _ in 0..8 {
                let start = std::time::Instant::now();
                let mut file = std::fs::File::create(&path).unwrap();
                {
                    let mut writer = std::io::BufWriter::with_capacity(64 * 1024, &mut file);
                    crate::native_project::write(&doc, &mut writer).unwrap();
                    writer.flush().unwrap();
                }
                file.sync_all().unwrap();
                save.push(start.elapsed().as_secs_f64() * 1000.);
                sizes.push(file.metadata().unwrap().len());
                drop(file);
                let start = std::time::Instant::now();
                let restored = crate::documents::read_project(&path).unwrap();
                open.push(start.elapsed().as_secs_f64() * 1000.);
                restored.validate().unwrap();
                assert_eq!(
                    expected,
                    (
                        restored.width,
                        restored.height,
                        restored.fps,
                        crate::animation::movie_count(&restored),
                        restored.layers.len(),
                        restored.extras.symbols.len(),
                        restored.extras.media.assets.len()
                    )
                );
            }
            let row = serde_json::json!({"scene":name,"samples":8,"profile":if cfg!(debug_assertions){"debug"}else{"release"},"write_flush_sync_ms_p50_p95_p99":percentiles(&mut save),"open_decode_ms_p50_p95_p99":percentiles(&mut open),"file_bytes":sizes,"scope":"native codec and file I/O; warm OS cache; excludes save conflict/version publication and UI latency"});
            eprintln!("{row}");
            report.push(row);
        }
        std::fs::remove_file(path).unwrap();
        std::fs::write(
            "target/qa/storage-benchmark.json",
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    #[test]
    #[ignore = "manual release end-to-end benchmark; optionally set LIBRETOON_BENCH_PROJECT"]
    fn benchmark_production_scenes() {
        let mut projects: Vec<_> = crate::examples::PROJECTS
            .iter()
            .map(|(name, key)| {
                (
                    name.to_string(),
                    crate::examples::project(key).unwrap().unwrap(),
                )
            })
            .collect();
        if let Some(path) = std::env::var_os("LIBRETOON_BENCH_PROJECT") {
            projects.push((
                path.to_string_lossy().into_owned(),
                crate::documents::read_project(std::path::Path::new(&path)).unwrap(),
            ));
        }
        let gpu = if std::env::var_os("LIBRETOON_BENCH_GPU").is_some() {
            use crate::gpu_lens::{GpuLens, block_on};
            let adapter =
                block_on(eframe::wgpu::Instance::default().request_adapter(&Default::default()))
                    .expect("GPU adapter");
            let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
            Some(std::sync::Arc::new(GpuLens::new(device, queue)))
        } else {
            None
        };
        crate::gpu_lens::use_on_worker(gpu.clone());
        let mut report = Vec::new();
        for (name, doc) in projects {
            let ctx = eframe::egui::Context::default();
            let mut cache = crate::paint::RenderCache::default();
            let count = crate::animation::movie_count(&doc);
            let samples = count.min(16).max(1);
            let mut cold = Vec::new();
            let mut warm = Vec::new();
            let mut checksums = Vec::new();
            for i in 0..samples {
                let frame = i * count.saturating_sub(1) / samples.saturating_sub(1).max(1);
                let start = std::time::Instant::now();
                let pixels =
                    crate::exporter::raster(&doc, frame, doc.width, doc.height, &ctx, &mut cache);
                cold.push(start.elapsed().as_secs_f64() * 1000.);
                let start = std::time::Instant::now();
                let repeated =
                    crate::exporter::raster(&doc, frame, doc.width, doc.height, &ctx, &mut cache);
                warm.push(start.elapsed().as_secs_f64() * 1000.);
                assert_eq!(pixels, repeated, "cache changed rendered pixels");
                checksums.push(crc32fast::hash(&pixels));
            }
            let row = serde_json::json!({"scene":name, "resolution":[doc.width,doc.height], "samples":samples, "profile":if cfg!(debug_assertions){"debug"}else{"release"}, "backend":if gpu.is_some(){"GPU plus readback"}else{"CPU"}, "first_evaluation_ms_p50_p95_p99":percentiles(&mut cold), "repeat_evaluation_ms_p50_p95_p99":percentiles(&mut warm), "crc32":checksums});
            eprintln!("{row}");
            report.push(row);
        }
        crate::gpu_lens::use_on_worker(None);
        std::fs::create_dir_all("target/qa").unwrap();
        let suffix = if gpu.is_some() { "gpu" } else { "cpu" };
        std::fs::write(
            format!("target/qa/production-benchmark-{suffix}.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}
