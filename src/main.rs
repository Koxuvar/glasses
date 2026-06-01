extern crate ffmpeg_next as ffmpeg;
use clap::Parser;
use glasses::{frameconv::fileinfo::get_stream_info, window::app::App};
use winit::event_loop::EventLoop;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,
}

fn main() {
    ffmpeg::init().unwrap();
    let args = Args::parse();

    let frame_arr = get_stream_info(args.path).unwrap();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
