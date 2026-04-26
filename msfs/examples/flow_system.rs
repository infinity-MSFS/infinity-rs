use infinity_rs::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct FlowState {
    paused: bool,
    needs_resync: bool,
}

pub struct FlowSystem {
    flow: Option<FlowSubscription>,
    flow_state: Rc<RefCell<FlowState>>,
}

impl FlowSystem {
    pub fn new() -> Self {
        Self {
            flow: None,
            flow_state: Rc::new(RefCell::new(FlowState::default())),
        }
    }
}

impl System for FlowSystem {
    fn init(&mut self, ctx: &Context, install: &SystemInstall) -> bool {
        let state = self.flow_state.clone();

        self.flow = flow_subscribe(move |msg| {
            let mut state = state.borrow_mut();

            if msg.event.is_pause_boundary() {
                state.paused = true;
            }

            if msg.event.is_resume_boundary() {
                state.paused = false;
                state.needs_resync = true;
            }

            if msg.event.is_shutdown_boundary() {
                state.paused = true;
            }
        })
        .ok();
        true
    }

    fn update(&mut self, ctx: &Context, dt: f32) -> bool {
        let mut flow = self.flow_state.borrow_mut();

        if flow.needs_resync {
            flow.needs_resync = false;
            // resync system
        }

        if flow.paused {
            return true;
        }

        drop(flow);

        true
    }

    fn kill(&mut self, ctx: &Context) -> bool {
        self.flow = None;
        true
    }
}
