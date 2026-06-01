#[derive(Debug)]
pub enum VideoVariant {
    HAP = 0x31706168,      // HAP
    HAPALPHA = 0x35706168, // HAP Alpha
    HAPQ = 0x59706168,     // HAP Q
    NOTCHLC = 0x434C544E,  // NotchLC
    UNKOWN,                // None
}
#[derive(Debug)]
pub struct StreamInfo {
    pub width: u32,
    pub height: u32,
    pub stream_index: usize,
    pub variant: VideoVariant,
}
