//! Safe, idiomatic Rust bindings for the MSFS [SimConnect SDK].
//!
//! Enable the `simconnect` cargo feature to compile this module. The same API
//! surface is available on Windows, Linux, and the WASM gauge target — under
//! WASM, SimConnect symbols are resolved through the host runtime, so no
//! additional link configuration is required.
//!
//! # Quickstart
//!
//! ```no_run
//! # #[cfg(feature = "simconnect")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use infinity_rs::simconnect::{SimConnect, Period, DataType, Recv, DataRequestFlag};
//! use infinity_rs::simconnect::{DataDefinitionId, DataRequestId, ObjectId};
//!
//! let mut sc = SimConnect::open("Demo")?;
//!
//! let def = DataDefinitionId::new(1);
//! sc.add_to_data_definition(def, "PLANE ALTITUDE", "feet", DataType::SIMCONNECT_DATATYPE_FLOAT64, 0.0, infinity_rs::simconnect::UNUSED)?;
//!
//! let req = DataRequestId::new(1);
//! sc.request_data_on_sim_object(req, def, ObjectId::USER, Period::SIMCONNECT_PERIOD_SIM_FRAME, DataRequestFlag::CHANGED, 0, 0, 0)?;
//!
//! while let Some(msg) = sc.next_dispatch()? {
//!     match msg {
//!         Recv::SimObjectData(data) => { /* read out the payload */ }
//!         Recv::Quit(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(()) }
//! # #[cfg(not(feature = "simconnect"))] fn main() {}
//! ```
//!
//! [SimConnect SDK]: https://docs.flightsimulator.com/html/Programming_Tools/SimConnect/SimConnect_API_Reference.htm

use crate::sys;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

mod error;
mod recv;
mod types;

pub use error::{Error, Result};
pub use recv::{
    ClientDataView, EventView, ExceptionView, Recv, SimObjectDataView,
};
pub use types::*;

/// Owned SimConnect connection handle. Closes on drop.
///
/// `SimConnect` is `Send` (the underlying handle can be moved between threads)
/// but **not** `Sync`: calls into SimConnect from multiple threads in parallel
/// are not safe. Wrap in your own synchronisation if you need shared access.
pub struct SimConnect {
    handle: sys::HANDLE,
}

// SimConnect's `HANDLE` is opaque on the SDK side; the API serialises calls per
// handle internally on Windows, but the safe contract here is that only one
// thread touches a `SimConnect` at a time.
unsafe impl Send for SimConnect {}

impl SimConnect {
    /// Open a SimConnect connection with the given client name.
    ///
    /// Equivalent to `SimConnect_Open` with `hWnd = nullptr`, `UserEventWin32 = 0`,
    /// `hEventHandle = nullptr`, `ConfigIndex = SIMCONNECT_OPEN_CONFIGINDEX_LOCAL`.
    pub fn open(name: &str) -> Result<Self> {
        Self::open_with_config(name, sys::SIMCONNECT_OPEN_CONFIGINDEX_LOCAL)
    }

    /// Open with a specific entry from `SimConnect.cfg`.
    pub fn open_with_config(name: &str, config_index: u32) -> Result<Self> {
        let c_name = CString::new(name)?;
        let mut handle: sys::HANDLE = 0;
        let hr = unsafe {
            sys::SimConnect_Open(
                &mut handle,
                c_name.as_ptr(),
                ptr::null_mut(),
                0,
                0,
                config_index,
            )
        };
        if hr < 0 || handle == 0 {
            return Err(Error::OpenFailed(hr as i32));
        }
        Ok(SimConnect { handle })
    }

    /// Borrow the raw `HANDLE`. Prefer the typed methods on this struct.
    #[inline]
    pub fn raw_handle(&self) -> sys::HANDLE {
        self.handle
    }

    /// Close the connection explicitly. Otherwise the handle is closed on drop.
    pub fn close(mut self) -> Result<()> {
        let hr = unsafe { sys::SimConnect_Close(self.handle) };
        self.handle = 0;
        Error::from_hresult(hr)
    }

    // -- Dispatch -----------------------------------------------------------

