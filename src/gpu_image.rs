//! Immutable device images shared by the frame graph and bounded scene caches.
use crate::gpu_lens::block_on;
use eframe::wgpu;
#[cfg(test)]
use eframe::wgpu::util::DeviceExt;
use std::{sync::Arc, time::Duration};
#[cfg(test)]
pub(crate) static READBACKS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
pub(crate) struct Image {
    pub buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
    pub bytes_per_pixel: u32,
}
impl Image {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, bytes_per_pixel: u32) -> Arc<Self> {
        Arc::new(Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Frame graph image"),
                size: u64::from(width) * u64::from(height) * u64::from(bytes_per_pixel),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            width,
            height,
            bytes_per_pixel,
        })
    }
    #[cfg(test)]
    pub fn upload(device: &wgpu::Device, width: u32, height: u32, pixels: &[u8]) -> Arc<Self> {
        Arc::new(Self {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frame graph input"),
                contents: pixels,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            }),
            width,
            height,
            bytes_per_pixel: 4,
        })
    }
    pub fn bytes(&self) -> usize {
        self.width as usize * self.height as usize * self.bytes_per_pixel as usize
    }
    pub fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Vec<u8>> {
        #[cfg(test)]
        READBACKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let bytes = self.bytes() as u64;
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Completed frame transfer"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.buffer, 0, &read, 0, bytes);
        let submission = queue.submit([encoder.finish()]);
        let slice = read.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(3)),
            })
            .ok()?;
        rx.recv_timeout(Duration::from_secs(3)).ok()?.ok()?;
        let mapped = slice.get_mapped_range().ok()?;
        let result = mapped.to_vec();
        drop(mapped);
        read.unmap();
        Some(result)
    }
}

pub(crate) fn checked_scope<T>(
    device: &wgpu::Device,
    draw: impl FnOnce() -> Option<T>,
) -> Option<T> {
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let allocation = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let result = draw();
    let bad = block_on(internal.pop()).is_some()
        | block_on(allocation.pop()).is_some()
        | block_on(validation.pop()).is_some();
    if bad { None } else { result }
}
