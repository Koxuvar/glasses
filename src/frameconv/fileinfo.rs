extern crate ffmpeg_next as ffmpeg;
use ffmpeg::{codec::context::Context, format::input, media::Type};
use std::path::Path;

use super::filetypes::{StreamInfo, VideoVariant};

pub fn get_stream_info(input_path: String) -> Result<(StreamInfo, Vec<u8>), ffmpeg::Error> {
    let file_path: &Path = Path::new(&input_path);
    if !file_path.exists() {
        return Err(ffmpeg::Error::Other { errno: 22 });
    }

    let mut ictx = input(&file_path)?;

    let (stream_index, width, height, variant, frame_rate, duration_secs, total_frames) = {
        let stream = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;

        let codec_context = Context::from_parameters(stream.parameters())?;
        let video = codec_context.decoder().video()?;

        let variant = match unsafe { (*stream.parameters().as_ptr()).codec_tag } {
            0x31706148 => VideoVariant::HAP,
            0x35706148 => VideoVariant::HAPALPHA,
            0x59706148 => VideoVariant::HAPQ,
            0x434C544E => VideoVariant::NOTCHLC,
            _ => VideoVariant::UNKOWN,
        };

        let rate = stream.avg_frame_rate();
        let frame_rate = if rate.1 != 0 {
            rate.0 as f64 / rate.1 as f64
        } else {
            24.0
        };

        let time_base = stream.time_base();
        let duration_secs = if stream.duration() > 0 {
            stream.duration() as f64 * time_base.0 as f64 / time_base.1 as f64
        } else {
            ictx.duration() as f64 / 1_000_000.0
        };

        let total_frames = (duration_secs * frame_rate) as i64;

        (
            stream.index(),
            video.width(),
            video.height(),
            variant,
            frame_rate,
            duration_secs,
            total_frames,
        )
    };

    let info = StreamInfo {
        path: input_path,
        width,
        height,
        stream_index,
        variant,
        frame_rate,
        duration_secs,
        total_frames,
    };

    let mut first_frame = Vec::new();
    for (stream, packet) in ictx.packets() {
        if stream.index() == stream_index {
            first_frame = packet.data().unwrap().to_vec();
            break;
        }
    }

    Ok((info, first_frame))
}
