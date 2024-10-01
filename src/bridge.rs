use crate::state::JoystickState;
use color_eyre::eyre::{ContextCompat, Result};
use grafton_visca::{
    command::{
        pan_tilt::{PanSpeed, PanTiltDirection, TiltSpeed},
        PanTiltCommand, ZoomCommand,
    },
    send_command_and_wait, TcpTransport,
};
use std::{net::ToSocketAddrs, thread, time::Duration};
use watch::WatchReceiver;

/// Sends commands to a PTZ camera based on joystick input.
pub struct CameraBridge {
    transport_addr: String,
    state_receiver: WatchReceiver<JoystickState>,
    min_update_interval: Duration,
    pan_tilt_threshold: f32,
    pan_tilt_direction: PanTiltDirection,
    pan_speed: PanSpeed,
    tilt_speed: TiltSpeed,
    last_zoom_position: Option<u16>,
}

impl CameraBridge {
    pub fn new(
        visca_addr: impl ToSocketAddrs,
        state_receiver: WatchReceiver<JoystickState>,
    ) -> Result<Self> {
        let socket_addr = visca_addr
            .to_socket_addrs()?
            .next()
            .wrap_err("invalid host address")?;

        Ok(Self {
            transport_addr: format!("{}:{}", socket_addr.ip(), socket_addr.port()),
            state_receiver,
            min_update_interval: Duration::from_millis(100),
            pan_tilt_threshold: 0.25,
            pan_tilt_direction: PanTiltDirection::Stop,
            pan_speed: PanSpeed::STOP,
            tilt_speed: TiltSpeed::STOP,
            last_zoom_position: None,
        })
    }

    pub fn run(&mut self) {
        loop {
            // Try to connect to the camera via VISCA.
            let mut transport = self.connect_to_camera();

            loop {
                let state = self.state_receiver.wait();
                let mut changes_detected = false;

                if self.handle_pan_tilt(&state) {
                    changes_detected = true;

                    println!(
                        "{:?} - {:?} - {:?}",
                        self.pan_tilt_direction, self.pan_speed, self.tilt_speed,
                    );

                    if let Err(e) = send_command_and_wait(
                        &mut transport,
                        &PanTiltCommand {
                            pan_speed: self.pan_speed,
                            tilt_speed: self.tilt_speed,
                            direction: self.pan_tilt_direction,
                        },
                    ) {
                        eprintln!("Failed to send pan/tilt command: {}", e);
                        break;
                    }
                }

                if self.handle_zoom(&state) {
                    changes_detected = true;

                    println!("{:?}", self.last_zoom_position);

                    if let Some(zoom_position) = self.last_zoom_position {
                        if let Err(e) = send_command_and_wait(
                            &mut transport,
                            &ZoomCommand::Direct(zoom_position),
                        ) {
                            eprintln!("Failed to send zoom command: {}", e);
                            break;
                        }
                    }
                }

                if changes_detected {
                    // Sleep for a while before checking for changes again to prevent
                    // sending commands too fast to the camera.
                    thread::sleep(self.min_update_interval);
                }
            }
        }
    }

    fn connect_to_camera(&self) -> TcpTransport {
        loop {
            match TcpTransport::new(&self.transport_addr) {
                Ok(transport) => return transport,
                Err(e) => {
                    eprintln!("Failed to connect to camera, retrying later: {}", e);
                    thread::sleep(Duration::from_secs(5));
                }
            }
        }
    }

    fn handle_pan_tilt(&mut self, state: &JoystickState) -> bool {
        let x_speed = interpret_axis_speed(state.axis_x, 24, self.pan_tilt_threshold);
        let y_speed = interpret_axis_speed(state.axis_y, 20, self.pan_tilt_threshold);
        let pan_speed = PanSpeed::new(x_speed.unsigned_abs()).unwrap();
        let tilt_speed = TiltSpeed::new(y_speed.unsigned_abs()).unwrap();

        // Determine the direction of movement.
        let direction = match (x_speed, y_speed) {
            (0, 0) => PanTiltDirection::Stop,
            (0, 1..) => PanTiltDirection::Down,
            (0, ..=-1) => PanTiltDirection::Up,
            (1.., 0) => PanTiltDirection::Right,
            (1.., 1..) => PanTiltDirection::DownRight,
            (1.., ..=-1) => PanTiltDirection::UpRight,
            (..=-1, 0) => PanTiltDirection::Left,
            (..=-1, 1..) => PanTiltDirection::DownLeft,
            (..=-1, ..=-1) => PanTiltDirection::UpLeft,
        };

        let mut changed = false;

        if direction != self.pan_tilt_direction {
            self.pan_tilt_direction = direction;
            changed = true;
        }

        if pan_speed.get_value() != self.pan_speed.get_value() {
            self.pan_speed = pan_speed;
            changed = true;
        }

        if tilt_speed.get_value() != self.tilt_speed.get_value() {
            self.tilt_speed = tilt_speed;
            changed = true;
        }

        changed
    }

    fn handle_zoom(&mut self, state: &JoystickState) -> bool {
        let zoom_level = interpret_zoom_level(state.axis_z);

        if Some(zoom_level) != self.last_zoom_position {
            self.last_zoom_position = Some(zoom_level);
            true
        } else {
            false
        }
    }
}

fn interpret_axis_speed(axis_position: f32, axis_max: u8, move_threshold: f32) -> i8 {
    let axis_position = axis_position.clamp(-1.0, 1.0);
    let axis_position_abs = axis_position.abs();

    if axis_position_abs < move_threshold {
        return 0;
    }

    let percentage = (axis_position_abs - move_threshold) / (1.0 - move_threshold);
    let speed = (axis_max as f32 * percentage).round() as i8;

    if axis_position.is_sign_negative() {
        -speed
    } else {
        speed
    }
}

fn interpret_zoom_level(axis_position: f32) -> u16 {
    if axis_position > 0.99 {
        return 4000;
    }

    if axis_position < -0.99 {
        return 0;
    }

    let percentage = axis_position / 2.0 + 0.5;
    (4000.0 * percentage) as u16
}
