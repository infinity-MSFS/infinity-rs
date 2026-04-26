use crate::sys::*;

use std::{
    cell::RefCell,
    os::raw::{c_char, c_uint, c_void},
    ptr,
};

#[derive(Debug)]
pub enum FlowError {
    RegistrationFailed,
    UnregisterFailed,
}

impl std::fmt::Display for FlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowError::RegistrationFailed => write!(f, "Flow registration failed"),
            FlowError::UnregisterFailed => write!(f, "Flow unregistration failed"),
        }
    }
}

impl std::error::Error for FlowError {}

pub type FlowResult<T> = Result<T, FlowError>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FlowEvent {
    None,
    FltLoad,
    FltLoaded,
    TeleportStart,
    TeleportDone,
    BackOnTrackStart,
    BackOnTrackDone,
    SkipStart,
    SkipDone,
    BackToMainMenu,
    RTCStart,
    RTCEnd,
    ReplayStart,
    ReplayEnd,
    FlightStart,
    FlightEnd,
    PlaneCrash,
    Unknown(u16),
}

impl FlowEvent {
    #[inline]
    pub fn from_raw(raw: FsFlowEvent) -> Self {
        match raw {
            FsFlowEvent_FsFlowEvent_None => Self::None,
            FsFlowEvent_FsFlowEvent_FltLoad => Self::FltLoad,
            FsFlowEvent_FsFlowEvent_FltLoaded => Self::FltLoaded,
            FsFlowEvent_FsFlowEvent_TeleportStart => Self::TeleportStart,
            FsFlowEvent_FsFlowEvent_TeleportDone => Self::TeleportDone,
            FsFlowEvent_FsFlowEvent_BackOnTrackStart => Self::BackOnTrackStart,
            FsFlowEvent_FsFlowEvent_BackOnTrackDone => Self::BackOnTrackDone,
            FsFlowEvent_FsFlowEvent_SkipStart => Self::SkipStart,
            FsFlowEvent_FsFlowEvent_SkipDone => Self::SkipDone,
            FsFlowEvent_FsFlowEvent_BackToMainMenu => Self::BackToMainMenu,
            FsFlowEvent_FsFlowEvent_RTCStart => Self::RTCStart,
            FsFlowEvent_FsFlowEvent_RTCEnd => Self::RTCEnd,
            FsFlowEvent_FsFlowEvent_ReplayStart => Self::ReplayStart,
            FsFlowEvent_FsFlowEvent_ReplayEnd => Self::ReplayEnd,
            FsFlowEvent_FsFlowEvent_FlightStart => Self::FlightStart,
            FsFlowEvent_FsFlowEvent_FlightEnd => Self::FlightEnd,
            FsFlowEvent_FsFlowEvent_PlaneCrash => Self::PlaneCrash,
            other => Self::Unknown(other),
        }
    }

    #[inline]
    pub fn as_raw(self) -> Option<FsFlowEvent> {
        match self {
            Self::None => Some(FsFlowEvent_FsFlowEvent_None),
            Self::FltLoad => Some(FsFlowEvent_FsFlowEvent_FltLoad),
            Self::FltLoaded => Some(FsFlowEvent_FsFlowEvent_FltLoaded),
            Self::TeleportStart => Some(FsFlowEvent_FsFlowEvent_TeleportStart),
            Self::TeleportDone => Some(FsFlowEvent_FsFlowEvent_TeleportDone),
            Self::BackOnTrackStart => Some(FsFlowEvent_FsFlowEvent_BackOnTrackStart),
            Self::BackOnTrackDone => Some(FsFlowEvent_FsFlowEvent_BackOnTrackDone),
            Self::SkipStart => Some(FsFlowEvent_FsFlowEvent_SkipStart),
            Self::SkipDone => Some(FsFlowEvent_FsFlowEvent_SkipDone),
            Self::BackToMainMenu => Some(FsFlowEvent_FsFlowEvent_BackToMainMenu),
            Self::RTCStart => Some(FsFlowEvent_FsFlowEvent_RTCStart),
            Self::RTCEnd => Some(FsFlowEvent_FsFlowEvent_RTCEnd),
            Self::ReplayStart => Some(FsFlowEvent_FsFlowEvent_ReplayStart),
            Self::ReplayEnd => Some(FsFlowEvent_FsFlowEvent_ReplayEnd),
            Self::FlightStart => Some(FsFlowEvent_FsFlowEvent_FlightStart),
            Self::FlightEnd => Some(FsFlowEvent_FsFlowEvent_FlightEnd),
            Self::PlaneCrash => Some(FsFlowEvent_FsFlowEvent_PlaneCrash),
            Self::Unknown(_) => None,
        }
    }

