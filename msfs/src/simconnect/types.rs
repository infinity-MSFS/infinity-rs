//! Strongly-typed ID newtypes, enum aliases and bitflags for SimConnect.

use crate::sys;

macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident, $raw:ty) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(pub $raw);

        impl $name {
            #[inline]
            pub const fn new(raw: $raw) -> Self { Self(raw) }
            #[inline]
            pub const fn raw(self) -> $raw { self.0 }
        }

        impl From<$raw> for $name {
            #[inline]
            fn from(v: $raw) -> Self { Self(v) }
        }

        impl From<$name> for $raw {
            #[inline]
            fn from(v: $name) -> Self { v.0 }
        }
    };
}

id_newtype!(
    /// Identifies a registered client event.
    ClientEventId, sys::SIMCONNECT_CLIENT_EVENT_ID);
id_newtype!(
    /// Identifies a notification group used to receive client events.
    NotificationGroupId, sys::SIMCONNECT_NOTIFICATION_GROUP_ID);
id_newtype!(
    /// Identifies an input group used for keyboard/joystick input mapping.
    InputGroupId, sys::SIMCONNECT_INPUT_GROUP_ID);
id_newtype!(
    /// Identifies a data definition (a set of `(variable, units, type)` tuples).
    DataDefinitionId, sys::SIMCONNECT_DATA_DEFINITION_ID);
id_newtype!(
    /// Identifies a request for data, used to correlate replies on the dispatch queue.
    DataRequestId, sys::SIMCONNECT_DATA_REQUEST_ID);
id_newtype!(
    /// Identifies a piece of named client data shared between SimConnect clients.
    ClientDataId, sys::SIMCONNECT_CLIENT_DATA_ID);
id_newtype!(
    /// Identifies a definition describing the layout of a `ClientData` blob.
    ClientDataDefinitionId, sys::SIMCONNECT_CLIENT_DATA_DEFINITION_ID);

/// Identifies a sim object (aircraft, vehicle, etc.).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ObjectId(pub sys::SIMCONNECT_OBJECT_ID);

impl ObjectId {
    /// The user aircraft.
    pub const USER: Self = Self(sys::SIMCONNECT_OBJECT_ID_USER_AIRCRAFT);
    /// The user avatar.
    pub const USER_AVATAR: Self = Self(sys::SIMCONNECT_OBJECT_ID_USER_AVATAR);
    /// The user's currently controlled aircraft or avatar.
    pub const USER_CURRENT: Self = Self(sys::SIMCONNECT_OBJECT_ID_USER_CURRENT);

    #[inline]
    pub const fn new(raw: sys::SIMCONNECT_OBJECT_ID) -> Self { Self(raw) }
    #[inline]
    pub const fn raw(self) -> sys::SIMCONNECT_OBJECT_ID { self.0 }
}

impl From<sys::SIMCONNECT_OBJECT_ID> for ObjectId {
    #[inline]
    fn from(v: sys::SIMCONNECT_OBJECT_ID) -> Self { Self(v) }
}

// ---- Re-exported rustified enums ------------------------------------------------

pub use sys::{
    SIMCONNECT_CLIENT_DATA_PERIOD as ClientDataPeriod, SIMCONNECT_DATATYPE as DataType,
    SIMCONNECT_EXCEPTION as Exception, SIMCONNECT_FACILITY_DATA_TYPE as FacilityDataType,
    SIMCONNECT_FACILITY_LIST_TYPE as FacilityListType, SIMCONNECT_FLOW_EVENT as FlowEvent,
    SIMCONNECT_INPUT_EVENT_TYPE as InputEventType, SIMCONNECT_MISSION_END as MissionEnd,
    SIMCONNECT_PERIOD as Period, SIMCONNECT_SIMOBJECT_TYPE as SimObjectType,
    SIMCONNECT_STATE as EventState, SIMCONNECT_TEXT_RESULT as TextResult,
    SIMCONNECT_TEXT_TYPE as TextType, SIMCONNECT_WEATHER_MODE as WeatherMode,
};

// ---- Bitflags wrapping the C `SIMCONNECT_ENUM_FLAGS` typedefs -------------------

