use wgpu::util::DeviceExt;
extern crate ffmpeg_next as ffmpeg;

use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::frameconv::{
    decode::decode_hap_packet,
    filetypes::{StreamInfo, VideoVariant},
};

// ── overlay rendering ────────────────────────────────────────────────────────

/// Tiny 5×7 pixel-font bitmap renderer baked into a texture.
/// Each ASCII character 32–126 is stored as a 5-column × 7-row bitmask.
const FONT_W: usize = 5;
const FONT_H: usize = 7;
const GLYPH_COUNT: usize = 95; // chars 32..=126

static FONT: [[u8; FONT_H]; GLYPH_COUNT] = include_font();

const fn include_font() -> [[u8; FONT_H]; GLYPH_COUNT] {
    // Each row is a bitmask of 5 bits (MSB = left column).
    // Only the characters we actually need are fully specified; the rest are spaces.
    let mut f = [[0u8; FONT_H]; GLYPH_COUNT];

    // '0' = 48
    f[16] = [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ];
    f[17] = [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ]; // 1
    f[18] = [
        0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
    ]; // 2
    f[19] = [
        0b11111, 0b00010, 0b00100, 0b00110, 0b00001, 0b10001, 0b01110,
    ]; // 3
    f[20] = [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ]; // 4
    f[21] = [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ]; // 5
    f[22] = [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ]; // 6
    f[23] = [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ]; // 7
    f[24] = [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ]; // 8
    f[25] = [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ]; // 9

    // ':' = 58  index 26
    f[26] = [
        0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
    ];
    // '.' = 46  index 14
    f[14] = [
        0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100,
    ];
    // ' ' = 32  index 0  (already zero)

    // uppercase letters A-Z  (indices 33-58)
    f[33] = [
        0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ]; // A
    f[34] = [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
    ]; // B
    f[35] = [
        0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
    ]; // C
    f[36] = [
        0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100,
    ]; // D
    f[37] = [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
    ]; // E
    f[38] = [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
    ]; // F
    f[39] = [
        0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
    ]; // G
    f[40] = [
        0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ]; // H
    f[41] = [
        0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ]; // I
    f[42] = [
        0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
    ]; // J
    f[43] = [
        0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
    ]; // K
    f[44] = [
        0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
    ]; // L
    f[45] = [
        0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
    ]; // M
    f[46] = [
        0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
    ]; // N
    f[47] = [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ]; // O
    f[48] = [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
    ]; // P
    f[49] = [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
    ]; // Q
    f[50] = [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
    ]; // R
    f[51] = [
        0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
    ]; // S
    f[52] = [
        0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
    ]; // T
    f[53] = [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ]; // U
    f[54] = [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
    ]; // V
    f[55] = [
        0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
    ]; // W
    f[56] = [
        0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
    ]; // X
    f[57] = [
        0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
    ]; // Y
    f[58] = [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
    ]; // Z

    // lowercase a-z (indices 65-90)
    f[65] = [
        0b00000, 0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111,
    ]; // a
    f[66] = [
        0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110,
    ]; // b
    f[67] = [
        0b00000, 0b00000, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110,
    ]; // c
    f[68] = [
        0b00001, 0b00001, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111,
    ]; // d
    f[69] = [
        0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110,
    ]; // e
    f[70] = [
        0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000,
    ]; // f
    f[71] = [
        0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110,
    ]; // g
    f[72] = [
        0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001,
    ]; // h
    f[73] = [
        0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110,
    ]; // i
    f[74] = [
        0b00010, 0b00000, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100,
    ]; // j
    f[75] = [
        0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010,
    ]; // k
    f[76] = [
        0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ]; // l
    f[77] = [
        0b00000, 0b00000, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001,
    ]; // m
    f[78] = [
        0b00000, 0b00000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001,
    ]; // n
    f[79] = [
        0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110,
    ]; // o
    f[80] = [
        0b00000, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000,
    ]; // p
    f[81] = [
        0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001,
    ]; // q
    f[82] = [
        0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000,
    ]; // r
    f[83] = [
        0b00000, 0b00000, 0b01110, 0b10000, 0b01110, 0b00001, 0b11110,
    ]; // s
    f[84] = [
        0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110,
    ]; // t
    f[85] = [
        0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101,
    ]; // u
    f[86] = [
        0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
    ]; // v
    f[87] = [
        0b00000, 0b00000, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
    ]; // w
    f[88] = [
        0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001,
    ]; // x
    f[89] = [
        0b00000, 0b10001, 0b10001, 0b01111, 0b00001, 0b10001, 0b01110,
    ]; // y
    f[90] = [
        0b00000, 0b00000, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111,
    ]; // z

    // '/' = 47  index 15
    f[15] = [
        0b00001, 0b00010, 0b00100, 0b00100, 0b01000, 0b10000, 0b10000,
    ];
    // 'x' already done above as lowercase

    f
}

