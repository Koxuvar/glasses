extern crate ffmpeg_next as ffmpeg;
use ffmpeg::{format::input, media::Type};
use std::{collections::HashMap, path::Path};

pub fn get_frames(path: String) -> Result<HashMap<usize, usize>, ffmpeg::Error> {
    let file_path: &Path = Path::new(&path);
    if !file_path.exists() {
        return Err(ffmpeg::Error::Other { errno: 2 });
    }

    ffmpeg::init().unwrap();

    let mut ictx = input(&file_path)?;
    let stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let stream_index = stream.index();

    let mut packet_vec = HashMap::new();

    for (i, (stream, packet)) in ictx.packets().enumerate() {
        if stream.index() == stream_index {
            let first_pac = packet.data();
            packet_vec.entry(i).or_insert(first_pac.unwrap().len());
        }
    }

    Ok(packet_vec)
}
