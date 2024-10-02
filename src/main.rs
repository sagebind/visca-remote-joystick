use clap::Parser;
use color_eyre::eyre::Result;
use std::thread;

mod bridge;
mod controller;
mod state;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The name of the controller to use
    #[arg(long)]
    controller: String,

    #[arg(long, default_value_t = 0.25)]
    pan_tilt_threshold: f32,

    #[arg(long, default_value_t = 16)]
    pan_max_speed: u8,

    #[arg(long, default_value_t = 16)]
    tilt_max_speed: u8,

    #[arg(long, default_value_t = true)]
    invert_z_axis: bool,

    /// The address of the camera to control
    #[arg(long)]
    visca_host: String,

    /// The port of the camera to control
    #[arg(long, default_value_t = 1259)]
    visca_port: u16,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut controller_monitor = controller::ControllerMonitor::new(&args.controller);
    controller_monitor.select_gamepad();
    let receiver = controller_monitor.state_receiver();

    let mut bridge = bridge::CameraBridge::new(
        (args.visca_host, args.visca_port),
        args.pan_tilt_threshold,
        args.pan_max_speed,
        args.tilt_max_speed,
        args.invert_z_axis,
        receiver,
    )?;

    thread::spawn(move || bridge.run());

    loop {
        controller_monitor.run();
    }
}
