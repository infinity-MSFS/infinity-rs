//! Named MSFS key events.
//!
//! The MSFS gauge API identifies key events by the numeric `KEY_*` constants
//! from `MSFS/Legacy/gauges.h` (lifted into [`crate::sys`] by bindgen). MSFS
//! WASM has no runtime name→id lookup, so this module exposes the subset the
//! avionics gauges need as [`EventId`] constants, plus `trigger_key` helpers
//! over [`super::trigger1`] / [`super::trigger0`].

use super::EventId;
use crate::sys;
use crate::utils::{FsParamArg, FsParamError};

macro_rules! key_event_ids {
    ($($(#[$m:meta])* $name:ident = $sys_const:ident;)*) => {
        $(
            $(#[$m])*
            pub const $name: EventId = EventId::raw(sys::$sys_const as sys::FsEventId);
        )*
    };
}

key_event_ids! {
    /// `KEY_HEADING_BUG_SET` — set the autopilot heading bug (degrees).
    HEADING_BUG_SET = KEY_HEADING_BUG_SET;
    /// `KEY_TACAN1_SET` — set TACAN 1 channel.
    TACAN1_SET = KEY_TACAN1_SET;
    /// `KEY_TACAN2_SET` — set TACAN 2 channel.
    TACAN2_SET = KEY_TACAN2_SET;
    /// `KEY_TACAN1_ACTIVE_CHANNEL_SET`.
    TACAN1_ACTIVE_CHANNEL_SET = KEY_TACAN1_ACTIVE_CHANNEL_SET;
    /// `KEY_TACAN1_ACTIVE_MODE_SET`.
    TACAN1_ACTIVE_MODE_SET = KEY_TACAN1_ACTIVE_MODE_SET;
    /// `KEY_TACAN2_ACTIVE_CHANNEL_SET`.
    TACAN2_ACTIVE_CHANNEL_SET = KEY_TACAN2_ACTIVE_CHANNEL_SET;
    /// `KEY_TACAN2_ACTIVE_MODE_SET`.
    TACAN2_ACTIVE_MODE_SET = KEY_TACAN2_ACTIVE_MODE_SET;
    /// `KEY_VOR1_SET` — set the VOR 1 OBS course.
    VOR1_SET = KEY_VOR1_SET;
    /// `KEY_VOR2_SET` — set the VOR 2 OBS course.
    VOR2_SET = KEY_VOR2_SET;
    /// `KEY_VOR3_SET` — set the VOR 3 OBS course.
    VOR3_SET = KEY_VOR3_SET;
    /// `KEY_NAV1_STBY_SET_HZ` — set NAV 1 standby frequency (Hz).
    NAV1_STBY_SET_HZ = KEY_NAV1_STBY_SET_HZ;
    /// `KEY_NAV2_STBY_SET_HZ` — set NAV 2 standby frequency (Hz).
    NAV2_STBY_SET_HZ = KEY_NAV2_STBY_SET_HZ;
    /// `KEY_XPNDR_SET` — set the transponder code (BCD-encoded).
    XPNDR_SET = KEY_XPNDR_SET;
    /// `KEY_NAV1_RADIO_SWAP` — swap NAV 1 active/standby.
    NAV1_RADIO_SWAP = KEY_NAV1_RADIO_SWAP;
    /// `KEY_NAV2_RADIO_SWAP` — swap NAV 2 active/standby.
    NAV2_RADIO_SWAP = KEY_NAV2_RADIO_SWAP;
    /// `KEY_NAV1_RADIO_SET` — set NAV 1 active frequency (BCD-encoded MHz).
    NAV1_RADIO_SET = KEY_NAV1_RADIO_SET;
    /// `KEY_NAV2_RADIO_SET` — set NAV 2 active frequency (BCD-encoded MHz).
    NAV2_RADIO_SET = KEY_NAV2_RADIO_SET;
    /// `KEY_KOHLSMAN_SET` — set altimeter Kohlsman setting (mb × 16).
    KOHLSMAN_SET = KEY_KOHLSMAN_SET;
    /// `KEY_KOHLSMAN_INC` — increment altimeter Kohlsman setting.
    KOHLSMAN_INC = KEY_KOHLSMAN_INC;
    /// `KEY_KOHLSMAN_DEC` — decrement altimeter Kohlsman setting.
    KOHLSMAN_DEC = KEY_KOHLSMAN_DEC;
    /// `KEY_XPNDR_IDENT_ON` — trigger transponder IDENT pulse.
    XPNDR_IDENT_ON = KEY_XPNDR_IDENT_ON;
}

/// Trigger a key event with a single integer argument — the common case for
/// `KEY_*_SET` events, e.g. `trigger_key(keys::HEADING_BUG_SET, hdg)`.
#[inline]
pub fn trigger_key(event: EventId, value: u32) -> Result<(), FsParamError> {
    super::trigger1(event, FsParamArg::Index(value))
}

/// Trigger a key event with no argument.
#[inline]
pub fn trigger_key0(event: EventId) -> Result<(), FsParamError> {
    super::trigger0(event)
}
