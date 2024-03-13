use std::time::Instant;

use epgi_2d::Point2d;
use epgi_common::{
    PointerButtons, PointerContactData, PointerContactProfile, PointerDeviceKind, PointerEvent,
    PointerEventCommonData, PointerHoverData, PointerInteractionId,
};
use epgi_core::foundation::SyncMpscSender;
use hashbrown::{hash_map::Entry, HashMap};

use crate::utils::ToEpgiExt;

pub(crate) struct WinitPointerEventConverter {
    pointer_devices: HashMap<winit::event::DeviceId, WinitPointerDeviceState>,
    interaction_id_counter: InteractonIdCounter,
    tx: SyncMpscSender<PointerEvent>,
}

struct InteractonIdCounter(u64);

impl InteractonIdCounter {
    fn generate(&mut self) -> PointerInteractionId {
        let result = PointerInteractionId::new(self.0);
        self.0 += 1;
        result
    }
}

enum WinitPointerDeviceState {
    Cursor(PointerState),
    Touch(HashMap<u64, PointerState>),
}

enum PointerState {
    ToBeAdded,
    Added {
        last_position: Point2d,
    },
    Interacting {
        interaction_id: PointerInteractionId,
        pressed_buttons: PointerButtons,
        last_position: Point2d,
    },
}

impl WinitPointerEventConverter {
    pub(crate) fn new(tx: SyncMpscSender<PointerEvent>) -> Self {
        Self {
            pointer_devices: Default::default(),
            interaction_id_counter: InteractonIdCounter(0),
            tx,
        }
    }