    #[inline]
    pub fn is_pause_boundary(self) -> bool {
        matches!(
            self,
            Self::TeleportStart
                | Self::BackOnTrackStart
                | Self::SkipStart
                | Self::RTCStart
                | Self::ReplayStart
        )
    }

    #[inline]
    pub fn is_resume_boundary(self) -> bool {
        matches!(
            self,
            Self::TeleportDone
                | Self::BackOnTrackDone
                | Self::SkipDone
                | Self::RTCEnd
                | Self::ReplayEnd
        )
    }

    #[inline]
    pub fn is_shutdown_boundary(self) -> bool {
        matches!(self, Self::BackToMainMenu | Self::FlightEnd)
    }
}

#[derive(Debug, Clone)]
pub struct FlowMessage {
    pub event: FlowEvent,
    pub payload: Vec<u8>,
}

impl FlowMessage {
    #[inline]
    pub fn payload_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.payload).ok()
    }
}

type FlowHandler = Box<dyn FnMut(FlowMessage)>;

struct HandlerSlot {
    id: u64,
    handler: FlowHandler,
}

#[derive(Default)]
struct FlowRegistry {
    registered: bool,
    next_id: u64,
    handlers: Vec<HandlerSlot>,
}

thread_local! {
    static REGISTRY: RefCell<FlowRegistry> = RefCell::new(FlowRegistry::default());
}

extern "C" fn flow_trampoline(
    event: FsFlowEvent,
    buf: *const c_char,
    buf_size: c_uint,
    ctx: *mut c_void,
) {
    let payload = if buf.is_null() || buf_size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buf as *const u8, buf_size as usize).to_vec() }
    };

    let msg = FlowMessage {
        event: FlowEvent::from_raw(event),
        payload,
    };

    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();

        for slot in &mut registry.handlers {
            (slot.handler)(msg.clone());
        }
    });
}

pub struct FlowSubscription {
    id: u64,
}

impl FlowSubscription {
    pub fn register(handler: impl FnMut(FlowMessage) + 'static) -> FlowResult<Self> {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();

            if !registry.registered {
                let ok = unsafe { fsFlowRegister(Some(flow_trampoline), ptr::null_mut()) };

                if !ok {
                    return Err(FlowError::RegistrationFailed);
                }

                registry.registered = true;
            }

            let id = registry.next_id;
            registry.next_id = registry.next_id.wrapping_add(1);

            registry.handlers.push(HandlerSlot {
                id,
                handler: Box::new(handler),
            });

            Ok(Self { id })
        })
    }
}

impl Drop for FlowSubscription {
    fn drop(&mut self) {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();

            registry.handlers.retain(|s| s.id != self.id);

            if registry.handlers.is_empty() && registry.registered {
                let ok = unsafe { fsFlowUnregister(Some(flow_trampoline)) };

                registry.registered = false;

                eprintln!("Failed to unregister flow handler");
                let _ = ok;
            }
        })
    }
}

pub fn subscribe(handler: impl FnMut(FlowMessage) + 'static) -> FlowResult<FlowSubscription> {
    FlowSubscription::register(handler)
}

pub fn unregister_all() -> FlowResult<()> {
    let ok = unsafe { fsFlowUnregisterAll() };

    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.registered = false;
        registry.handlers.clear();
    });

    if ok {
        Ok(())
    } else {
        Err(FlowError::UnregisterFailed)
    }
}
