use crate::{
    sys::*,
    utils::{FsParamArg, FsParamError, FsVarParamArrayOwned, fs_create_param_array},
};
use std::{marker::PhantomData, os::raw::c_void, ptr::NonNull};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventId(pub FsEventId);

impl EventId {
    #[inline]
    pub const fn raw(id: FsEventId) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_raw(self) -> FsEventId {
        self.0
    }
}

impl From<FsEventId> for EventId {
    #[inline]
    fn from(value: FsEventId) -> Self {
        Self(value)
    }
}

impl From<u32> for EventId {
    #[inline]
    fn from(value: u32) -> Self {
        Self(FsEventId::try_from(value).expect("event ID does not fit in FsEventId"))
    }
}

impl PartialEq<u32> for EventId {
    #[inline]
    fn eq(&self, other: &u32) -> bool {
        FsEventId::try_from(*other) == Ok(self.0)
    }
}

impl PartialEq<EventId> for u32 {
    #[inline]
    fn eq(&self, other: &EventId) -> bool {
        other == self
    }
}

#[derive(Debug)]
pub enum EventError {
    EmptySubscriptionContext,
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySubscriptionContext => {
                write!(f, "event subscription context pointer was null")
            }
        }
    }
}

impl std::error::Error for EventError {}

pub type EventResult<T> = Result<T, EventError>;

#[derive(Debug, Copy, Clone)]
pub struct EventParamsRef<'a> {
    raw: *const FsVarParamArray,
    _marker: PhantomData<&'a FsVarParamArray>,
}

impl<'a> EventParamsRef<'a> {
    #[inline]
    pub unsafe fn from_raw(raw: *const FsVarParamArray) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn as_raw(self) -> Option<&'a FsVarParamArray> {
        if self.raw.is_null() {
            None
        } else {
            Some(unsafe { &*self.raw })
        }
    }

    #[inline]
    pub fn len(self) -> usize {
        self.as_raw().map(|p| p.size as usize).unwrap_or(0)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get(self, index: usize) -> Option<FsParamArg> {
        use crate::sys::{
            eFsVarParamType_FsVarParamTypeCRC, eFsVarParamType_FsVarParamTypeDouble,
            eFsVarParamType_FsVarParamTypeInteger, eFsVarParamType_FsVarParamTypeString,
        };

        let raw = self.as_raw()?;

        if index >= raw.size as usize || raw.array.is_null() {
            return None;
        }

        let variant = unsafe { &*raw.array.add(index) };

        unsafe {
            match variant.type_ {
                eFsVarParamType_FsVarParamTypeCRC => {
                    Some(FsParamArg::Crc(variant.__bindgen_anon_1.CRCValue))
                }
                eFsVarParamType_FsVarParamTypeString => {
                    Some(FsParamArg::Str(variant.__bindgen_anon_1.stringValue))
                }
                eFsVarParamType_FsVarParamTypeInteger => {
                    Some(FsParamArg::Index(variant.__bindgen_anon_1.intValue))
                }
                eFsVarParamType_FsVarParamTypeDouble => {
                    Some(FsParamArg::Double(variant.__bindgen_anon_1.doubleValue))
                }
                _ => None,
            }
        }
    }

    #[inline]
    pub fn first(self) -> Option<FsParamArg> {
        self.get(0)
    }

    #[inline]
    pub fn first_index(self) -> Option<u32> {
        match self.first()? {
            FsParamArg::Index(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct KeyEvent<'a> {
    pub id: EventId,
    pub params: EventParamsRef<'a>,
}

struct CallbackState {
    cb: Box<dyn for<'a> FnMut(KeyEvent<'a>)>,
}

extern "C" fn events_trampoline(
    event_id: FsEventId,
    params: *mut FsVarParamArray,
    user_param: *mut c_void,
) {
    if user_param.is_null() {
        return;
    }

    let state = unsafe { &mut *(user_param as *mut CallbackState) };

    let event = KeyEvent {
        id: EventId(event_id),
        params: unsafe { EventParamsRef::from_raw(params as *const FsVarParamArray) },
    };

    (state.cb)(event);
}

pub struct Subscription {
    state: NonNull<CallbackState>,
}

impl Subscription {
    pub fn subscribe(cb: impl for<'a> FnMut(KeyEvent<'a>) + 'static) -> Self {
        let state = Box::new(CallbackState { cb: Box::new(cb) });
        let state = unsafe { NonNull::new_unchecked(Box::into_raw(state)) };

        unsafe {
            fsEventsRegisterKeyEventHandler(Some(events_trampoline), state.as_ptr() as *mut c_void);
        }

        Self { state }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        unsafe {
            fsEventsUnregisterKeyEventHandler(
                Some(events_trampoline),
                self.state.as_ptr() as *mut c_void,
            );

            drop(Box::from_raw(self.state.as_ptr()));
        }
    }
}

#[inline]
pub fn subscribe(cb: impl for<'a> FnMut(KeyEvent<'a>) + 'static) -> Subscription {
    Subscription::subscribe(cb)
}

#[inline]
pub fn trigger(event_id: impl Into<EventId>, params: &FsVarParamArrayOwned) {
    unsafe {
        fsEventsTriggerKeyEvent(event_id.into().as_raw(), params.as_raw());
    }
}

#[inline]
pub fn trigger_raw(event_id: impl Into<EventId>, params: FsVarParamArray) {
    unsafe {
        fsEventsTriggerKeyEvent(event_id.into().as_raw(), params);
    }
}

#[inline]
pub fn trigger0(event_id: impl Into<EventId>) -> Result<(), FsParamError> {
    let params = fs_create_param_array("", &[])?;
    trigger(event_id, &params);
    Ok(())
}

#[inline]
pub fn trigger1(event_id: impl Into<EventId>, arg: FsParamArg) -> Result<(), FsParamError> {
    let fmt = match arg {
        FsParamArg::Crc(_) => "c",
        FsParamArg::Str(_) => "s",
        FsParamArg::Index(_) => "i",
        FsParamArg::Double(_) => "f",
    };

    let params = fs_create_param_array(fmt, &[arg])?;
    trigger(event_id, &params);
    Ok(())
}

#[inline]
pub fn trigger_fmt(
    event_id: impl Into<EventId>,
    fmt: &str,
    args: &[FsParamArg],
) -> Result<(), FsParamError> {
    let params = fs_create_param_array(fmt, args)?;
    trigger(event_id, &params);
    Ok(())
}
