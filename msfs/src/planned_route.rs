//! Safe wrapper around the MSFS `fsPlannedRoute*` API.
//!
//! Three primitives:
//!
//! * [`current_efb_route`] — borrow the route currently held by the EFB, if
//!   one is set.
//! * [`subscribe_broadcast`] — listen for route updates pushed by the EFB
//!   or other planners.
//! * [`subscribe_requests`] — receive [`RouteRequest`] handles when a
//!   consumer asks "what is the current planned route?"; respond by handing
//!   back any [`PlannedRoute`] borrow.
//!
//! Routes are host-owned. The wrappers here only borrow them — there is no
//! free/destroy entry point in the SDK header, so we never call one.

use crate::sys;

use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;

#[derive(Debug)]
pub enum PlannedRouteError {
    RegistrationFailed,
    UnregisterFailed,
}

impl std::fmt::Display for PlannedRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegistrationFailed => f.write_str("planned route registration failed"),
            Self::UnregisterFailed => f.write_str("planned route unregistration failed"),
        }
    }
}

impl std::error::Error for PlannedRouteError {}

pub type PlannedRouteResult<T> = Result<T, PlannedRouteError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FlightAltitudeType {
    None = 0,
    Feet = 1,
    FlightLevel = 2,
}

impl FlightAltitudeType {
    fn from_raw(raw: sys::FsFlightAltitudeType) -> Self {
        match raw as u32 {
            1 => Self::Feet,
            2 => Self::FlightLevel,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EnrouteLegType {
    Normal = 0,
    LatLon = 1,
    PointBearingDistance = 2,
}

impl EnrouteLegType {
    fn from_raw(raw: sys::FsEnrouteLegType) -> Self {
        match raw as u32 {
            1 => Self::LatLon,
            2 => Self::PointBearingDistance,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightAltitude {
    pub kind: FlightAltitudeType,
    /// Altitude value, interpreted per `kind` (feet or flight level).
    pub value: i32,
}

impl FlightAltitude {
    fn from_raw(raw: &sys::FsFlightAltitude) -> Self {
        Self {
            kind: FlightAltitudeType::from_raw(raw.type_),
            value: raw.altitude,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualPattern {
    pub pattern: i32,
    pub is_left_traffic: bool,
    pub distance: f32,
    pub altitude: f32,
}

impl VisualPattern {
    fn from_raw(raw: &sys::FsVisualPattern) -> Self {
        Self {
            pattern: raw.pattern,
            is_left_traffic: raw.isLeftTraffic,
            distance: raw.distance,
            altitude: raw.altitude,
        }
    }
}

/// Borrowed view over a single enroute leg.
#[derive(Clone, Copy)]
pub struct EnrouteLegRef<'a> {
    raw: &'a sys::FsEnrouteLeg,
}

impl<'a> EnrouteLegRef<'a> {
    pub fn kind(self) -> EnrouteLegType {
        EnrouteLegType::from_raw(self.raw.type_)
    }

    pub fn fix_icao(self) -> &'a sys::FsIcao {
        &self.raw.fixIcao
    }

    pub fn via(self) -> &'a str {
        c_array_to_str(&self.raw.via)
    }

    pub fn name(self) -> &'a str {
        unsafe { c_str_to_borrowed(self.raw.name) }
    }

    pub fn altitude(self) -> FlightAltitude {
        FlightAltitude::from_raw(&self.raw.altitude)
    }

    pub fn lat(self) -> f64 {
        self.raw.lat
    }

    pub fn lon(self) -> f64 {
        self.raw.lon
    }

    pub fn pbd_reference_icao(self) -> &'a sys::FsIcao {
        &self.raw.pbdReferenceIcao
    }

    pub fn bearing(self) -> f64 {
        self.raw.bearing
    }

    pub fn distance(self) -> f64 {
        self.raw.distance
    }
}

/// Borrowed view over a host-owned `FsPlannedRoute`.
///
/// Holds a raw pointer plus a lifetime so the borrow can't outlive the
/// callback (or, in the case of [`current_efb_route`], the next host poke).
#[derive(Clone, Copy)]
pub struct PlannedRoute<'a> {
    raw: *const sys::FsPlannedRoute,
    _life: std::marker::PhantomData<&'a sys::FsPlannedRoute>,
}

unsafe impl Send for PlannedRoute<'_> {}

impl<'a> PlannedRoute<'a> {
    /// # Safety
    /// `raw` must be a valid `FsPlannedRoute*` for the duration `'a`.
    pub unsafe fn from_raw(raw: *const sys::FsPlannedRoute) -> Self {
        Self {
            raw,
            _life: std::marker::PhantomData,
        }
    }

    /// Underlying pointer. Useful to relay the route into
    /// [`RouteRequest::respond`].
    #[inline]
    pub fn as_ptr(&self) -> *const sys::FsPlannedRoute {
        self.raw
    }

    fn r(&self) -> &'a sys::FsPlannedRoute {
        unsafe { &*self.raw }
    }

    pub fn departure_airport(&self) -> &'a sys::FsIcao {
        &self.r().departureAirport
    }

    pub fn departure_runway(&self) -> sys::FsRunwayIdentifier {
        // FsPlannedRoute is `#pragma pack(1)`, so inner struct fields can't
        // be borrowed at their natural alignment — copy out instead.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.r().departureRunway)) }
    }

    pub fn departure(&self) -> &'a str {
        c_array_to_str(&self.r().departure)
    }

