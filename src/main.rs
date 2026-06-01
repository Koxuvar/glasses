use clap::Parser;
use glasses::{frameconv::ff::get_frames, window::app::App};
use winit::event_loop::EventLoop;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,
}
fn main() {
    let args = Args::parse();

    let frame_arr = get_frames(args.path).unwrap();

    for (k, v) in frame_arr {
        println!("Frame: {}, packetSize: {}", k, v);
    }

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