    #[inline]
    pub(crate) fn convert(&mut self, event: &winit::event::WindowEvent) {
        use winit::event::WindowEvent::*;
        let time_stamp = Instant::now();
        match event {
            CursorMoved {
                device_id,
                position,
            } => {
                let state = self.pointer_devices.entry(*device_id).or_insert_with(|| {
                    log::warn!(
                        "Winit produces CursorMoved event while the cursor has not been registered."
                    );
                    WinitPointerDeviceState::Cursor(PointerState::ToBeAdded)
                });
                let WinitPointerDeviceState::Cursor(state) = state else {
                    panic!("Potential winit bug detected: cursor event delivered under a non-cursor device id.")
                };
                let physical_position = position.to_epgi();
                let common = PointerEventCommonData {
                    time_stamp,
                    physical_position,
                    pointer_kind: PointerDeviceKind::Mouse,
                    synthesized: false,
                };
                use PointerState::*;
                match state {
                    ToBeAdded => {
                        self.tx
                            .send(PointerEvent::new_added(common.clone()))
                            .unwrap();
                        *state = Added {
                            last_position: physical_position,
                        };
                    }
                    Added { last_position } => {
                        *last_position = physical_position;
                        self.tx
                            .send(PointerEvent::new_hover(
                                common,
                                PointerHoverData::new_mouse(),
                            ))
                            .unwrap();
                    }
                    Interacting {
                        interaction_id,
                        pressed_buttons,
                        last_position,
                    } => {
                        *last_position = physical_position;
                        self.tx
                            .send(PointerEvent::new_move(
                                common,
                                *interaction_id,
                                PointerContactData::new_mouse(pressed_buttons.clone()),
                            ))
                            .unwrap();
                    }
                }
            }
            CursorEntered { device_id } => {
                let state = self
                    .pointer_devices
                    .entry(*device_id)
                    .or_insert_with(|| WinitPointerDeviceState::Cursor(PointerState::ToBeAdded));
                let WinitPointerDeviceState::Cursor(state) = state else {
                    panic!(
                        "Potential winit bug detected: \
                        cursor event delivered under a non-cursor device id."
                    )
                };
                use PointerState::*;
                match state {
                    ToBeAdded => {}
                    Added { last_position } | Interacting { last_position, .. } => {
                        log::warn!(
                            "Winit produces CursorEntered event while the cursor has not left."
                        );
                        let common = PointerEventCommonData {
                            time_stamp,
                            physical_position: last_position.clone(),
                            pointer_kind: PointerDeviceKind::Mouse,
                            synthesized: false,
                        };
                        if let Interacting { interaction_id, .. } = state {
                            self.tx
                                .send(PointerEvent::new_cancel(common.clone(), *interaction_id))
                                .unwrap();
                        }
                        self.tx.send(PointerEvent::new_removed(common)).unwrap();
                        *state = ToBeAdded;
                    }
                }
            }
            CursorLeft { device_id } => {
                let Entry::Occupied(mut entry) = self.pointer_devices.entry(*device_id) else {
                    log::warn!(
                        "Winit produces CursorLeft event while the cursor has not been registered"
                    );
                    return;
                };
                let WinitPointerDeviceState::Cursor(state) = entry.get_mut() else {
                    panic!(
                        "Potential winit bug detected: \
                        cursor event delivered under a non-cursor device id."
                    )
                };
                use PointerState::*;
                match state {
                    ToBeAdded => {}
                    Added { last_position } | Interacting { last_position, .. } => {
                        let common = PointerEventCommonData {
                            time_stamp,
                            physical_position: last_position.clone(),
                            pointer_kind: PointerDeviceKind::Mouse,
                            synthesized: false,
                        };
                        if let Interacting { interaction_id, .. } = state {
                            self.tx
                                .send(PointerEvent::new_cancel(common.clone(), *interaction_id))
                                .unwrap();
                        }
                        self.tx.send(PointerEvent::new_removed(common)).unwrap();
                    }
                }
                entry.remove();
            }
            MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                log::error!("Pointer signal event is not implemented")
            }
            MouseInput {
                device_id,
                state: press,
                button,
            } => {
                let Entry::Occupied(mut entry) = self.pointer_devices.entry(*device_id) else {
                    log::error!(
                        "Winit produces MouseInput event while the cursor has not been registered"
                    );
                    return;
                };
                let WinitPointerDeviceState::Cursor(state) = entry.get_mut() else {
                    panic!(
                        "Potential winit bug detected: \
                        cursor event delivered under a non-cursor device id."
                    )
                };

                use PointerState::*;
                let (Added { last_position } | Interacting { last_position, .. }) = state else {
                    log::error!(
                        "Winit produces MouseInput event while the cursor has not been added"
                    );
                    return;
                };
                let common = PointerEventCommonData {
                    time_stamp,
                    physical_position: last_position.clone(),
                    pointer_kind: PointerDeviceKind::Mouse,
                    synthesized: false,
                };
                let button = button.to_epgi();
                use winit::event::ElementState::*;
                // We have to write this nested match instead of a binary tuple match
                // Because the NLL got messed up with binary tuple match
                // and prevent us from writing back to the mutable reference.
                match state {
                    Added { last_position } => match press {
                        Pressed => {
                            let interaction_id = self.interaction_id_counter.generate();
                            self.tx.send(PointerEvent::new_down(
                                common,
                                interaction_id,
                                PointerContactData::new_mouse(button),
                            ))
                            .unwrap();
                            *state = Interacting {
                                interaction_id,
                                pressed_buttons: button,
                                last_position: last_position.clone(),
                            };
                        }
                        Released => log::warn!(
                            "Winit produced a button release event while no button has been pressed"
                        ),
                    },
                    Interacting {
                        interaction_id,
                        pressed_buttons,
                        last_position,
                    } => match press {
                        Pressed => {
                            self.tx.send(PointerEvent::new_move(
                                common,
                                *interaction_id,
                                PointerContactData::new_mouse(button),
                            ))
                            .unwrap();
                            pressed_buttons.extend(button);
                        }
                        Released => {
                            let buttons = pressed_buttons.difference(button);
                            if buttons.is_empty() {
                                self.tx.send(PointerEvent::new_up(
                                    common,
                                    *interaction_id,
                                    PointerHoverData::new_mouse(),
                                ))
                                .unwrap();
                                *state = Added {
                                    last_position: last_position.clone(),
                                };
                            } else {
                                self.tx.send(PointerEvent::new_move(
                                    common,
                                    *interaction_id,
                                    PointerContactData::new_mouse(buttons),
                                ))
                                .unwrap();
                            }
                        }
                    },
                    ToBeAdded => log::error!("Winit produces a CursorInput event while the pointer has no registered position"),
                };
            }
            TouchpadMagnify {
                device_id,
                delta,
                phase,
            } => {
                log::error!("Pointer signal event is not implemented")
            }
            SmartMagnify { device_id } => {
                log::error!("Smart magnify event is not implemented")
            }
            TouchpadRotate {
                device_id,
                delta,
                phase,
            } => {
                log::error!("Pointer signal event is not implemented")
            }
            TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => {
                log::error!("Force touch is not implemented")
            }
            AxisMotion {
                device_id,
                axis,
                value,
            } => {
                log::error!("Pointer signal event is not implemented")
            }
            Touch(touch) => convert_winit_touch(self, touch, time_stamp),
            _ => {}
        }
    }
}

