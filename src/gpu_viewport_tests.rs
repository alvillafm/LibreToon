use super::*;
#[test]
#[ignore = "requires GPU; real egui callbacks, frame changes, resident cache and fallback"]
fn viewport_uses_shared_device_callbacks_without_readback_and_recovers() {
    use crate::gpu_lens::block_on;
    use eframe::{egui_wgpu, wgpu};
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
    eprintln!("integration GPU: {:?}", adapter.get_info());
    let renderer =
        egui_wgpu::Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, Default::default());
    let render = egui_wgpu::RenderState {
        instance,
        adapter,
        device: device.clone(),
        queue: queue.clone(),
        target_format: wgpu::TextureFormat::Rgba8Unorm,
        renderer: std::sync::Arc::new(egui::mutex::RwLock::new(renderer)),
        available_adapters: Vec::new(),
    };
    let ctx = egui::Context::default();
    let mut cc = eframe::CreationContext::_new_kittest(ctx.clone());
    cc.wgpu_render_state = Some(render.clone());
    let mut editor = crate::Editor::new(&cc);
    let source = std::env::var_os("LIBRETOON_VIEWPORT_PROJECT");
    let doc = source.as_ref().map_or_else(
        || {
            crate::examples::project(":builtin-orbit-lights")
                .unwrap()
                .unwrap()
        },
        |p| crate::documents::read_project(std::path::Path::new(p)).unwrap(),
    );
    editor.add_document(doc, None);
    editor.tool = Tool::Camera;
    editor.feature.selected_camera = editor.doc.extras.cameras.first().map(|c| c.id);
    editor.hub.home = false;
    editor.onion = false;
    editor.studio.prefs.saving.enabled = false;
    editor.canvas.fit();
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [1360, 860],
        pixels_per_point: 1.,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Integration viewport"),
        size: wgpu::Extent3d {
            width: 1360,
            height: 860,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: render.target_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let tick = |editor: &mut crate::Editor| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1360., 860.))),
                ..Default::default()
            },
            |ui| {
                editor.workspace(ui);
                egui::CentralPanel::default().show(ui, |ui| editor.canvas_ui(ui));
            },
        );
        let primitives =
            ctx.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
        let mut renderer = render.renderer.write();
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&device, &queue, *id, delta);
            }
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut commands =
            renderer.update_buffers(&device, &queue, &mut encoder, &primitives, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Integration UI pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            renderer.render(&mut pass, &primitives, &screen);
        }
        commands.push(encoder.finish());
        queue.submit(commands);
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        output.textures_delta.clear();
    };
    let readbacks = crate::gpu_image::READBACKS.load(std::sync::atomic::Ordering::Relaxed);
    let second = 18.min(editor.doc.layers[0].frames.len() - 1);
    for frame in [0, second, 0] {
        editor.frame = frame;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            tick(&mut editor);
            if editor
                .depth
                .presenter
                .as_ref()
                .unwrap()
                .shown_key()
                .is_some_and(|key| key.get(1) == Some(&frame))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "frame {frame} never completed"
            );
            std::thread::sleep(std::time::Duration::from_millis(4));
        }
        assert!(
            editor.depth.job.is_none(),
            "interactive presentation must not also queue a RAM render"
        );
    }
    assert_eq!(
        editor.depth.presenter.as_ref().unwrap().stats().1,
        if second == 0 { 1 } else { 2 },
        "returning to a resident frame reuses its image"
    );
    assert_eq!(
        crate::gpu_image::READBACKS.load(std::sync::atomic::Ordering::Relaxed),
        readbacks
    );
    if std::env::var_os("LIBRETOON_VIEWPORT_BENCH").is_some() {
        let count = editor.doc.layers[0].frames.len().min(16);
        let mut rounds = Vec::new();
        for _ in 0..2 {
            let mut times = Vec::new();
            for frame in 0..count {
                let start = std::time::Instant::now();
                editor.frame = frame;
                loop {
                    tick(&mut editor);
                    if editor
                        .depth
                        .presenter
                        .as_ref()
                        .unwrap()
                        .shown_key()
                        .is_some_and(|key| key.get(1) == Some(&frame))
                    {
                        break;
                    }
                    assert!(
                        start.elapsed() < std::time::Duration::from_secs(15),
                        "viewport benchmark stalled"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(std::time::Duration::from_secs(10)),
                    })
                    .unwrap();
                times.push(start.elapsed().as_secs_f64() * 1000.);
            }
            times.sort_by(f64::total_cmp);
            rounds.push(
                [0.5, 0.95, 0.99].map(|p| times[((times.len() - 1) as f64 * p).ceil() as usize]),
            );
        }
        let key = editor
            .depth
            .presenter
            .as_ref()
            .unwrap()
            .shown_key()
            .unwrap();
        let info = render.adapter.get_info();
        let report = serde_json::json!({"profile":if cfg!(debug_assertions){"debug"}else{"release"},"adapter":info.name,"backend":format!("{:?}",info.backend),"project":source.as_ref().and_then(|p|std::path::Path::new(p).file_name()).map_or("Orbit example".into(),|n|n.to_string_lossy().into_owned()),"headless_ui_size":[1360,860],"preview_size":&key[3..5],"frames":count,"warmed_pipeline_changing_frames_ms_p50_p95_p99":rounds[0],"resident_loop_ms_p50_p95_p99":rounds[1],"scene_readbacks":crate::gpu_image::READBACKS.load(std::sync::atomic::Ordering::Relaxed)-readbacks});
        std::fs::create_dir_all("target/qa").unwrap();
        std::fs::write(
            "target/qa/gpu-viewport-benchmark.json",
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        eprintln!("{report}");
        assert_eq!(
            crate::gpu_image::READBACKS.load(std::sync::atomic::Ordering::Relaxed),
            readbacks
        );
    }
    if editor.doc.layers[0].frames.len() > 5 {
        editor.frame = 0;
        tick(&mut editor);
        let composed = editor.depth.presenter.as_ref().unwrap().stats().2;
        editor.doc.layers[0].frames[5].push(crate::Mark {
            points: vec![[10., 10.], [20., 20.]].into(),
            ..Default::default()
        });
        tick(&mut editor);
        assert_eq!(
            editor.depth.presenter.as_ref().unwrap().stats().2,
            composed,
            "editing a different exposure must not recompose the current resident frame"
        );
        assert_eq!(
            editor
                .depth
                .presenter
                .as_ref()
                .unwrap()
                .shown_key()
                .unwrap()[0],
            editor.depth.preview_epoch
        );
    }
    editor.frame = 0;
    editor.prepare_preview(Some((0, 2)));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        tick(&mut editor);
        if editor.depth.prepare.is_none()
            && editor.depth.cached.iter().any(|f| f.key.get(1) == Some(&1))
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "RAM preparation stopped completing after native presentation"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    editor.frame = 1;
    tick(&mut editor);
    assert_eq!(
        editor.depth.shown.get(1),
        Some(&1),
        "prepared frames remain immediately available"
    );
    editor.depth.presenter.as_ref().unwrap().fail_for_test();
    editor.frame = 5;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        tick(&mut editor);
        if editor.depth.shown.get(1) == Some(&5) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "failed direct renderer did not recover"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!editor.depth.cached.is_empty());
}
