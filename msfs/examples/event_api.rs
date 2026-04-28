use infinity_rs::{events, prelude::*, sys::*, utils::FsParamArg};

pub struct EventSystem {
    event_sub: Option<events::Subscription>,
}

impl EventSystem {
    pub fn new() -> Self {
        Self { event_sub: None }
    }
}

impl Default for EventSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for EventSystem {
    fn init(&mut self, _ctx: &Context, _install: &SystemInstall) -> bool {
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
                let _battery_index = event.params.first_index().unwrap_or(1);
            }
        }));

        let _ = events::trigger1(KEY_TOGGLE_MASTER_BATTERY, FsParamArg::Index(1));
        true
    }

    fn update(&mut self, _ctx: &Context, _dt: f32) -> bool {
        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        true
    }
}
