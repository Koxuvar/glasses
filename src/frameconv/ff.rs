extern crate ffmpeg_next as ffmpeg;
use ffmpeg::{format::input, media::Type};
use std::{collections::HashMap, env};

pub fn get_frames() -> Result<HashMap<usize, usize>, ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let mut ictx = input(&env::args().nth(1).expect("Cant open file"))?;
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
