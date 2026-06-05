#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoVariant {
    HAP = 0x31706148,
    HAPALPHA = 0x35706148,
    HAPQ = 0x59706148,
    NOTCHLC = 0x434C544E,
    UNKOWN,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub stream_index: usize,
    pub variant: VideoVariant,
    pub frame_rate: f64,
    pub duration_secs: f64,
    pub total_frames: i64,
}
