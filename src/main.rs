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

    /// The address of the camera to control
    #[arg(long)]
    visca_host: String,

    /// The port of the camera to control
    #[arg(long, default_value_t = 5678)]
    visca_port: u16,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut controller_monitor = controller::ControllerMonitor::new(&args.controller);
    controller_monitor.select_gamepad();
    let receiver = controller_monitor.state_receiver();

    let mut bridge = bridge::CameraBridge::new((args.visca_host, args.visca_port), receiver)?;

    thread::spawn(move || bridge.run());

    loop {
        controller_monitor.run();
    }
}