    /// Pull the next pending message. Returns `Ok(None)` when the queue is
    /// empty (HRESULT `E_FAIL`).
    pub fn next_dispatch(&mut self) -> Result<Option<Recv<'_>>> {
        let mut p_data: *mut sys::SIMCONNECT_RECV = ptr::null_mut();
        let mut cb: sys::DWORD = 0;
        let hr = unsafe { sys::SimConnect_GetNextDispatch(self.handle, &mut p_data, &mut cb) };
        if hr < 0 {
            // Empty queue is reported as E_FAIL on the SDK side. Treat as None.
            return Ok(None);
        }
        if p_data.is_null() {
            return Ok(None);
        }
        Ok(Some(unsafe { Recv::from_raw(p_data, cb) }))
    }

    /// Drain pending messages, invoking `f` for each. Stops at the first
    /// empty-queue or error.
    pub fn dispatch<F: FnMut(Recv<'_>)>(&mut self, mut f: F) -> Result<()> {
        while let Some(msg) = self.next_dispatch()? {
            f(msg);
        }
        Ok(())
    }

    /// Hook a C-style dispatch callback that SimConnect drives internally.
    /// The `proc` is called with raw `(SIMCONNECT_RECV*, cbData, context)`.
    ///
    /// # Safety
    /// `proc` must respect SimConnect's calling convention and not panic across
    /// the FFI boundary. `context` must remain valid for the lifetime of the
    /// connection.
    pub unsafe fn call_dispatch_raw(
        &mut self,
        proc_: sys::DispatchProc,
        context: *mut std::os::raw::c_void,
    ) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_CallDispatch(self.handle, proc_, context) })
    }

    // -- Misc ---------------------------------------------------------------

    /// Returns the packet ID assigned to the most recent outgoing call.
    pub fn last_sent_packet_id(&self) -> Result<u32> {
        let mut id: sys::DWORD = 0;
        let hr = unsafe { sys::SimConnect_GetLastSentPacketID(self.handle, &mut id) };
        Error::from_hresult(hr).map(|_| id)
    }

    /// Request response-time samples (the `nCount` slowest packet round-trips).
    pub fn request_response_times(&mut self, samples: &mut [f32]) -> Result<()> {
        let n = samples.len() as sys::DWORD;
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestResponseTimes(self.handle, n, samples.as_mut_ptr())
        })
    }

    /// Insert a length-prefixed string into a `STRINGV` buffer.
    pub fn insert_string(
        &self,
        dest: &mut [u8],
        source: &str,
    ) -> Result<(usize, usize)> {
        let src = CString::new(source)?;
        let mut p_end: *mut std::os::raw::c_void = ptr::null_mut();
        let mut cb_string: sys::DWORD = 0;
        let hr = unsafe {
            sys::SimConnect_InsertString(
                dest.as_mut_ptr() as *mut c_char,
                dest.len() as sys::DWORD,
                &mut p_end,
                &mut cb_string,
                src.as_ptr(),
            )
        };
        Error::from_hresult(hr).map(|_| {
            let end_offset = if p_end.is_null() {
                0
            } else {
                p_end as usize - dest.as_ptr() as usize
            };
            (end_offset, cb_string as usize)
        })
    }

    // -- Client events ------------------------------------------------------

    pub fn map_client_event_to_sim_event(
        &mut self,
        event: ClientEventId,
        name: &str,
    ) -> Result<()> {
        let c_name = CString::new(name)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_MapClientEventToSimEvent(self.handle, event.raw(), c_name.as_ptr())
        })
    }

    pub fn transmit_client_event(
        &mut self,
        object: ObjectId,
        event: ClientEventId,
        data: u32,
        group: NotificationGroupId,
        flags: EventFlag,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_TransmitClientEvent(
                self.handle,
                object.raw(),
                event.raw(),
                data,
                group.raw(),
                flags.bits(),
            )
        })
    }

    pub fn transmit_client_event_ex1(
        &mut self,
        object: ObjectId,
        event: ClientEventId,
        group: NotificationGroupId,
        flags: EventFlag,
        data: [u32; 5],
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_TransmitClientEvent_EX1(
                self.handle,
                object.raw(),
                event.raw(),
                group.raw(),
                flags.bits(),
                data[0],
                data[1],
                data[2],
                data[3],
                data[4],
            )
        })
    }

    pub fn set_system_event_state(
        &mut self,
        event: ClientEventId,
        state: EventState,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SetSystemEventState(self.handle, event.raw(), state)
        })
    }

    pub fn add_client_event_to_notification_group(
        &mut self,
        group: NotificationGroupId,
        event: ClientEventId,
        maskable: bool,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_AddClientEventToNotificationGroup(
                self.handle,
                group.raw(),
                event.raw(),
                maskable as sys::BOOL,
            )
        })
    }

    pub fn remove_client_event(
        &mut self,
        group: NotificationGroupId,
        event: ClientEventId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RemoveClientEvent(self.handle, group.raw(), event.raw())
        })
    }

    pub fn set_notification_group_priority(
        &mut self,
        group: NotificationGroupId,
        priority: u32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SetNotificationGroupPriority(self.handle, group.raw(), priority)
        })
    }

    pub fn clear_notification_group(&mut self, group: NotificationGroupId) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_ClearNotificationGroup(self.handle, group.raw())
        })
    }

    pub fn request_notification_group(
        &mut self,
        group: NotificationGroupId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestNotificationGroup(self.handle, group.raw(), 0, 0)
        })
    }

    pub fn subscribe_to_system_event(
        &mut self,
        event: ClientEventId,
        system_event_name: &str,
    ) -> Result<()> {
        let c_name = CString::new(system_event_name)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_SubscribeToSystemEvent(self.handle, event.raw(), c_name.as_ptr())
        })
    }

    pub fn unsubscribe_from_system_event(&mut self, event: ClientEventId) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_UnsubscribeFromSystemEvent(self.handle, event.raw())
        })
    }

    // -- Data definition / requests ----------------------------------------

    pub fn add_to_data_definition(
        &mut self,
        def: DataDefinitionId,
        datum_name: &str,
        units_name: &str,
        datum_type: DataType,
        epsilon: f32,
        datum_id: u32,
    ) -> Result<()> {
        let n = CString::new(datum_name)?;
        let u = CString::new(units_name)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AddToDataDefinition(
                self.handle,
                def.raw(),
                n.as_ptr(),
                u.as_ptr(),
                datum_type,
                epsilon,
                datum_id,
            )
        })
    }

    pub fn clear_data_definition(&mut self, def: DataDefinitionId) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_ClearDataDefinition(self.handle, def.raw())
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn request_data_on_sim_object(
        &mut self,
        request: DataRequestId,
        def: DataDefinitionId,
        object: ObjectId,
        period: Period,
        flags: DataRequestFlag,
        origin: u32,
        interval: u32,
        limit: u32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestDataOnSimObject(
                self.handle,
                request.raw(),
                def.raw(),
                object.raw(),
                period,
                flags.bits(),
                origin,
                interval,
                limit,
            )
        })
    }

    pub fn request_data_on_sim_object_type(
        &mut self,
        request: DataRequestId,
        def: DataDefinitionId,
        radius_meters: u32,
        ty: SimObjectType,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestDataOnSimObjectType(
                self.handle,
                request.raw(),
                def.raw(),
                radius_meters,
                ty,
            )
        })
    }

    /// Push `data` to the simulator using the schema described by `def`.
    /// The whole slice is sent as a single contiguous block.
    ///
    /// # Safety
    /// `T`'s memory layout must match the registered data definition and use
    /// the SimConnect packing rules (`#[repr(C, packed)]`). Mismatches are
    /// reported asynchronously as `SIMCONNECT_EXCEPTION_DATA_ERROR`.
    pub unsafe fn set_data_on_sim_object<T: Copy>(
        &mut self,
        def: DataDefinitionId,
        object: ObjectId,
        flags: DataSetFlag,
        data: &[T],
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SetDataOnSimObject(
                self.handle,
                def.raw(),
                object.raw(),
                flags.bits(),
                data.len() as sys::DWORD,
                std::mem::size_of::<T>() as sys::DWORD,
                data.as_ptr() as *mut std::os::raw::c_void,
            )
        })
    }

    // -- Input groups -------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn map_input_event_to_client_event(
        &mut self,
        group: InputGroupId,
        input_definition: &str,
        down_event: ClientEventId,
        down_value: u32,
        up_event: Option<ClientEventId>,
        up_value: u32,
        maskable: bool,
    ) -> Result<()> {
        let c_def = CString::new(input_definition)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_MapInputEventToClientEvent(
                self.handle,
                group.raw(),
                c_def.as_ptr(),
                down_event.raw(),
                down_value,
                up_event.map(|e| e.raw()).unwrap_or(sys::SIMCONNECT_UNUSED),
                up_value,
                maskable as sys::BOOL,
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn map_input_event_to_client_event_ex1(
        &mut self,
        group: InputGroupId,
        input_definition: &str,
        down_event: ClientEventId,
        down_value: u32,
        up_event: Option<ClientEventId>,
        up_value: u32,
        maskable: bool,
    ) -> Result<()> {
        let c_def = CString::new(input_definition)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_MapInputEventToClientEvent_EX1(
                self.handle,
                group.raw(),
                c_def.as_ptr(),
                down_event.raw(),
                down_value,
                up_event.map(|e| e.raw()).unwrap_or(sys::SIMCONNECT_UNUSED),
                up_value,
                maskable as sys::BOOL,
            )
        })
    }

    pub fn set_input_group_priority(
        &mut self,
        group: InputGroupId,
        priority: u32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SetInputGroupPriority(self.handle, group.raw(), priority)
        })
    }

    pub fn remove_input_event(
        &mut self,
        group: InputGroupId,
        input_definition: &str,
    ) -> Result<()> {
        let c_def = CString::new(input_definition)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_RemoveInputEvent(self.handle, group.raw(), c_def.as_ptr())
        })
    }

    pub fn clear_input_group(&mut self, group: InputGroupId) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_ClearInputGroup(self.handle, group.raw()) })
    }

    pub fn set_input_group_state(&mut self, group: InputGroupId, state: u32) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SetInputGroupState(self.handle, group.raw(), state)
        })
    }

    pub fn request_reserved_key(
        &mut self,
        event: ClientEventId,
        choice1: &str,
        choice2: &str,
        choice3: &str,
    ) -> Result<()> {
        let c1 = CString::new(choice1)?;
        let c2 = CString::new(choice2)?;
        let c3 = CString::new(choice3)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestReservedKey(
                self.handle,
                event.raw(),
                c1.as_ptr(),
                c2.as_ptr(),
                c3.as_ptr(),
            )
        })
    }

    // -- Weather (legacy) ---------------------------------------------------

    pub fn weather_request_interpolated_observation(
        &mut self,
        request: DataRequestId,
        lat: f32,
        lon: f32,
        alt: f32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherRequestInterpolatedObservation(
                self.handle,
                request.raw(),
                lat,
                lon,
                alt,
            )
        })
    }

    pub fn weather_request_observation_at_station(
        &mut self,
        request: DataRequestId,
        icao: &str,
    ) -> Result<()> {
        let c_icao = CString::new(icao)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherRequestObservationAtStation(
                self.handle,
                request.raw(),
                c_icao.as_ptr(),
            )
        })
    }

    pub fn weather_request_observation_at_nearest_station(
        &mut self,
        request: DataRequestId,
        lat: f32,
        lon: f32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherRequestObservationAtNearestStation(
                self.handle,
                request.raw(),
                lat,
                lon,
            )
        })
    }

    pub fn weather_create_station(
        &mut self,
        request: DataRequestId,
        icao: &str,
        name: &str,
        lat: f32,
        lon: f32,
        alt: f32,
    ) -> Result<()> {
        let c_icao = CString::new(icao)?;
        let c_name = CString::new(name)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherCreateStation(
                self.handle,
                request.raw(),
                c_icao.as_ptr(),
                c_name.as_ptr(),
                lat,
                lon,
                alt,
            )
        })
    }

    pub fn weather_remove_station(
        &mut self,
        request: DataRequestId,
        icao: &str,
    ) -> Result<()> {
        let c_icao = CString::new(icao)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherRemoveStation(self.handle, request.raw(), c_icao.as_ptr())
        })
    }

    pub fn weather_set_observation(&mut self, seconds: u32, metar: &str) -> Result<()> {
        let c_metar = CString::new(metar)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherSetObservation(self.handle, seconds, c_metar.as_ptr())
        })
    }

    pub fn weather_set_mode_server(&mut self, port: u32, seconds: u32) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherSetModeServer(self.handle, port, seconds)
        })
    }

    pub fn weather_set_mode_theme(&mut self, theme: &str) -> Result<()> {
        let c_theme = CString::new(theme)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherSetModeTheme(self.handle, c_theme.as_ptr())
        })
    }

    pub fn weather_set_mode_global(&mut self) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_WeatherSetModeGlobal(self.handle) })
    }

    pub fn weather_set_mode_custom(&mut self) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_WeatherSetModeCustom(self.handle) })
    }

    pub fn weather_set_dynamic_update_rate(&mut self, rate: u32) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherSetDynamicUpdateRate(self.handle, rate)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn weather_request_cloud_state(
        &mut self,
        request: DataRequestId,
        min_lat: f32,
        min_lon: f32,
        min_alt: f32,
        max_lat: f32,
        max_lon: f32,
        max_alt: f32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherRequestCloudState(
                self.handle,
                request.raw(),
                min_lat,
                min_lon,
                min_alt,
                max_lat,
                max_lon,
                max_alt,
                0,
            )
        })
    }

    /// Configure thermal parameters and create a thermal at the given location.
    /// All `core_*` / `sink_*` parameters fall back to the SDK defaults if `None`.
    #[allow(clippy::too_many_arguments)]
    pub fn weather_create_thermal(
        &mut self,
        request: DataRequestId,
        lat: f32,
        lon: f32,
        alt: f32,
        radius: f32,
        height: f32,
        core_rate: f32,
        core_turbulence: f32,
        sink_rate: f32,
        sink_turbulence: f32,
        core_size: f32,
        core_transition_size: f32,
        sink_layer_size: f32,
        sink_transition_size: f32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherCreateThermal(
                self.handle,
                request.raw(),
                lat,
                lon,
                alt,
                radius,
                height,
                core_rate,
                core_turbulence,
                sink_rate,
                sink_turbulence,
                core_size,
                core_transition_size,
                sink_layer_size,
                sink_transition_size,
            )
        })
    }

    pub fn weather_remove_thermal(&mut self, object: ObjectId) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_WeatherRemoveThermal(self.handle, object.raw())
        })
    }

    // -- AI -----------------------------------------------------------------

    pub fn ai_create_parked_atc_aircraft(
        &mut self,
        container_title: &str,
        tail_number: &str,
        airport_id: &str,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        let c2 = CString::new(tail_number)?;
        let c3 = CString::new(airport_id)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateParkedATCAircraft(
                self.handle,
                c1.as_ptr(),
                c2.as_ptr(),
                c3.as_ptr(),
                request.raw(),
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ai_create_enroute_atc_aircraft(
        &mut self,
        container_title: &str,
        tail_number: &str,
        flight_number: i32,
        flight_plan_path: &str,
        flight_plan_position: f64,
        touch_and_go: bool,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        let c2 = CString::new(tail_number)?;
        let c3 = CString::new(flight_plan_path)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateEnrouteATCAircraft(
                self.handle,
                c1.as_ptr(),
                c2.as_ptr(),
                flight_number,
                c3.as_ptr(),
                flight_plan_position,
                touch_and_go as sys::BOOL,
                request.raw(),
            )
        })
    }

    pub fn ai_create_non_atc_aircraft(
        &mut self,
        container_title: &str,
        tail_number: &str,
        init_pos: InitPosition,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        let c2 = CString::new(tail_number)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateNonATCAircraft(
                self.handle,
                c1.as_ptr(),
                c2.as_ptr(),
                init_pos,
                request.raw(),
            )
        })
    }

    pub fn ai_create_simulated_object(
        &mut self,
        container_title: &str,
        init_pos: InitPosition,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateSimulatedObject(
                self.handle,
                c1.as_ptr(),
                init_pos,
                request.raw(),
            )
        })
    }

    pub fn ai_create_parked_atc_aircraft_ex1(
        &mut self,
        container_title: &str,
        livery: &str,
        tail_number: &str,
        airport_id: &str,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        let c2 = CString::new(livery)?;
        let c3 = CString::new(tail_number)?;
        let c4 = CString::new(airport_id)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateParkedATCAircraft_EX1(
                self.handle,
                c1.as_ptr(),
                c2.as_ptr(),
                c3.as_ptr(),
                c4.as_ptr(),
                request.raw(),
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ai_create_enroute_atc_aircraft_ex1(
        &mut self,
        container_title: &str,
        livery: &str,
        tail_number: &str,
        flight_number: i32,
        flight_plan_path: &str,
        flight_plan_position: f64,
        touch_and_go: bool,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        let c2 = CString::new(livery)?;
        let c3 = CString::new(tail_number)?;
        let c4 = CString::new(flight_plan_path)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateEnrouteATCAircraft_EX1(
                self.handle,
                c1.as_ptr(),
                c2.as_ptr(),
                c3.as_ptr(),
                flight_number,
                c4.as_ptr(),
                flight_plan_position,
                touch_and_go as sys::BOOL,
                request.raw(),
            )
        })
    }

    pub fn ai_create_non_atc_aircraft_ex1(
        &mut self,
        container_title: &str,
        livery: &str,
        tail_number: &str,
        init_pos: InitPosition,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        let c2 = CString::new(livery)?;
        let c3 = CString::new(tail_number)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateNonATCAircraft_EX1(
                self.handle,
                c1.as_ptr(),
                c2.as_ptr(),
                c3.as_ptr(),
                init_pos,
                request.raw(),
            )
        })
    }

    pub fn ai_create_simulated_object_ex1(
        &mut self,
        container_title: &str,
        livery: &str,
        init_pos: InitPosition,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(container_title)?;
        let c2 = CString::new(livery)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AICreateSimulatedObject_EX1(
                self.handle,
                c1.as_ptr(),
                c2.as_ptr(),
                init_pos,
                request.raw(),
            )
        })
    }

    pub fn ai_release_control(
        &mut self,
        object: ObjectId,
        request: DataRequestId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_AIReleaseControl(self.handle, object.raw(), request.raw())
        })
    }

    pub fn ai_remove_object(
        &mut self,
        object: ObjectId,
        request: DataRequestId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_AIRemoveObject(self.handle, object.raw(), request.raw())
        })
    }

    pub fn ai_set_aircraft_flight_plan(
        &mut self,
        object: ObjectId,
        flight_plan_path: &str,
        request: DataRequestId,
    ) -> Result<()> {
        let c1 = CString::new(flight_plan_path)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AISetAircraftFlightPlan(
                self.handle,
                object.raw(),
                c1.as_ptr(),
                request.raw(),
            )
        })
    }

    // -- Mission ------------------------------------------------------------

    pub fn execute_mission_action(&mut self, instance_id: sys::GUID) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_ExecuteMissionAction(self.handle, instance_id)
        })
    }

    pub fn complete_custom_mission_action(&mut self, instance_id: sys::GUID) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_CompleteCustomMissionAction(self.handle, instance_id)
        })
    }

    // -- Camera -------------------------------------------------------------

    pub fn camera_set_relative_6dof(
        &mut self,
        delta_xyz: [f32; 3],
        pitch_bank_heading_deg: [f32; 3],
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_CameraSetRelative6DOF(
                self.handle,
                delta_xyz[0],
                delta_xyz[1],
                delta_xyz[2],
                pitch_bank_heading_deg[0],
                pitch_bank_heading_deg[1],
                pitch_bank_heading_deg[2],
            )
        })
    }

    // -- Menu ---------------------------------------------------------------

    pub fn menu_add_item(
        &mut self,
        text: &str,
        event: ClientEventId,
        data: u32,
    ) -> Result<()> {
        let c = CString::new(text)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_MenuAddItem(self.handle, c.as_ptr(), event.raw(), data)
        })
    }

    pub fn menu_delete_item(&mut self, event: ClientEventId) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_MenuDeleteItem(self.handle, event.raw()) })
    }

    pub fn menu_add_sub_item(
        &mut self,
        parent_event: ClientEventId,
        text: &str,
        sub_event: ClientEventId,
        data: u32,
    ) -> Result<()> {
        let c = CString::new(text)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_MenuAddSubItem(
                self.handle,
                parent_event.raw(),
                c.as_ptr(),
                sub_event.raw(),
                data,
            )
        })
    }

    pub fn menu_delete_sub_item(
        &mut self,
        parent_event: ClientEventId,
        sub_event: ClientEventId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_MenuDeleteSubItem(self.handle, parent_event.raw(), sub_event.raw())
        })
    }

    // -- System state ------------------------------------------------------

    pub fn request_system_state(
        &mut self,
        request: DataRequestId,
        state: &str,
    ) -> Result<()> {
        let c = CString::new(state)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestSystemState(self.handle, request.raw(), c.as_ptr())
        })
    }

    pub fn set_system_state(
        &mut self,
        state: &str,
        integer: u32,
        floating: f32,
        string: &str,
    ) -> Result<()> {
        let c_state = CString::new(state)?;
        let c_str = CString::new(string)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_SetSystemState(
                self.handle,
                c_state.as_ptr(),
                integer,
                floating,
                c_str.as_ptr(),
            )
        })
    }

    // -- Client data --------------------------------------------------------

    pub fn map_client_data_name_to_id(
        &mut self,
        name: &str,
        id: ClientDataId,
    ) -> Result<()> {
        let c = CString::new(name)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_MapClientDataNameToID(self.handle, c.as_ptr(), id.raw())
        })
    }

    pub fn create_client_data(
        &mut self,
        id: ClientDataId,
        size: u32,
        flags: CreateClientDataFlag,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_CreateClientData(self.handle, id.raw(), size, flags.bits())
        })
    }

    pub fn add_to_client_data_definition(
        &mut self,
        def: ClientDataDefinitionId,
        offset: u32,
        size_or_type: u32,
        epsilon: f32,
        datum_id: u32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_AddToClientDataDefinition(
                self.handle,
                def.raw(),
                offset,
                size_or_type,
                epsilon,
                datum_id,
            )
        })
    }

    pub fn clear_client_data_definition(&mut self, def: ClientDataDefinitionId) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_ClearClientDataDefinition(self.handle, def.raw())
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn request_client_data(
        &mut self,
        id: ClientDataId,
        request: DataRequestId,
        def: ClientDataDefinitionId,
        period: ClientDataPeriod,
        flags: ClientDataRequestFlag,
        origin: u32,
        interval: u32,
        limit: u32,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestClientData(
                self.handle,
                id.raw(),
                request.raw(),
                def.raw(),
                period,
                flags.bits(),
                origin,
                interval,
                limit,
            )
        })
    }

    /// # Safety
    /// `T`'s layout must match the registered client-data definition.
    pub unsafe fn set_client_data<T: Copy>(
        &mut self,
        id: ClientDataId,
        def: ClientDataDefinitionId,
        flags: ClientDataSetFlag,
        data: &[T],
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SetClientData(
                self.handle,
                id.raw(),
                def.raw(),
                flags.bits(),
                0,
                std::mem::size_of::<T>() as sys::DWORD,
                data.as_ptr() as *mut std::os::raw::c_void,
            )
        })
    }

    // -- Flight ------------------------------------------------------------

    pub fn flight_load(&mut self, file_name: &str) -> Result<()> {
        let c = CString::new(file_name)?;
        Error::from_hresult(unsafe { sys::SimConnect_FlightLoad(self.handle, c.as_ptr()) })
    }

    pub fn flight_save(
        &mut self,
        file_name: &str,
        title: &str,
        description: &str,
    ) -> Result<()> {
        let c1 = CString::new(file_name)?;
        let c2 = CString::new(title)?;
        let c3 = CString::new(description)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_FlightSave(self.handle, c1.as_ptr(), c2.as_ptr(), c3.as_ptr(), 0)
        })
    }

    pub fn flight_plan_load(&mut self, file_name: &str) -> Result<()> {
        let c = CString::new(file_name)?;
        Error::from_hresult(unsafe { sys::SimConnect_FlightPlanLoad(self.handle, c.as_ptr()) })
    }

    // -- Text --------------------------------------------------------------

    /// Display a text message in the simulator. The text is sent as a
    /// length-prefixed buffer; this helper handles the size accounting.
    pub fn text(
        &mut self,
        ty: TextType,
        time_seconds: f32,
        event: ClientEventId,
        text: &str,
    ) -> Result<()> {
        let mut buf: Vec<u8> = text.as_bytes().to_vec();
        buf.push(0);
        Error::from_hresult(unsafe {
            sys::SimConnect_Text(
                self.handle,
                ty,
                time_seconds,
                event.raw(),
                buf.len() as sys::DWORD,
                buf.as_mut_ptr() as *mut std::os::raw::c_void,
            )
        })
    }

    // -- Facilities --------------------------------------------------------

    pub fn subscribe_to_facilities(
        &mut self,
        ty: FacilityListType,
        request: DataRequestId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SubscribeToFacilities(self.handle, ty, request.raw())
        })
    }

    pub fn unsubscribe_to_facilities(&mut self, ty: FacilityListType) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_UnsubscribeToFacilities(self.handle, ty)
        })
    }

    pub fn request_facilities_list(
        &mut self,
        ty: FacilityListType,
        request: DataRequestId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestFacilitiesList(self.handle, ty, request.raw())
        })
    }

    pub fn subscribe_to_facilities_ex1(
        &mut self,
        ty: FacilityListType,
        new_in_range: DataRequestId,
        old_out_range: DataRequestId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SubscribeToFacilities_EX1(
                self.handle,
                ty,
                new_in_range.raw(),
                old_out_range.raw(),
            )
        })
    }

    pub fn unsubscribe_to_facilities_ex1(
        &mut self,
        ty: FacilityListType,
        unsubscribe_new_in_range: bool,
        unsubscribe_old_out_range: bool,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_UnsubscribeToFacilities_EX1(
                self.handle,
                ty,
                unsubscribe_new_in_range,
                unsubscribe_old_out_range,
            )
        })
    }

    pub fn request_facilities_list_ex1(
        &mut self,
        ty: FacilityListType,
        request: DataRequestId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestFacilitiesList_EX1(self.handle, ty, request.raw())
        })
    }

    pub fn add_to_facility_definition(
        &mut self,
        def: DataDefinitionId,
        field_name: &str,
    ) -> Result<()> {
        let c = CString::new(field_name)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AddToFacilityDefinition(self.handle, def.raw(), c.as_ptr())
        })
    }

    pub fn request_facility_data(
        &mut self,
        def: DataDefinitionId,
        request: DataRequestId,
        icao: &str,
        region: &str,
    ) -> Result<()> {
        let c1 = CString::new(icao)?;
        let c2 = CString::new(region)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestFacilityData(
                self.handle,
                def.raw(),
                request.raw(),
                c1.as_ptr(),
                c2.as_ptr(),
            )
        })
    }

    pub fn request_facility_data_ex1(
        &mut self,
        def: DataDefinitionId,
        request: DataRequestId,
        icao: &str,
        region: &str,
        ty: u8,
    ) -> Result<()> {
        let c1 = CString::new(icao)?;
        let c2 = CString::new(region)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestFacilityData_EX1(
                self.handle,
                def.raw(),
                request.raw(),
                c1.as_ptr(),
                c2.as_ptr(),
                ty as c_char,
            )
        })
    }

    /// # Safety
    /// `filter_data` must match the layout expected for `filter_path`.
    pub unsafe fn add_facility_data_definition_filter<T: Copy>(
        &mut self,
        def: DataDefinitionId,
        filter_path: &str,
        filter_data: &T,
    ) -> Result<()> {
        let c = CString::new(filter_path)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_AddFacilityDataDefinitionFilter(
                self.handle,
                def.raw(),
                c.as_ptr(),
                std::mem::size_of::<T>() as sys::DWORD,
                filter_data as *const T as *mut std::os::raw::c_void,
            )
        })
    }

    pub fn clear_all_facility_data_definition_filters(
        &mut self,
        def: DataDefinitionId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_ClearAllFacilityDataDefinitionFilters(self.handle, def.raw())
        })
    }

    pub fn request_all_facilities(
        &mut self,
        ty: FacilityListType,
        request: DataRequestId,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestAllFacilities(self.handle, ty, request.raw())
        })
    }

    pub fn request_jetway_data(
        &mut self,
        airport_icao: &str,
        indexes: &[i32],
    ) -> Result<()> {
        let c = CString::new(airport_icao)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_RequestJetwayData(
                self.handle,
                c.as_ptr(),
                indexes.len() as sys::DWORD,
                indexes.as_ptr() as *mut i32,
            )
        })
    }

    pub fn enumerate_controllers(&mut self) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_EnumerateControllers(self.handle) })
    }

    pub fn enumerate_simobjects_and_liveries(
        &mut self,
        request: DataRequestId,
        ty: SimObjectType,
    ) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_EnumerateSimObjectsAndLiveries(self.handle, request.raw(), ty)
        })
    }

    // -- Action -------------------------------------------------------------

    /// # Safety
    /// `param_values` must match the parameter schema declared by the action.
    pub unsafe fn execute_action<T: Copy>(
        &mut self,
        request_id: u32,
        action_id: &str,
        param_values: &[T],
    ) -> Result<()> {
        let c = CString::new(action_id)?;
        Error::from_hresult(unsafe {
            sys::SimConnect_ExecuteAction(
                self.handle,
                request_id,
                c.as_ptr(),
                std::mem::size_of_val(param_values) as sys::DWORD,
                param_values.as_ptr() as *mut std::os::raw::c_void,
            )
        })
    }

    // -- Input Events ------------------------------------------------------

    pub fn enumerate_input_events(&mut self, request: DataRequestId) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_EnumerateInputEvents(self.handle, request.raw())
        })
    }

    pub fn get_input_event(&mut self, request: DataRequestId, hash: u64) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_GetInputEvent(self.handle, request.raw(), hash)
        })
    }

    /// # Safety
    /// `value`'s layout must match the input event's declared type.
    pub unsafe fn set_input_event<T: Copy>(&mut self, hash: u64, value: &T) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_SetInputEvent(
                self.handle,
                hash,
                std::mem::size_of::<T>() as sys::DWORD,
                value as *const T as *mut std::os::raw::c_void,
            )
        })
    }

    pub fn subscribe_input_event(&mut self, hash: u64) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_SubscribeInputEvent(self.handle, hash) })
    }

    pub fn unsubscribe_input_event(&mut self, hash: u64) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_UnsubscribeInputEvent(self.handle, hash) })
    }

    pub fn enumerate_input_event_params(&mut self, hash: u64) -> Result<()> {
        Error::from_hresult(unsafe {
            sys::SimConnect_EnumerateInputEventParams(self.handle, hash)
        })
    }

    // -- Flow events --------------------------------------------------------

    pub fn subscribe_to_flow_event(&mut self) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_SubscribeToFlowEvent(self.handle) })
    }

    pub fn unsubscribe_to_flow_event(&mut self) -> Result<()> {
        Error::from_hresult(unsafe { sys::SimConnect_UnsubscribeToFlowEvent(self.handle) })
    }
}

impl Drop for SimConnect {
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe {
                sys::SimConnect_Close(self.handle);
            }
            self.handle = 0;
        }
    }
}

impl std::fmt::Debug for SimConnect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimConnect")
            .field("handle", &(self.handle as u64))
            .finish()
    }
}
