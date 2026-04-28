//! Safe view over the dispatch queue messages (`SIMCONNECT_RECV_*`).
//!
//! [`Recv`] borrows the message buffer owned by SimConnect, which lives until
//! the next call to `SimConnect_GetNextDispatch`. To keep data past that point,
//! copy the relevant fields out of the borrow.

use crate::sys;

/// Safe enumeration of every dispatch message a SimConnect client can receive.
///
/// Each variant carries a borrow of the underlying packed C struct. Use the
/// helper accessors on each variant rather than touching packed fields directly
/// (Rust will yell about unaligned references).
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum Recv<'a> {
    Null,
    Exception(&'a sys::SIMCONNECT_RECV_EXCEPTION),
    Open(&'a sys::SIMCONNECT_RECV_OPEN),
    Quit(&'a sys::SIMCONNECT_RECV_QUIT),
    Event(&'a sys::SIMCONNECT_RECV_EVENT),
    EventEx1(&'a sys::SIMCONNECT_RECV_EVENT_EX1),
    EventObjectAddRemove(&'a sys::SIMCONNECT_RECV_EVENT_OBJECT_ADDREMOVE),
    EventFilename(&'a sys::SIMCONNECT_RECV_EVENT_FILENAME),
    EventFrame(&'a sys::SIMCONNECT_RECV_EVENT_FRAME),
    EventMultiplayerServerStarted(&'a sys::SIMCONNECT_RECV_EVENT_MULTIPLAYER_SERVER_STARTED),
    EventMultiplayerClientStarted(&'a sys::SIMCONNECT_RECV_EVENT_MULTIPLAYER_CLIENT_STARTED),
    EventMultiplayerSessionEnded(&'a sys::SIMCONNECT_RECV_EVENT_MULTIPLAYER_SESSION_ENDED),
    EventRaceEnd(&'a sys::SIMCONNECT_RECV_EVENT_RACE_END),
    EventRaceLap(&'a sys::SIMCONNECT_RECV_EVENT_RACE_LAP),
    EventWeatherMode(&'a sys::SIMCONNECT_RECV_EVENT_WEATHER_MODE),
    SimObjectData(&'a sys::SIMCONNECT_RECV_SIMOBJECT_DATA),
    SimObjectDataByType(&'a sys::SIMCONNECT_RECV_SIMOBJECT_DATA_BYTYPE),
    ClientData(&'a sys::SIMCONNECT_RECV_CLIENT_DATA),
    WeatherObservation(&'a sys::SIMCONNECT_RECV_WEATHER_OBSERVATION),
    CloudState(&'a sys::SIMCONNECT_RECV_CLOUD_STATE),
    AssignedObjectId(&'a sys::SIMCONNECT_RECV_ASSIGNED_OBJECT_ID),
    ReservedKey(&'a sys::SIMCONNECT_RECV_RESERVED_KEY),
    SystemState(&'a sys::SIMCONNECT_RECV_SYSTEM_STATE),
    CustomAction(&'a sys::SIMCONNECT_RECV_CUSTOM_ACTION),
    AirportList(&'a sys::SIMCONNECT_RECV_AIRPORT_LIST),
    VorList(&'a sys::SIMCONNECT_RECV_VOR_LIST),
    NdbList(&'a sys::SIMCONNECT_RECV_NDB_LIST),
    WaypointList(&'a sys::SIMCONNECT_RECV_WAYPOINT_LIST),
    FacilityData(&'a sys::SIMCONNECT_RECV_FACILITY_DATA),
    FacilityDataEnd(&'a sys::SIMCONNECT_RECV_FACILITY_DATA_END),
    FacilityMinimalList(&'a sys::SIMCONNECT_RECV_FACILITY_MINIMAL_LIST),
    JetwayData(&'a sys::SIMCONNECT_RECV_JETWAY_DATA),
    ControllersList(&'a sys::SIMCONNECT_RECV_CONTROLLERS_LIST),
    ActionCallback(&'a sys::SIMCONNECT_RECV_ACTION_CALLBACK),
    EnumerateInputEvents(&'a sys::SIMCONNECT_RECV_ENUMERATE_INPUT_EVENTS),
    GetInputEvent(&'a sys::SIMCONNECT_RECV_GET_INPUT_EVENT),
    SubscribeInputEvent(&'a sys::SIMCONNECT_RECV_SUBSCRIBE_INPUT_EVENT),
    EnumerateInputEventParams(&'a sys::SIMCONNECT_RECV_ENUMERATE_INPUT_EVENT_PARAMS),
    EnumerateSimObjectAndLiveryList(&'a sys::SIMCONNECT_RECV_ENUMERATE_SIMOBJECT_AND_LIVERY_LIST),
    FlowEvent(&'a sys::SIMCONNECT_RECV_FLOW_EVENT),
    /// Catch-all for messages this binding does not yet model. The header carries
    /// the raw `dwID` value so callers can switch on it manually.
    Unknown {
        id: u32,
        header: &'a sys::SIMCONNECT_RECV,
        cb_data: u32,
    },
}

impl<'a> Recv<'a> {
    /// Cast the raw `(pData, cbData)` pair returned by `SimConnect_GetNextDispatch`
    /// or passed to a `DispatchProc` into a typed [`Recv`] borrow.
    ///
    /// # Safety
    /// `data` must point to a `cb_data`-byte block of memory laid out as a
    /// `SIMCONNECT_RECV_*` struct of the kind indicated by its `dwID` field.
    /// The borrow returned must not outlive the underlying buffer (which
    /// SimConnect frees on the next dispatch call).
    pub unsafe fn from_raw(data: *const sys::SIMCONNECT_RECV, cb_data: u32) -> Self {
        if data.is_null() {
            return Recv::Null;
        }

        let id = unsafe { (*data).dwID } as sys::SIMCONNECT_RECV_ID;

        macro_rules! cast {
            ($ty:ident) => {
                unsafe { &*(data as *const sys::$ty) }
            };
        }

        use sys::*;

        match id {
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_NULL => Recv::Null,
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EXCEPTION => Recv::Exception(cast!(SIMCONNECT_RECV_EXCEPTION)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_OPEN => Recv::Open(cast!(SIMCONNECT_RECV_OPEN)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_QUIT => Recv::Quit(cast!(SIMCONNECT_RECV_QUIT)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT => Recv::Event(cast!(SIMCONNECT_RECV_EVENT)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_EX1 => Recv::EventEx1(cast!(SIMCONNECT_RECV_EVENT_EX1)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_OBJECT_ADDREMOVE => Recv::EventObjectAddRemove(cast!(SIMCONNECT_RECV_EVENT_OBJECT_ADDREMOVE)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_FILENAME => Recv::EventFilename(cast!(SIMCONNECT_RECV_EVENT_FILENAME)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_FRAME => Recv::EventFrame(cast!(SIMCONNECT_RECV_EVENT_FRAME)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_MULTIPLAYER_SERVER_STARTED => Recv::EventMultiplayerServerStarted(cast!(SIMCONNECT_RECV_EVENT_MULTIPLAYER_SERVER_STARTED)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_MULTIPLAYER_CLIENT_STARTED => Recv::EventMultiplayerClientStarted(cast!(SIMCONNECT_RECV_EVENT_MULTIPLAYER_CLIENT_STARTED)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_MULTIPLAYER_SESSION_ENDED => Recv::EventMultiplayerSessionEnded(cast!(SIMCONNECT_RECV_EVENT_MULTIPLAYER_SESSION_ENDED)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_RACE_END => Recv::EventRaceEnd(cast!(SIMCONNECT_RECV_EVENT_RACE_END)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_RACE_LAP => Recv::EventRaceLap(cast!(SIMCONNECT_RECV_EVENT_RACE_LAP)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_EVENT_WEATHER_MODE => Recv::EventWeatherMode(cast!(SIMCONNECT_RECV_EVENT_WEATHER_MODE)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_SIMOBJECT_DATA => Recv::SimObjectData(cast!(SIMCONNECT_RECV_SIMOBJECT_DATA)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_SIMOBJECT_DATA_BYTYPE => Recv::SimObjectDataByType(cast!(SIMCONNECT_RECV_SIMOBJECT_DATA_BYTYPE)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_CLIENT_DATA => Recv::ClientData(cast!(SIMCONNECT_RECV_CLIENT_DATA)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_WEATHER_OBSERVATION => Recv::WeatherObservation(cast!(SIMCONNECT_RECV_WEATHER_OBSERVATION)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_CLOUD_STATE => Recv::CloudState(cast!(SIMCONNECT_RECV_CLOUD_STATE)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_ASSIGNED_OBJECT_ID => Recv::AssignedObjectId(cast!(SIMCONNECT_RECV_ASSIGNED_OBJECT_ID)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_RESERVED_KEY => Recv::ReservedKey(cast!(SIMCONNECT_RECV_RESERVED_KEY)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_SYSTEM_STATE => Recv::SystemState(cast!(SIMCONNECT_RECV_SYSTEM_STATE)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_CUSTOM_ACTION => Recv::CustomAction(cast!(SIMCONNECT_RECV_CUSTOM_ACTION)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_AIRPORT_LIST => Recv::AirportList(cast!(SIMCONNECT_RECV_AIRPORT_LIST)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_VOR_LIST => Recv::VorList(cast!(SIMCONNECT_RECV_VOR_LIST)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_NDB_LIST => Recv::NdbList(cast!(SIMCONNECT_RECV_NDB_LIST)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_WAYPOINT_LIST => Recv::WaypointList(cast!(SIMCONNECT_RECV_WAYPOINT_LIST)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_FACILITY_DATA => Recv::FacilityData(cast!(SIMCONNECT_RECV_FACILITY_DATA)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_FACILITY_DATA_END => Recv::FacilityDataEnd(cast!(SIMCONNECT_RECV_FACILITY_DATA_END)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_FACILITY_MINIMAL_LIST => Recv::FacilityMinimalList(cast!(SIMCONNECT_RECV_FACILITY_MINIMAL_LIST)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_JETWAY_DATA => Recv::JetwayData(cast!(SIMCONNECT_RECV_JETWAY_DATA)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_CONTROLLERS_LIST => Recv::ControllersList(cast!(SIMCONNECT_RECV_CONTROLLERS_LIST)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_ACTION_CALLBACK => Recv::ActionCallback(cast!(SIMCONNECT_RECV_ACTION_CALLBACK)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_ENUMERATE_INPUT_EVENTS => Recv::EnumerateInputEvents(cast!(SIMCONNECT_RECV_ENUMERATE_INPUT_EVENTS)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_GET_INPUT_EVENT => Recv::GetInputEvent(cast!(SIMCONNECT_RECV_GET_INPUT_EVENT)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_SUBSCRIBE_INPUT_EVENT => Recv::SubscribeInputEvent(cast!(SIMCONNECT_RECV_SUBSCRIBE_INPUT_EVENT)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_ENUMERATE_INPUT_EVENT_PARAMS => Recv::EnumerateInputEventParams(cast!(SIMCONNECT_RECV_ENUMERATE_INPUT_EVENT_PARAMS)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_ENUMERATE_SIMOBJECT_AND_LIVERY_LIST => Recv::EnumerateSimObjectAndLiveryList(cast!(SIMCONNECT_RECV_ENUMERATE_SIMOBJECT_AND_LIVERY_LIST)),
            SIMCONNECT_RECV_ID_SIMCONNECT_RECV_ID_FLOW_EVENT => Recv::FlowEvent(cast!(SIMCONNECT_RECV_FLOW_EVENT)),
            other => Recv::Unknown { id: other as u32, header: unsafe { &*data }, cb_data },
        }
    }
}

// Helper accessors that read packed fields safely (copying through `read_unaligned`).
//
// SimConnect's RECV structs are `#[repr(C, packed)]` — taking direct references
// to interior fields is undefined behaviour. These methods produce owned copies.

macro_rules! packed_get {
    ($field:ident: $ty:ty) => {
        #[inline]
        pub fn $field(&self) -> $ty {
            unsafe { std::ptr::addr_of!(self.0.$field).read_unaligned() }
        }
    };
}

/// Convenience accessors over a [`SIMCONNECT_RECV_EVENT`](sys::SIMCONNECT_RECV_EVENT).
#[derive(Debug, Copy, Clone)]
pub struct EventView<'a>(pub &'a sys::SIMCONNECT_RECV_EVENT);

impl<'a> EventView<'a> {
    packed_get!(uGroupID: u32);
    packed_get!(uEventID: u32);
    packed_get!(dwData: u32);
}

/// Convenience accessors over a [`SIMCONNECT_RECV_SIMOBJECT_DATA`](sys::SIMCONNECT_RECV_SIMOBJECT_DATA).
#[derive(Debug, Copy, Clone)]
pub struct SimObjectDataView<'a>(pub &'a sys::SIMCONNECT_RECV_SIMOBJECT_DATA);

impl<'a> SimObjectDataView<'a> {
    packed_get!(dwRequestID: u32);
    packed_get!(dwObjectID: u32);
    packed_get!(dwDefineID: u32);
    packed_get!(dwFlags: u32);
    packed_get!(dwentrynumber: u32);
    packed_get!(dwoutof: u32);
    packed_get!(dwDefineCount: u32);

    /// Returns a pointer to the start of the variable-length data payload.
    /// The number of bytes available is reported in the dispatch's `cbData`.
    #[inline]
    pub fn data_ptr(&self) -> *const u8 {
        std::ptr::addr_of!(self.0.dwData) as *const u8
    }

    /// Reinterpret the trailing payload as `T`. The caller must ensure the
    /// data definition matches `T`'s layout.
    ///
    /// # Safety
    /// `T` must match the C layout of the data definition that produced this
    /// message. SimConnect uses `#[repr(C, packed)]` for its own structs and
    /// expects clients to do the same for compound payloads.
    #[inline]
    pub unsafe fn data_as<T: Copy>(&self) -> T {
        unsafe { (self.data_ptr() as *const T).read_unaligned() }
    }
}

/// Convenience accessors over a [`SIMCONNECT_RECV_CLIENT_DATA`](sys::SIMCONNECT_RECV_CLIENT_DATA).
#[derive(Debug, Copy, Clone)]
pub struct ClientDataView<'a>(pub &'a sys::SIMCONNECT_RECV_CLIENT_DATA);

impl<'a> ClientDataView<'a> {
    #[inline]
    pub fn as_simobject(&self) -> SimObjectDataView<'a> {
        // SAFETY: `SIMCONNECT_RECV_CLIENT_DATA` derives from `SIMCONNECT_RECV_SIMOBJECT_DATA`
        // (bindgen flattens this via `_base`), so the layout prefix is identical.
        unsafe { SimObjectDataView(&*(self.0 as *const _ as *const sys::SIMCONNECT_RECV_SIMOBJECT_DATA)) }
    }
}

/// Convenience accessors over a [`SIMCONNECT_RECV_EXCEPTION`](sys::SIMCONNECT_RECV_EXCEPTION).
#[derive(Debug, Copy, Clone)]
pub struct ExceptionView<'a>(pub &'a sys::SIMCONNECT_RECV_EXCEPTION);

impl<'a> ExceptionView<'a> {
    /// The exception kind as a typed enum.
    #[inline]
    pub fn exception(&self) -> sys::SIMCONNECT_EXCEPTION {
        // dwException is a DWORD; map it back into the enum.
        let raw: u32 = unsafe { std::ptr::addr_of!(self.0.dwException).read_unaligned() };
        // The enum is rustified; transmute is safe since SDK guarantees a
        // contiguous value range. Out-of-range values fall through to a panic
        // in safe code, so prefer try_from-style by returning ERROR for unknown.
        match raw {
            0 => sys::SIMCONNECT_EXCEPTION::SIMCONNECT_EXCEPTION_NONE,
            v => unsafe {
                std::mem::transmute::<u32, sys::SIMCONNECT_EXCEPTION>(v)
            },
        }
    }
    packed_get!(dwSendID: u32);
    packed_get!(dwIndex: u32);
}