bitflags::bitflags! {
    /// Flags for `transmit_client_event` / `transmit_client_event_ex1`.
    pub struct EventFlag: u32 {
        const FAST_REPEAT_TIMER   = sys::SIMCONNECT_EVENT_FLAG_FAST_REPEAT_TIMER;
        const SLOW_REPEAT_TIMER   = sys::SIMCONNECT_EVENT_FLAG_SLOW_REPEAT_TIMER;
        const GROUPID_IS_PRIORITY = sys::SIMCONNECT_EVENT_FLAG_GROUPID_IS_PRIORITY;
    }

    /// Flags for `request_data_on_sim_object`.
    pub struct DataRequestFlag: u32 {
        const CHANGED = sys::SIMCONNECT_DATA_REQUEST_FLAG_CHANGED;
        const TAGGED  = sys::SIMCONNECT_DATA_REQUEST_FLAG_TAGGED;
    }

    /// Flags for `set_data_on_sim_object`.
    pub struct DataSetFlag: u32 {
        const TAGGED = sys::SIMCONNECT_DATA_SET_FLAG_TAGGED;
    }

    /// Flags for `create_client_data`.
    pub struct CreateClientDataFlag: u32 {
        const READ_ONLY = sys::SIMCONNECT_CREATE_CLIENT_DATA_FLAG_READ_ONLY;
    }

    /// Flags for `request_client_data`.
    pub struct ClientDataRequestFlag: u32 {
        const CHANGED = sys::SIMCONNECT_CLIENT_DATA_REQUEST_FLAG_CHANGED;
        const TAGGED  = sys::SIMCONNECT_CLIENT_DATA_REQUEST_FLAG_TAGGED;
    }

    /// Flags for `set_client_data`.
    pub struct ClientDataSetFlag: u32 {
        const TAGGED = sys::SIMCONNECT_CLIENT_DATA_SET_FLAG_TAGGED;
    }
}

// ---- Notification-group priority sentinels --------------------------------------

/// Pre-defined notification-group priorities. Higher numbers are lower priority.
#[allow(non_snake_case)]
pub mod GroupPriority {
    use crate::sys;
    pub const HIGHEST: u32 = sys::SIMCONNECT_GROUP_PRIORITY_HIGHEST;
    pub const HIGHEST_MASKABLE: u32 = sys::SIMCONNECT_GROUP_PRIORITY_HIGHEST_MASKABLE;
    pub const STANDARD: u32 = sys::SIMCONNECT_GROUP_PRIORITY_STANDARD;
    pub const DEFAULT: u32 = sys::SIMCONNECT_GROUP_PRIORITY_DEFAULT;
    pub const LOWEST: u32 = sys::SIMCONNECT_GROUP_PRIORITY_LOWEST;
}

/// `dwSizeOrType` constants for `add_to_client_data_definition`.
#[allow(non_snake_case)]
pub mod ClientDataType {
    use crate::sys;
    pub const INT8: u32 = sys::SIMCONNECT_CLIENTDATATYPE_INT8;
    pub const INT16: u32 = sys::SIMCONNECT_CLIENTDATATYPE_INT16;
    pub const INT32: u32 = sys::SIMCONNECT_CLIENTDATATYPE_INT32;
    pub const INT64: u32 = sys::SIMCONNECT_CLIENTDATATYPE_INT64;
    pub const FLOAT32: u32 = sys::SIMCONNECT_CLIENTDATATYPE_FLOAT32;
    pub const FLOAT64: u32 = sys::SIMCONNECT_CLIENTDATATYPE_FLOAT64;
    /// Compute the offset automatically.
    pub const OFFSET_AUTO: u32 = sys::SIMCONNECT_CLIENTDATAOFFSET_AUTO;
}

/// Use as the `unused` placeholder where the C API expects `SIMCONNECT_UNUSED`.
pub const UNUSED: u32 = sys::SIMCONNECT_UNUSED;

/// Maximum METAR string length.
pub const MAX_METAR_LENGTH: u32 = sys::MAX_METAR_LENGTH;

/// Re-export of the raw C structs that downstream users sometimes need
/// (POD payloads carried by RECV variants and used for `set_data_on_sim_object`).
pub use sys::{
    SIMCONNECT_DATA_FACILITY_AIRPORT as FacilityAirport,
    SIMCONNECT_DATA_FACILITY_NDB as FacilityNdb,
    SIMCONNECT_DATA_FACILITY_VOR as FacilityVor,
    SIMCONNECT_DATA_FACILITY_WAYPOINT as FacilityWaypoint,
    SIMCONNECT_DATA_INITPOSITION as InitPosition, SIMCONNECT_DATA_LATLONALT as LatLonAlt,
    SIMCONNECT_DATA_MARKERSTATE as MarkerState, SIMCONNECT_DATA_PBH as PitchBankHeading,
    SIMCONNECT_DATA_RACE_RESULT as RaceResult, SIMCONNECT_DATA_WAYPOINT as Waypoint,
    SIMCONNECT_DATA_XYZ as Xyz, SIMCONNECT_FACILITY_MINIMAL as FacilityMinimal,
    SIMCONNECT_ICAO as Icao, SIMCONNECT_INPUT_EVENT_DESCRIPTOR as InputEventDescriptor,
    SIMCONNECT_JETWAY_DATA as JetwayData,
};
