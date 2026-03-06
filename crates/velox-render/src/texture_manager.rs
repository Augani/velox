use std::collections::HashMap;
use velox_scene::TextureId;

use crate::gpu::GpuContext;

struct TextureEntry {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    _width: u32,
    _height: u32,
    size_bytes: u64,
    last_used_frame: u64,
}

pub struct TextureManager {
    textures: HashMap<TextureId, TextureEntry>,
    next_id: u64,
    current_frame: u64,
    total_bytes: u64,
    max_bytes: u64,
}

impl TextureManager {
    pub fn new(max_bytes: u64) -> Self {
        Self {
            textures: HashMap::new(),
            next_id: 1,
            current_frame: 0,
            total_bytes: 0,
            max_bytes,
        }
    }

    pub fn upload(&mut self, gpu: &GpuContext, width: u32, height: u32, data: &[u8]) -> TextureId {
        let size_bytes = (width as u64) * (height as u64) * 4;
        let expected_len = (width as usize) * (height as usize) * 4;
        assert!(
            data.len() >= expected_len,
            "data too short: expected {} bytes, got {}",
            expected_len,
            data.len()
        );

        if size_bytes > self.max_bytes {
            eprintln!(
                "[velox] texture too large ({size_bytes} bytes exceeds {max} byte budget), skipping",
                max = self.max_bytes
            );
            return TextureId(0);
        }

        if self.total_bytes + size_bytes > self.max_bytes {
            self.evict_lru(size_bytes);
        }

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture_manager_entry"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let id = TextureId(self.next_id);
        self.next_id += 1;

        self.textures.insert(
            id,
            TextureEntry {
                _texture: texture,
                view,
                _width: width,
                _height: height,
                size_bytes,
                last_used_frame: self.current_frame,
            },
        );
        self.total_bytes += size_bytes;

        id
    }

    pub fn get_view(&mut self, id: TextureId) -> Option<&wgpu::TextureView> {
        let entry = self.textures.get_mut(&id)?;
        entry.last_used_frame = self.current_frame;
        Some(&entry.view)
    }

    pub fn remove(&mut self, id: TextureId) {
        if let Some(entry) = self.textures.remove(&id) {
            self.total_bytes = self.total_bytes.saturating_sub(entry.size_bytes);
        }
    }

    pub fn evict_lru(&mut self, bytes_needed: u64) {
        while self.total_bytes + bytes_needed > self.max_bytes {
            let oldest = self
                .textures
                .iter()
                .min_by_key(|(_, e)| e.last_used_frame)
                .map(|(id, _)| *id);

            match oldest {
                Some(id) => self.remove(id),
                None => break,
            }
        }
    }

    pub fn tick_frame(&mut self) {
        self.current_frame += 1;
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}