    pub fn departure_transition(&self) -> &'a str {
        c_array_to_str(&self.r().departureTransition)
    }

    pub fn departure_visual_pattern(&self) -> VisualPattern {
        VisualPattern::from_raw(&self.r().departureVisualPattern)
    }

    pub fn destination_airport(&self) -> &'a sys::FsIcao {
        &self.r().destinationAirport
    }

    pub fn destination_runway(&self) -> sys::FsRunwayIdentifier {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.r().destinationRunway)) }
    }

    pub fn arrival(&self) -> &'a str {
        c_array_to_str(&self.r().arrival)
    }

    pub fn arrival_transition(&self) -> &'a str {
        c_array_to_str(&self.r().arrivalTransition)
    }

    pub fn approach(&self) -> sys::FsApproachIdentifier {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.r().approach)) }
    }

    pub fn approach_transition(&self) -> &'a str {
        c_array_to_str(&self.r().approachTransition)
    }

    pub fn approach_visual_pattern(&self) -> VisualPattern {
        VisualPattern::from_raw(&self.r().approachVisualPattern)
    }

    pub fn cruise_altitude(&self) -> FlightAltitude {
        FlightAltitude::from_raw(&self.r().cruiseAltitude)
    }

    pub fn is_vfr(&self) -> bool {
        self.r().isVfr
    }

    pub fn enroute_legs(&self) -> &'a [sys::FsEnrouteLeg] {
        let r = self.r();
        unsafe {
            if r.enrouteLegs.is_null() || r.numEnrouteLegs <= 0 {
                &[]
            } else {
                std::slice::from_raw_parts(r.enrouteLegs, r.numEnrouteLegs as usize)
            }
        }
    }

    pub fn iter_enroute_legs(&self) -> impl Iterator<Item = EnrouteLegRef<'a>> {
        self.enroute_legs().iter().map(|l| EnrouteLegRef { raw: l })
    }
}

/// Fetch the EFB's current planned route, if one is set.
///
/// The returned borrow is tied to `'static` for ergonomics; treat it as
/// short-lived and re-fetch each frame rather than caching across host
/// callbacks (the EFB may swap routes at any time).
pub fn current_efb_route() -> Option<PlannedRoute<'static>> {
    let raw = unsafe { sys::fsPlannedRouteGetEfbRoute() };
    if raw.is_null() {
        None
    } else {
        Some(unsafe { PlannedRoute::from_raw(raw) })
    }
}

// -- Subscriptions -------------------------------------------------------------

