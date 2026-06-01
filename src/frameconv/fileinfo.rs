extern crate ffmpeg_next as ffmpeg;
use ffmpeg::{format::input, media::Type};
use ffmpeg_next::codec::Context;
use std::path::Path;

use crate::frameconv::filetypes::{StreamInfo, VideoVariant};

pub fn get_stream_info(path: String) -> Result<StreamInfo, ffmpeg::Error> {
    let file_path: &Path = Path::new(&path);
    if !file_path.exists() {
        return Err(ffmpeg::Error::Other { errno: 2 });
    }

    let ictx = input(&file_path)?;
    let stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;

    let codec_context = Context::from_parameters(stream.parameters())?;
    let video = codec_context.decoder().video()?;

    let info = StreamInfo {
        width: video.width() as u32,
        height: video.height() as u32,
        stream_index: stream.index(),
        variant: match unsafe { (*stream.parameters().as_ptr()).codec_tag } {
            0x31706168 => VideoVariant::HAP,
            0x35706168 => VideoVariant::HAPALPHA,
            0x59706168 => VideoVariant::HAPQ,
            0x434C544E => VideoVariant::NOTCHLC,
            _ => VideoVariant::None,
        },
    };

    Ok(info)
}

// pub fn get_frames(path: String) -> Result<HashMap<usize, usize>, ffmpeg::Error> {
//     let thing = get_stream(path).unwrap();
//     let mut ictx = thing.0;
//     let stream = thing.1;
//
//     let stream_index = stream.stream_index;
//
//     let mut packet_vec = HashMap::new();
//
//     for (i, (stream, packet)) in ictx.packets().enumerate() {
//         if stream.index() == stream_index {
//             let first_pac = packet.data();
//             packet_vec.entry(i).or_insert(first_pac.unwrap().len());
//         }
//     }
//
//     Ok(packet_vec)
// }
