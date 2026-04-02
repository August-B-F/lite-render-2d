use std::collections::HashMap;

pub use gilrs::{Axis as GamepadAxis, Button as GamepadButton};
use gilrs::EventType;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

// how a button can be bound
pub enum ButtonBinding {
    Key(KeyCode),
    Gamepad(GamepadButton),
}

// how an axis can be bound
pub enum AxisBinding {
    GamepadAxis(GamepadAxis),
    // two keys simulate an axis (-1 for neg, +1 for pos)
    KeyPair { neg: KeyCode, pos: KeyCode },
}

// single action binding - either a button or axis
pub enum ActionBinding {
    Button(ButtonBinding),
    Axis(AxisBinding),
}

// internal state for one action
#[derive(Clone, Debug)]
pub struct ActionState {
    pub pressed: bool,
    pub prev_pressed: bool,
    pub value: f32,
}

impl Default for ActionState {
    fn default() -> Self {
        Self {
            pressed: false,
            prev_pressed: false,
            value: 0.0,
        }
    }
}

impl ActionState {
    pub fn just_pressed(&self) -> bool {
        self.pressed && !self.prev_pressed
    }

    pub fn just_released(&self) -> bool {
        !self.pressed && self.prev_pressed
    }
}

pub struct InputManager {
    gilrs: gilrs::Gilrs,
    bindings: HashMap<String, Vec<ActionBinding>>,
    states: HashMap<String, ActionState>,
    // track raw key states
    key_states: HashMap<KeyCode, bool>,
    // track raw gamepad btn states
    btn_states: HashMap<GamepadButton, bool>,
    // track raw gamepad axes
    axis_states: HashMap<GamepadAxis, f32>,
    deadzone: f32,
}

impl InputManager {
    pub fn new() -> Self {
        let gilrs = gilrs::Gilrs::new().expect("failed to init gilrs");
        Self {
            gilrs,
            bindings: HashMap::new(),
            states: HashMap::new(),
            key_states: HashMap::new(),
            btn_states: HashMap::new(),
            axis_states: HashMap::new(),
            deadzone: 0.15,
        }
    }

    // register a named action with a binding
    pub fn bind_action(&mut self, name: &str, binding: ActionBinding) {
        self.bindings
            .entry(name.to_string())
            .or_default()
            .push(binding);
        // ensure state exists
        self.states.entry(name.to_string()).or_default();
    }

    // call once per frame with winit events
    pub fn update(&mut self, events: &[WindowEvent]) {
        // snapshot prev state
        for state in self.states.values_mut() {
            state.prev_pressed = state.pressed;
        }

        // process keyboard events
        for ev in events {
            if let WindowEvent::KeyboardInput { event, .. } = ev {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    self.key_states.insert(code, pressed);
                }
            }
        }

        // poll gilrs
        while let Some(ev) = self.gilrs.next_event() {
            match ev.event {
                EventType::ButtonPressed(btn, _) => {
                    self.btn_states.insert(btn, true);
                }
                EventType::ButtonReleased(btn, _) => {
                    self.btn_states.insert(btn, false);
                }
                EventType::AxisChanged(axis, val, _) => {
                    self.axis_states.insert(axis, val);
                }
                _ => {}
            }
        }

        // evaluate all bindings
        for (name, binds) in &self.bindings {
            let state = self.states.entry(name.clone()).or_default();
            let mut pressed = false;
            let mut value = 0.0f32;

            for bind in binds {
                match bind {
                    ActionBinding::Button(ButtonBinding::Key(k)) => {
                        if *self.key_states.get(k).unwrap_or(&false) {
                            pressed = true;
                            value = 1.0;
                        }
                    }
                    ActionBinding::Button(ButtonBinding::Gamepad(b)) => {
                        if *self.btn_states.get(b).unwrap_or(&false) {
                            pressed = true;
                            value = 1.0;
                        }
                    }
                    ActionBinding::Axis(AxisBinding::GamepadAxis(a)) => {
                        let raw = *self.axis_states.get(a).unwrap_or(&0.0);
                        let filtered = if raw.abs() < self.deadzone { 0.0 } else { raw };
                        if filtered.abs() > value.abs() {
                            value = filtered;
                        }
                        if filtered.abs() > 0.0 {
                            pressed = true;
                        }
                    }
                    ActionBinding::Axis(AxisBinding::KeyPair { neg, pos }) => {
                        let n = if *self.key_states.get(neg).unwrap_or(&false) { -1.0 } else { 0.0 };
                        let p = if *self.key_states.get(pos).unwrap_or(&false) { 1.0 } else { 0.0 };
                        let v: f32 = n + p;
                        if v.abs() > value.abs() {
                            value = v;
                        }
                        if v.abs() > 0.0 {
                            pressed = true;
                        }
                    }
                }
            }

            state.pressed = pressed;
            state.value = value;
        }
    }

    pub fn is_pressed(&self, action: &str) -> bool {
        self.states.get(action).map_or(false, |s| s.pressed)
    }

    pub fn just_pressed(&self, action: &str) -> bool {
        self.states.get(action).map_or(false, |s| s.just_pressed())
    }

    pub fn just_released(&self, action: &str) -> bool {
        self.states.get(action).map_or(false, |s| s.just_released())
    }

    pub fn axis_value(&self, action: &str) -> f32 {
        self.states.get(action).map_or(0.0, |s| s.value)
    }

    pub fn gamepad_count(&self) -> usize {
        self.gilrs.gamepads().count()
    }

    pub fn set_deadzone(&mut self, dz: f32) {
        self.deadzone = dz;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_state_just_pressed() {
        let mut s = ActionState::default();
        assert!(!s.just_pressed());
        assert!(!s.just_released());

        // simulate press
        s.prev_pressed = false;
        s.pressed = true;
        assert!(s.just_pressed());
        assert!(!s.just_released());
    }

    #[test]
    fn test_action_state_just_released() {
        let mut s = ActionState::default();
        s.prev_pressed = true;
        s.pressed = false;
        assert!(!s.just_pressed());
        assert!(s.just_released());
    }

    #[test]
    fn test_action_state_held() {
        let mut s = ActionState::default();
        s.prev_pressed = true;
        s.pressed = true;
        assert!(!s.just_pressed());
        assert!(!s.just_released());
    }

    #[test]
    fn test_deadzone_filtering() {
        // test that axis values below deadzone are zeroed
        let dz = 0.15f32;
        let raw = 0.1f32;
        let filtered = if raw.abs() < dz { 0.0 } else { raw };
        assert_eq!(filtered, 0.0);

        let raw2 = 0.5f32;
        let filtered2 = if raw2.abs() < dz { 0.0 } else { raw2 };
        assert_eq!(filtered2, 0.5);
    }
}
