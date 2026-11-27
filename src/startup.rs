//! Keep Windows startup away from Vulkan driver enumeration by default.
use eframe::{egui_wgpu, wgpu};

fn backends(
    windows: bool,
    explicit: Option<wgpu::Backends>,
    default: wgpu::Backends,
) -> wgpu::Backends {
    explicit.unwrap_or(if windows {
        wgpu::Backends::DX12
    } else {
        default
    })
}

pub(crate) fn gpu_configuration() -> egui_wgpu::WgpuConfiguration {
    let mut config = egui_wgpu::WgpuConfiguration::default();
    if let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut config.wgpu_setup {
        setup.instance_descriptor.backends = backends(
            cfg!(windows),
            wgpu::Backends::from_env(),
            setup.instance_descriptor.backends,
        );
    }
    config
}

pub(crate) fn report_error(error: &eframe::Error) {
    let path = std::env::temp_dir().join("LibreToon-startup-error.log");
    let text = format!(
        "LibreToon {}\nStartup error: {error}\nWGPU_BACKEND: {:?}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::var("WGPU_BACKEND").ok()
    );
    let saved = std::fs::write(&path, &text).is_ok();
    let detail = if saved {
        format!("{error}\n\nDiagnostic: {}", path.display())
    } else {
        error.to_string()
    };
    rfd::MessageDialog::new()
        .set_title("LibreToon could not start")
        .set_description(detail)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_does_not_probe_vulkan_unless_explicitly_requested() {
        let all = wgpu::Backends::all();
        assert_eq!(backends(true, None, all), wgpu::Backends::DX12);
        assert_eq!(
            backends(true, Some(wgpu::Backends::VULKAN), all),
            wgpu::Backends::VULKAN
        );
        assert_eq!(backends(false, None, all), all);
    }
}

#[cfg(test)]
mod shader_probe {
    use super::*;
    #[test]
    #[ignore = "requires GPU; compile current post shader using the Windows startup backend"]
    fn startup_backend_compiles_post_shader() {
        use crate::gpu_lens::block_on;
        let egui_wgpu::WgpuSetup::CreateNew(setup) = gpu_configuration().wgpu_setup else {
            panic!("native setup")
        };
        let instance = wgpu::Instance::new(setup.instance_descriptor);
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("shader adapter: {:?}", adapter.get_info());
        let (device, _queue) = block_on(adapter.request_device(&Default::default())).unwrap();
        let path = std::env::var_os("LIBRETOON_SHADER_PROBE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "src/gpu_post.wgsl".into());
        let source = std::fs::read_to_string(path).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DX12 post probe"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let _pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("DX12 post probe"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let invalid = block_on(validation.pop());
        let internal = block_on(scope.pop());
        assert!(
            invalid.is_none() && internal.is_none(),
            "validation={invalid:?}; internal={internal:?}"
        );
    }
}

#[cfg(test)]
pub(crate) fn test_instance() -> wgpu::Instance {
    let egui_wgpu::WgpuSetup::CreateNew(setup) = gpu_configuration().wgpu_setup else {
        panic!("native setup")
    };
    wgpu::Instance::new(setup.instance_descriptor)
}
