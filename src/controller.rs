use std::time::Duration;

use crate::state::JoystickState;
use gilrs::{Axis, EventType, GamepadId, Gilrs};
use watch::{WatchReceiver, WatchSender};

/// Monitors input from a game controller, and reports state changes.
pub struct ControllerMonitor {
    name_matches: String,
    gilrs: Gilrs,
    selected_gamepad: Option<GamepadId>,
    sender: WatchSender<JoystickState>,
    receiver: WatchReceiver<JoystickState>,
}

impl ControllerMonitor {
    /// Create a new monitor. Only events from the first controller found whose
    /// name contains the given string will be monitored.
    pub fn new(name_matches: &str) -> Self {
        let (sender, receiver) = watch::channel(JoystickState::default());

        Self {
            name_matches: name_matches.to_string(),
            gilrs: Gilrs::new().unwrap(),
            selected_gamepad: None,
            sender,
            receiver,
        }
    }

    /// Get a receiver for getting notified of state changes.
    pub fn state_receiver(&self) -> WatchReceiver<JoystickState> {
        self.receiver.clone()
    }

    pub fn run(&mut self) {
        while let Some(event) = self.gilrs.next_event_blocking(Some(Duration::from_secs(1))) {
            // Re-select a gamepad whenever one is connected or disconnected.
            if let EventType::Connected | EventType::Disconnected = event.event {
                self.select_gamepad();
            }
            // Handle events from the selected gamepad.
            else if let Some(id) = self.selected_gamepad.as_ref() {
                if &event.id == id {
                    match event.event {
                        EventType::AxisChanged(Axis::LeftStickX, value, ..) => {
                            self.sender.update(|f| f.axis_x = value);
                        }
                        EventType::AxisChanged(Axis::LeftStickY, value, ..) => {
                            self.sender.update(|f| f.axis_y = value);
                        }
                        EventType::AxisChanged(
                            Axis::LeftZ | Axis::RightZ | Axis::RightStickX | Axis::Unknown,
                            value,
                            ..,
                        ) => {
                            self.sender.update(|f| f.axis_z = value);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn select_gamepad(&mut self) {
        println!("Discovering game controllers...");
        let mut selected = None;

        for (id, gamepad) in self.gilrs.gamepads() {
            println!("  {}", gamepad.name());

            if selected.is_none() && gamepad.name().contains(&self.name_matches) {
                selected = Some(id);
            }
        }

        self.selected_gamepad = selected;
    }
}