/// Render text into an RGBA byte buffer at pixel position (px, py).
/// Scale is integer pixel size per font pixel.
fn render_text(buf: &mut Vec<u8>, buf_w: usize, text: &str, px: i32, py: i32, scale: usize) {
    let mut cx = px;
    for ch in text.chars() {
        let idx = (ch as usize).saturating_sub(32);
        if idx >= GLYPH_COUNT {
            cx += (FONT_W as i32 + 1) * scale as i32;
            continue;
        }
        let glyph = &FONT[idx];
        for row in 0..FONT_H {
            for col in 0..FONT_W {
                let bit = (glyph[row] >> (FONT_W - 1 - col)) & 1;
                if bit == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let fx = cx + (col * scale + sx) as i32;
                        let fy = py + (row * scale + sy) as i32;
                        if fx < 0 || fy < 0 {
                            continue;
                        }
                        let fx = fx as usize;
                        let fy = fy as usize;
                        if fx >= buf_w {
                            continue;
                        }
                        let off = (fy * buf_w + fx) * 4;
                        if off + 3 < buf.len() {
                            buf[off] = 255;
                            buf[off + 1] = 255;
                            buf[off + 2] = 255;
                            buf[off + 3] = 220;
                        }
                    }
                }
            }
        }
        cx += (FONT_W as i32 + 1) * scale as i32;
    }
}

// ── App state ────────────────────────────────────────────────────────────────

pub struct App {
    pub stream_info: Option<StreamInfo>,
    pub first_frame_data: Option<Vec<u8>>,

    // wgpu
    pub window: Option<Arc<Window>>,
    pub device: Option<wgpu::Device>,
    pub queue: Option<wgpu::Queue>,
    pub surface: Option<wgpu::Surface<'static>>,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub render_pipeline: Option<wgpu::RenderPipeline>,
    pub texture_bind_group: Option<wgpu::BindGroup>,
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub video_texture: Option<wgpu::Texture>,
    pub use_ycocg_buf: Option<wgpu::Buffer>,
    pub viewport_buf: Option<wgpu::Buffer>,

    // overlay
    pub overlay_texture: Option<wgpu::Texture>,
    pub overlay_pipeline: Option<wgpu::RenderPipeline>,
    pub overlay_bind_group: Option<wgpu::BindGroup>,
    pub overlay_bind_group_layout: Option<wgpu::BindGroupLayout>,

    // ffmpeg
    pub ictx: Option<ffmpeg::format::context::Input>,
    pub stream_index: Option<usize>,

    // playback
    pub playing: bool,
    pub current_frame: i64,
    pub last_frame_time: Option<Instant>,
    pub frame_duration: Option<Duration>,

    // mouse
    pub mouse_down: bool,
    pub mouse_x: f32,
}

impl App {
    pub fn new(stream_info: StreamInfo, first_frame_data: Vec<u8>) -> Self {
        Self {
            stream_info: Some(stream_info),
            first_frame_data: Some(first_frame_data),
            window: None,
            device: None,
            queue: None,
            surface: None,
            config: None,
            render_pipeline: None,
            texture_bind_group: None,
            bind_group_layout: None,
            video_texture: None,
            use_ycocg_buf: None,
            viewport_buf: None,
            overlay_texture: None,
            overlay_pipeline: None,
            overlay_bind_group: None,
            overlay_bind_group_layout: None,
            ictx: None,
            stream_index: None,
            playing: true,
            current_frame: 0,
            last_frame_time: None,
            frame_duration: None,
            mouse_down: false,
            mouse_x: 0.0,
        }
    }

    fn bytes_per_row(variant: VideoVariant, width: u32) -> u32 {
        match variant {
            VideoVariant::HAP => (width / 4) * 8,
            VideoVariant::HAPALPHA | VideoVariant::HAPQ => (width / 4) * 16,
            VideoVariant::NOTCHLC => (width / 4) * 16,
            VideoVariant::UNKOWN => panic!("Unknown variant"),
        }
    }