type BroadcastHandler = Box<dyn FnMut(PlannedRoute<'_>)>;
type RequestHandler = Box<dyn FnMut(RouteRequest)>;

struct BroadcastSlot {
    id: u64,
    handler: BroadcastHandler,
}

struct RequestSlot {
    id: u64,
    handler: RequestHandler,
}

#[derive(Default)]
struct Registry {
    broadcast_registered: bool,
    request_registered: bool,
    next_id: u64,
    broadcast: Vec<BroadcastSlot>,
    requests: Vec<RequestSlot>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

extern "C" fn broadcast_trampoline(route: *const sys::FsPlannedRoute, _ctx: *mut c_void) {
    if route.is_null() {
        return;
    }
    let view = unsafe { PlannedRoute::from_raw(route) };
    REGISTRY.with(|registry| {
        let mut r = registry.borrow_mut();
        for slot in &mut r.broadcast {
            (slot.handler)(view);
        }
    });
}

extern "C" fn request_trampoline(id: sys::FsRouteRequestId, _ctx: *mut c_void) {
    REGISTRY.with(|registry| {
        let mut r = registry.borrow_mut();
        for slot in &mut r.requests {
            (slot.handler)(RouteRequest { id });
        }
    });
}

/// Handle issued by the host when a consumer asks for the current route.
///
/// Forward whatever route you have via [`RouteRequest::respond`]. The handle
/// can be ignored if you have nothing to send — the host will time the
/// requester out.
pub struct RouteRequest {
    id: sys::FsRouteRequestId,
}

impl RouteRequest {
    #[inline]
    pub fn id(&self) -> sys::FsRouteRequestId {
        self.id
    }

    /// Reply with a planned route. The host copies what it needs before
    /// returning, so a borrowed view (e.g. from [`current_efb_route`]) is
    /// fine.
    pub fn respond(self, route: PlannedRoute<'_>) -> bool {
        unsafe {
            sys::fsPlannedRouteRespondToRequest(self.id, route.as_ptr() as *mut _)
        }
    }
}

pub struct BroadcastSubscription {
    id: u64,
}

impl BroadcastSubscription {
    pub fn register(handler: impl FnMut(PlannedRoute<'_>) + 'static) -> PlannedRouteResult<Self> {
        REGISTRY.with(|registry| {
            let mut r = registry.borrow_mut();
            if !r.broadcast_registered {
                let ok = unsafe {
                    sys::fsPlannedRouteRegisterForBroadcast(
                        Some(broadcast_trampoline),
                        ptr::null_mut(),
                    )
                };
                if !ok {
                    return Err(PlannedRouteError::RegistrationFailed);
                }
                r.broadcast_registered = true;
            }
            let id = r.next_id;
            r.next_id = r.next_id.wrapping_add(1);
            r.broadcast.push(BroadcastSlot {
                id,
                handler: Box::new(handler),
            });
            Ok(Self { id })
        })
    }
}

impl Drop for BroadcastSubscription {
    fn drop(&mut self) {
        REGISTRY.with(|registry| {
            let mut r = registry.borrow_mut();
            r.broadcast.retain(|s| s.id != self.id);
            if r.broadcast.is_empty() && r.broadcast_registered {
                unsafe {
                    let _ = sys::fsPlannedRouteUnregisterForBroadcast(Some(broadcast_trampoline));
                };
                r.broadcast_registered = false;
            }
        });
    }
}

pub struct RequestSubscription {
    id: u64,
}

impl RequestSubscription {
    pub fn register(handler: impl FnMut(RouteRequest) + 'static) -> PlannedRouteResult<Self> {
        REGISTRY.with(|registry| {
            let mut r = registry.borrow_mut();
            if !r.request_registered {
                let ok = unsafe {
                    sys::fsPlannedRouteRegisterForRequest(
                        Some(request_trampoline),
                        ptr::null_mut(),
                    )
                };
                if !ok {
                    return Err(PlannedRouteError::RegistrationFailed);
                }
                r.request_registered = true;
            }
            let id = r.next_id;
            r.next_id = r.next_id.wrapping_add(1);
            r.requests.push(RequestSlot {
                id,
                handler: Box::new(handler),
            });
            Ok(Self { id })
        })
    }
}

impl Drop for RequestSubscription {
    fn drop(&mut self) {
        REGISTRY.with(|registry| {
            let mut r = registry.borrow_mut();
            r.requests.retain(|s| s.id != self.id);
            if r.requests.is_empty() && r.request_registered {
                unsafe {
                    let _ = sys::fsPlannedRouteUnregisterForRequest(Some(request_trampoline));
                };
                r.request_registered = false;
            }
        });
    }
}

pub fn subscribe_broadcast(
    handler: impl FnMut(PlannedRoute<'_>) + 'static,
) -> PlannedRouteResult<BroadcastSubscription> {
    BroadcastSubscription::register(handler)
}

pub fn subscribe_requests(
    handler: impl FnMut(RouteRequest) + 'static,
) -> PlannedRouteResult<RequestSubscription> {
    RequestSubscription::register(handler)
}

// -- helpers ------------------------------------------------------------------

fn c_array_to_str(buf: &[std::os::raw::c_char]) -> &str {
    // Trim at the first NUL, then decode as UTF-8.
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len()) };
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

unsafe fn c_str_to_borrowed<'a>(p: *const std::os::raw::c_char) -> &'a str {
    if p.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("")
    }
}
