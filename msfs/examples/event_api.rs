use infinity_rs::{events, prelude::*, sys::*, utils::FsParamArg};

pub struct EvemtSystem {
    event_sub: Option<events::Subscription>,
}

impl EvemtSystem {
    pub fn new() -> Self {
        Self { event_sub: None }
    }
}

impl Default for EvemtSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for EvemtSystem {
    fn init(&mut self, ctx: &Context, install: &SystemInstall) -> bool {
        self.event_sub = Some(events::subscribe(|event| {
            let event_id = event.id;
            let first_param = event.params.first();

            match first_param {
                Some(FsParamArg::Index(value)) => {
                    let _ = (event_id, value);
                }
                Some(FsParamArg::Double(value)) => {
                    let _ = (event_id, value);
                }
                Some(FsParamArg::Crc(value)) => {
                    let _ = (event_id, value);
                }
                Some(FsParamArg::Str(ptr)) => {
                    let _ = (event_id, ptr);
                }
                None => {
                    let _ = event_id;
                }
            }

            if event_id == KEY_TOGGLE_MASTER_BATTERY {
                let battery_index = event.params.first_index().unwrap_or(1);
            }
        }));

        let _ = events::trigger1(KEY_TOGGLE_MASTER_BATTERY, FsParamArg::Index(1));
        true
    }

    fn update(&mut self, ctx: &Context, dt: f32) -> bool {
        true
    }

    fn kill(&mut self, ctx: &Context) -> bool {
        true
    }
}