    fn texture_format(variant: VideoVariant) -> wgpu::TextureFormat {
        match variant {
            VideoVariant::HAP => wgpu::TextureFormat::Bc1RgbaUnorm,
            VideoVariant::HAPALPHA => wgpu::TextureFormat::Bc3RgbaUnorm,
            VideoVariant::HAPQ => wgpu::TextureFormat::Bc3RgbaUnorm,
            VideoVariant::NOTCHLC => wgpu::TextureFormat::Bc6hRgbUfloat,
            VideoVariant::UNKOWN => panic!("Unknown variant"),
        }
    }

    /// Upload decoded frame bytes to the existing video texture.
    fn upload_frame(&self, decoded: &[u8]) {
        let si = self.stream_info.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let texture = self.video_texture.as_ref().unwrap();
        let bpr = Self::bytes_per_row(si.variant, si.width);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            decoded,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bpr),
                rows_per_image: Some(si.height),
            },
            wgpu::Extent3d {
                width: si.width,
                height: si.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Compute the letterbox/pillarbox viewport rectangle in normalised 0..1 coords.
    /// Returns (x, y, w, h) where (x,y) is the top-left corner of the video rect.
    fn compute_viewport(&self) -> [f32; 4] {
        let si = self.stream_info.as_ref().unwrap();
        let win = self.window.as_ref().unwrap().inner_size();
        let win_w = win.width as f32;
        let win_h = win.height as f32;
        let vid_aspect = si.width as f32 / si.height as f32;
        let win_aspect = win_w / win_h;

        let (vw, vh) = if win_aspect > vid_aspect {
            // pillarbox
            let h = 1.0f32;
            let w = vid_aspect / win_aspect;
            (w, h)
        } else {
            // letterbox
            let w = 1.0f32;
            let h = win_aspect / vid_aspect;
            (w, h)
        };

        let vx = (1.0 - vw) * 0.5;
        let vy = (1.0 - vh) * 0.5;
        [vx, vy, vw, vh]
    }

    /// Rebuild the overlay RGBA texture with current info text.
    fn update_overlay(&mut self) {
        let si = self.stream_info.as_ref().unwrap();
        let win = self.window.as_ref().unwrap().inner_size();
        let ow = win.width as usize;
        let oh = win.height as usize;

        let variant_str = match si.variant {
            VideoVariant::HAP => "HAP",
            VideoVariant::HAPALPHA => "HAP Alpha",
            VideoVariant::HAPQ => "HAP Q",
            VideoVariant::NOTCHLC => "NotchLC",
            VideoVariant::UNKOWN => "Unknown",
        };

        let total_secs = si.duration_secs as u64;
        let cur_secs = (self.current_frame as f64 / si.frame_rate) as u64;

        let lines = [
            format!(
                "{}x{}  {:.2}fps  {}",
                si.width, si.height, si.frame_rate, variant_str
            ),
            format!(
                "{:02}:{:02} / {:02}:{:02}  frame {}/{}",
                cur_secs / 60,
                cur_secs % 60,
                total_secs / 60,
                total_secs % 60,
                self.current_frame,
                si.total_frames
            ),
            format!("{}", if self.playing { "PLAYING" } else { "PAUSED" }),
        ];

        let scale = 2usize;
        let line_h = (FONT_H * scale + 4) as i32;
        let margin = 8i32;

        let mut buf = vec![0u8; ow * oh * 4];

        for (i, line) in lines.iter().enumerate() {
            let text_w = line.len() as i32 * (FONT_W as i32 + 1) * scale as i32;
            let px = ow as i32 - text_w - margin;
            let py = margin + i as i32 * line_h;
            render_text(&mut buf, ow, line, px, py, scale);
        }

        // space bar hint at bottom right
        let hint = "SPACE pause  LEFT/RIGHT seek  DRAG scrub";
        let hint_w = hint.len() as i32 * (FONT_W as i32 + 1) * scale as i32;
        let hx = ow as i32 - hint_w - margin;
        let hy = oh as i32 - line_h - margin;
        render_text(&mut buf, ow, hint, hx, hy, scale);

        let queue = self.queue.as_ref().unwrap();
        let texture = self.overlay_texture.as_ref().unwrap();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &buf,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(ow as u32 * 4),
                rows_per_image: Some(oh as u32),
            },
            wgpu::Extent3d {
                width: ow as u32,
                height: oh as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    fn seek_frames(&mut self, delta: i64) {
        let si = self.stream_info.as_ref().unwrap();
        let target = (self.current_frame + delta).max(0).min(si.total_frames - 1);
        let fps = si.frame_rate;
        let target_secs = target as f64 / fps;
        // ffmpeg seek in AV_TIME_BASE units (microseconds)
        let ts = (target_secs * 1_000_000.0) as i64;
        if let Some(ictx) = self.ictx.as_mut() {
            let _ = ictx.seek(ts, ..ts);
        }
        self.current_frame = target;
        self.last_frame_time = Some(Instant::now());
    }
}

// ── ApplicationHandler ───────────────────────────────────────────────────────

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let si = self.stream_info.as_ref().unwrap();

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Glasses")
                        .with_inner_size(winit::dpi::PhysicalSize::new(si.width, si.height)),
                )
                .unwrap(),
        );

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("No suitable GPU adapter found.");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
                ..Default::default()
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        // ── video shader & pipeline ──────────────────────────────────────────
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Video Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let use_ycocg_val: u32 = if si.variant == VideoVariant::HAPQ {
            1
        } else {
            0
        };
        let use_ycocg_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("use_ycocg"),
            contents: bytemuck::cast_slice(&[use_ycocg_val]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let viewport_data = [0.0f32, 0.0, 1.0, 1.0];
        let viewport_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport"),
            contents: bytemuck::cast_slice(&viewport_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Video BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Video Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Video Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ── video texture ────────────────────────────────────────────────────
        let video_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Video Texture"),
            size: wgpu::Extent3d {
                width: si.width,
                height: si.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::texture_format(si.variant),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let video_view = video_texture.create_view(&Default::default());

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Video Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&video_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: use_ycocg_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: viewport_buf.as_entire_binding(),
                },
            ],
        });

        // ── overlay pipeline ─────────────────────────────────────────────────
        let overlay_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Overlay Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
        });

        let overlay_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Overlay BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let overlay_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Overlay Pipeline Layout"),
                bind_group_layouts: &[&overlay_bgl],
                push_constant_ranges: &[],
            });

        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Overlay Pipeline"),
            layout: Some(&overlay_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &overlay_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &overlay_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let overlay_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Overlay Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let overlay_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let overlay_view = overlay_texture.create_view(&Default::default());
        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Overlay Bind Group"),
            layout: &overlay_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&overlay_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&overlay_sampler),
                },
            ],
        });

        // ── open ffmpeg context for playback ─────────────────────────────────
        let path = self.stream_info.as_ref().unwrap().path.clone();
        let ictx = ffmpeg::format::input(&path).unwrap();
        let stream_index = self.stream_info.as_ref().unwrap().stream_index;

        let fps = self.stream_info.as_ref().unwrap().frame_rate;
        let frame_duration = Duration::from_secs_f64(1.0 / fps);

        // assign everything
        self.window = Some(window);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.config = Some(config);
        self.render_pipeline = Some(render_pipeline);
        self.bind_group_layout = Some(bind_group_layout);
        self.video_texture = Some(video_texture);
        self.texture_bind_group = Some(texture_bind_group);
        self.use_ycocg_buf = Some(use_ycocg_buf);
        self.viewport_buf = Some(viewport_buf);
        self.overlay_texture = Some(overlay_texture);
        self.overlay_pipeline = Some(overlay_pipeline);
        self.overlay_bind_group = Some(overlay_bind_group);
        self.overlay_bind_group_layout = Some(overlay_bgl);
        self.ictx = Some(ictx);
        self.stream_index = Some(stream_index);
        self.frame_duration = Some(frame_duration);
        self.last_frame_time = Some(Instant::now());

        // upload first frame
        let first = self.first_frame_data.take().unwrap();
        let decoded = decode_hap_packet(&first);
        self.upload_frame(&decoded);

        // update viewport uniform
        let vp = self.compute_viewport();
        self.queue.as_ref().unwrap().write_buffer(
            self.viewport_buf.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&vp),
        );

        self.update_overlay();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device), Some(config)) = (
                    self.surface.as_ref(),
                    self.device.as_ref(),
                    self.config.as_mut(),
                ) {
                    config.width = new_size.width.max(1);
                    config.height = new_size.height.max(1);
                    surface.configure(device, config);
                }
                // rebuild overlay texture at new size
                if let Some(device) = self.device.as_ref() {
                    let win = self.window.as_ref().unwrap().inner_size();
                    let new_overlay = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Overlay Texture"),
                        size: wgpu::Extent3d {
                            width: win.width.max(1),
                            height: win.height.max(1),
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    let overlay_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                        mag_filter: wgpu::FilterMode::Nearest,
                        min_filter: wgpu::FilterMode::Nearest,
                        ..Default::default()
                    });
                    let overlay_view = new_overlay.create_view(&Default::default());
                    let new_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Overlay Bind Group"),
                        layout: self.overlay_bind_group_layout.as_ref().unwrap(),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&overlay_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&overlay_sampler),
                            },
                        ],
                    });
                    self.overlay_texture = Some(new_overlay);
                    self.overlay_bind_group = Some(new_bg);
                }
                // update viewport uniform
                let vp = self.compute_viewport();
                if let (Some(queue), Some(buf)) = (self.queue.as_ref(), self.viewport_buf.as_ref())
                {
                    queue.write_buffer(buf, 0, bytemuck::cast_slice(&vp));
                }
                self.update_overlay();
                self.window.as_ref().unwrap().request_redraw();
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key {
                KeyCode::Space => {
                    self.playing = !self.playing;
                    self.update_overlay();
                    self.window.as_ref().unwrap().request_redraw();
                }
                KeyCode::ArrowRight => {
                    self.seek_frames(1);
                    self.window.as_ref().unwrap().request_redraw();
                }
                KeyCode::ArrowLeft => {
                    self.seek_frames(-1);
                    self.window.as_ref().unwrap().request_redraw();
                }
                KeyCode::Escape => event_loop.exit(),
                _ => {}
            },

            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                self.mouse_down = state == ElementState::Pressed;
                if state == ElementState::Pressed {
                    self.playing = false;
                } else {
                    self.playing = true;
                    self.last_frame_time = Some(Instant::now());
                }
                self.update_overlay();
                self.window.as_ref().unwrap().request_redraw();
            }

            WindowEvent::CursorMoved { position, .. } => {
                let win = self.window.as_ref().unwrap().inner_size();
                let norm_x = (position.x as f32 / win.width as f32).clamp(0.0, 1.0);
                self.mouse_x = norm_x;
                if self.mouse_down {
                    let si = self.stream_info.as_ref().unwrap();
                    let target_frame = (norm_x * si.total_frames as f32) as i64;
                    let delta = target_frame - self.current_frame;
                    if delta != 0 {
                        self.seek_frames(delta);
                        self.update_overlay();
                        self.window.as_ref().unwrap().request_redraw();
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                // ── advance frame if playing ─────────────────────────────────
                if self.playing {
                    let now = Instant::now();
                    let elapsed = now.duration_since(self.last_frame_time.unwrap());
                    if elapsed >= self.frame_duration.unwrap() {
                        self.last_frame_time = Some(now);

                        let stream_index = self.stream_index.unwrap();
                        let mut got_frame = false;

                        // try to get the next video packet
                        let packet_data: Option<Vec<u8>> = {
                            let ictx = self.ictx.as_mut().unwrap();
                            let mut result = None;
                            for (stream, packet) in ictx.packets() {
                                if stream.index() == stream_index {
                                    result = packet.data().map(|d| d.to_vec());
                                    break;
                                }
                            }
                            result
                        };

                        if let Some(raw) = packet_data {
                            let decoded = decode_hap_packet(&raw);
                            self.upload_frame(&decoded);
                            self.current_frame += 1;
                            got_frame = true;
                        }

                        if !got_frame {
                            // end of file — loop
                            let si = self.stream_info.as_ref().unwrap();
                            let _ = self.ictx.as_mut().unwrap().seek(0, ..0);
                            self.current_frame = 0;
                            // update overlay with reset frame count
                        }

                        self.update_overlay();
                    }
                }

                // ── render ───────────────────────────────────────────────────
                let surface = self.surface.as_ref().unwrap();
                let device = self.device.as_ref().unwrap();
                let queue = self.queue.as_ref().unwrap();
                let pipeline = self.render_pipeline.as_ref().unwrap();
                let bind_group = self.texture_bind_group.as_ref().unwrap();
                let overlay_pipeline = self.overlay_pipeline.as_ref().unwrap();
                let overlay_bind_group = self.overlay_bind_group.as_ref().unwrap();

                let output = surface.get_current_texture().unwrap();
                let view = output.texture.create_view(&Default::default());
                let mut encoder = device.create_command_encoder(&Default::default());

                // video pass
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.draw(0..6, 0..1);
                }

                // overlay pass (alpha blend on top)
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(overlay_pipeline);
                    pass.set_bind_group(0, overlay_bind_group, &[]);
                    pass.draw(0..6, 0..1);
                }

                queue.submit([encoder.finish()]);
                output.present();
                self.window.as_ref().unwrap().request_redraw();
            }

            _ => {}
        }
    }
}