fn convert_winit_touch(
    converter: &mut WinitPointerEventConverter,
    touch: &winit::event::Touch,
    time_stamp: Instant,
) {
    let winit::event::Touch {
        device_id,
        phase,
        location,
        force,
        id,
        ..
    } = *touch;

    let physical_location = location.to_epgi();
    let common = PointerEventCommonData {
        time_stamp,
        physical_position: physical_location.clone(),
        pointer_kind: PointerDeviceKind::Touch,
        synthesized: false,
    };
    let profile = force.map_or_else(PointerContactProfile::new_mouse, |force| match force {
        winit::event::Force::Calibrated {
            force,
            max_possible_force,
            altitude_angle,
        } => PointerContactProfile {
            pressure: force as _,
            pressure_min: 0.0,
            pressure_max: max_possible_force as _,
            size: 0.0,
            radius_major: 0.0,
            radius_minor: 0.0,
            radius_min: 0.0,
            radius_max: 0.0,
            orientation: 0.0,
            tilt: altitude_angle.map_or(0.0, |altitude_angle| {
                f32::abs(std::f32::consts::FRAC_PI_2 - (altitude_angle as f32))
            }),
        },
        winit::event::Force::Normalized(force) => PointerContactProfile {
            pressure: force as _,
            pressure_min: 0.0,
            pressure_max: 1.0,
            size: 0.0,
            radius_major: 0.0,
            radius_minor: 0.0,
            radius_min: 0.0,
            radius_max: 0.0,
            orientation: 0.0,
            tilt: 0.0,
        },
    });

    let entry = converter.pointer_devices.entry(device_id);
    let states = entry.or_insert_with(|| WinitPointerDeviceState::Touch(Default::default()));
    let WinitPointerDeviceState::Touch(states) = states else {
        panic!("Potential winit bug detected: touch event delivered under a non-touch device id.")
    };
    let finger_entry = states.entry(id);
    let state = finger_entry.or_insert_with(|| PointerState::ToBeAdded);

    use winit::event::TouchPhase::*;
    use PointerState::*;
    match state {
        ToBeAdded | Added { .. } => {
            if let ToBeAdded = state {
                converter
                    .tx
                    .send(PointerEvent::new_added(common.clone()))
                    .unwrap();
            } else if let Added { last_position } = state {
                if *last_position != physical_location {
                    converter
                        .tx
                        .send(PointerEvent::new_hover(
                            common.clone(),
                            PointerHoverData::new_mouse(),
                        ))
                        .unwrap();
                }
            }
            if phase == Started || phase == Moved {
                let interaction_id = converter.interaction_id_counter.generate();
                converter
                    .tx
                    .send(PointerEvent::new_down(
                        common,
                        interaction_id,
                        PointerContactData::new_touch(PointerHoverData::new_mouse(), profile),
                    ))
                    .unwrap();
                *state = Interacting {
                    interaction_id,
                    pressed_buttons: PointerButtons::TOUCH_CONTACT,
                    last_position: physical_location,
                };
            } else {
                *state = Added {
                    last_position: physical_location,
                };
            }
        }
        Interacting {
            interaction_id,
            last_position,
            ..
        } => match phase {
            Started => {
                converter
                    .tx
                    .send(PointerEvent::new_cancel(
                        PointerEventCommonData {
                            time_stamp,
                            physical_position: last_position.clone(),
                            pointer_kind: PointerDeviceKind::Touch,
                            synthesized: false,
                        },
                        *interaction_id,
                    ))
                    .unwrap();
                if *last_position != physical_location {
                    converter
                        .tx
                        .send(PointerEvent::new_hover(
                            common.clone(),
                            PointerHoverData::new_mouse(),
                        ))
                        .unwrap();
                }
                let new_interaction_id = converter.interaction_id_counter.generate();
                converter
                    .tx
                    .send(PointerEvent::new_down(
                        common,
                        new_interaction_id,
                        PointerContactData::new_touch(PointerHoverData::new_mouse(), profile),
                    ))
                    .unwrap();
                *interaction_id = new_interaction_id;
                *last_position = physical_location;
            }
            Moved => {
                converter
                    .tx
                    .send(PointerEvent::new_move(
                        common,
                        *interaction_id,
                        PointerContactData::new_touch(PointerHoverData::new_mouse(), profile),
                    ))
                    .unwrap();
                *last_position = physical_location;
            }
            Ended | Cancelled => {
                if *last_position != physical_location {
                    converter
                        .tx
                        .send(PointerEvent::new_move(
                            common.clone(),
                            *interaction_id,
                            PointerContactData::new_touch(PointerHoverData::new_mouse(), profile),
                        ))
                        .unwrap();
                }

                converter
                    .tx
                    .send(if phase == Ended {
                        PointerEvent::new_up(common, *interaction_id, PointerHoverData::new_mouse())
                    } else {
                        PointerEvent::new_cancel(common, *interaction_id)
                    })
                    .unwrap();
                *state = Added {
                    last_position: physical_location,
                };
            }
        },
    };
}
