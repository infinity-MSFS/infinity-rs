//! # Example: planned-route relay system
//!
//! Subscribes to two channels of the EFB route API:
//!
//! * **Broadcast** — every time the EFB pushes a new route, log a summary.
//! * **Requests** — when another consumer asks for a route, hand it the
//!   EFB's current one (if any).
//!
//! Both subscriptions are stored on the system; their `Drop` impls
//! unregister with the host when the system is torn down.

use infinity_rs::planned_route;
use infinity_rs::prelude::*;

pub struct RouteRelay {
    broadcast: Option<BroadcastSubscription>,
    requests: Option<RequestSubscription>,
}

impl RouteRelay {
    pub fn new() -> Self {
        Self {
            broadcast: None,
            requests: None,
        }
    }
}

impl Default for RouteRelay {
    fn default() -> Self {
        Self::new()
    }
}

impl System for RouteRelay {
    fn init(&mut self, _ctx: &Context, _install: &SystemInstall) -> bool {
        // Stage 1: log every route broadcast for visibility.
        self.broadcast = planned_route::subscribe_broadcast(|route| {
            let dep = route.departure_airport();
            let dst = route.destination_airport();
            let cruise = route.cruise_altitude();
            let leg_count = route.enroute_legs().len();
            eprintln!(
                "[route] {dep_id} -> {dst_id} | {flavor} | cruise {cruise:?} | {leg_count} legs",
                dep_id = icao_str(dep.ident.as_ref()),
                dst_id = icao_str(dst.ident.as_ref()),
                flavor = if route.is_vfr() { "VFR" } else { "IFR" },
            );
        })
        .ok();

        // Stage 2: respond to incoming "what's the route?" requests by
        // forwarding whatever the EFB currently holds.
        self.requests = planned_route::subscribe_requests(|request| {
            match planned_route::current_efb_route() {
                Some(route) => {
                    let _ = request.respond(route);
                }
                None => {
                    // No route to share — let the request id drop without
                    // responding so the host can time the requester out.
                    let _ = request;
                }
            }
        })
        .ok();

        true
    }

    fn update(&mut self, _ctx: &Context, _dt: f32) -> bool {
        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        // Drop both subscriptions — their `Drop` impls call the matching
        // `fsPlannedRouteUnregisterFor*` once the slot count hits zero.
        self.broadcast = None;
        self.requests = None;
        true
    }
}

/// Render the `ident` field of an `FsIcao`'s C-style buffer as a `&str`,
/// stopping at the first NUL.
fn icao_str(buf: &[std::os::raw::c_char]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .map(|c| *c as u8)
        .take_while(|b| *b != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

infinity_rs::export_system!(
    name = planned_route_relay,
    state = RouteRelay,
    ctor = RouteRelay::new()
);
