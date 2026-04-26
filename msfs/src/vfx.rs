//! Safe wrapper around the MSFS `fsVfx*` API.
//!
//! A [`VfxInstance`] is a host-owned visual effect (a particle graph asset
//! identified by GUID) spawned either at a world position
//! ([`VfxInstance::spawn_in_world`]) or attached to a sim object
//! ([`VfxInstance::spawn_on_sim_object`]). The handle's `Drop` calls
//! `fsVfxDestroyInstance` so leaked effects don't pile up between gauge
//! reloads. To keep an effect alive past the handle, call
//! [`VfxInstance::leak`].
//!
//! Graph parameters are bound via [`VfxParam`]; pass them as a slice and the
//! API will materialize the C strings + array layout for the host call.

use crate::sys;

use std::ffi::CString;
use std::os::raw::c_int;
use std::ptr;

/// Sentinel value the SDK uses for "no instance".
const FSVFXID_NULL: sys::FsVfxId = 0xFFFF_FFFF_0000_0000_u64 as sys::FsVfxId;

/// Re-export the host's 3D vector type so callers don't need to import `sys`.
pub type Vec3d = sys::FsVec3d;

/// Re-export the host's sim-object id type.
pub type SimObjId = sys::FsSimObjId;

#[inline]
pub const fn vec3d(x: f64, y: f64, z: f64) -> Vec3d {
    sys::FsVec3d { x, y, z }
}

/// One parameter binding for a VFX graph.
///
/// The host expects two C strings: the parameter name and an RPN expression
/// evaluated each frame to drive it.
#[derive(Debug, Clone)]
pub struct VfxParam {
    pub name: String,
    pub rpn: String,
}

impl VfxParam {
    pub fn new(name: impl Into<String>, rpn: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rpn: rpn.into(),
        }
    }
}

/// Marshals a `&[VfxParam]` into the `(*mut FsVfxGraphParam, len)` shape the
/// host expects, owning all the intermediate `CString`s for the duration of
/// the FFI call.
struct ParamBuffer {
    // Held to keep the `*const c_char` pointers in `entries` alive.
    _strings: Vec<(CString, CString)>,
    entries: Vec<sys::FsVfxGraphParam>,
}

impl ParamBuffer {
    fn new(params: &[VfxParam]) -> Option<Self> {
        let mut strings = Vec::with_capacity(params.len());
        let mut entries = Vec::with_capacity(params.len());
        for p in params {
            let name = CString::new(p.name.as_str()).ok()?;
            let rpn = CString::new(p.rpn.as_str()).ok()?;
            entries.push(sys::FsVfxGraphParam {
                paramName: name.as_ptr(),
                RPNExpression: rpn.as_ptr(),
            });
            strings.push((name, rpn));
        }
        Some(Self {
            _strings: strings,
            entries,
        })
    }

    fn as_ffi(&mut self) -> (*mut sys::FsVfxGraphParam, c_int) {
        if self.entries.is_empty() {
            (ptr::null_mut(), 0)
        } else {
            (self.entries.as_mut_ptr(), self.entries.len() as c_int)
        }
    }
}

/// Owned handle to a spawned VFX instance.
///
/// Dropped via `fsVfxDestroyInstance`. Use [`VfxInstance::leak`] to detach
/// the host instance from the Rust handle.
pub struct VfxInstance {
    id: sys::FsVfxId,
}

unsafe impl Send for VfxInstance {}

impl VfxInstance {
    /// Spawn an effect at a fixed lat/long/alt position in the world.
    ///
    /// `lla` is the spawn position (latitude/longitude/altitude — units per
    /// the SDK convention for `FsVec3d` in this call). `pbh` is the
    /// pitch/bank/heading orientation. `min_emission_time` is the minimum
    /// emission lifetime in seconds, or `None` to use the graph default.
    pub fn spawn_in_world(
        guid: &str,
        lla: Vec3d,
        pbh: Vec3d,
        min_emission_time: Option<f32>,
        params: &[VfxParam],
    ) -> Option<Self> {
        let guid_c = CString::new(guid).ok()?;
        let mut buf = ParamBuffer::new(params)?;
        let (ptr_, len) = buf.as_ffi();
        let id = unsafe {
            sys::fsVfxSpawnInWorld(
                guid_c.as_ptr(),
                lla,
                pbh,
                min_emission_time.unwrap_or(-1.0),
                ptr_,
                len,
            )
        };
        Self::from_raw(id)
    }

    /// Spawn an effect attached to a sim object, optionally on a specific
    /// node, with positional and rotational offsets.
    pub fn spawn_on_sim_object(
        guid: &str,
        sim_obj: SimObjId,
        node_name: Option<&str>,
        offset: Vec3d,
        pbh_offset: Vec3d,
        min_emission_time: Option<f32>,
        params: &[VfxParam],
    ) -> Option<Self> {
        let guid_c = CString::new(guid).ok()?;
        let node_c = match node_name {
            Some(n) => Some(CString::new(n).ok()?),
            None => None,
        };
        let node_ptr = node_c.as_ref().map_or(ptr::null(), |c| c.as_ptr());
        let mut buf = ParamBuffer::new(params)?;
        let (ptr_, len) = buf.as_ffi();
        let id = unsafe {
            sys::fsVfxSpawnOnSimObject(
                guid_c.as_ptr(),
                sim_obj,
                node_ptr,
                offset,
                pbh_offset,
                min_emission_time.unwrap_or(-1.0),
                ptr_,
                len,
            )
        };
        Self::from_raw(id)
    }

    fn from_raw(id: sys::FsVfxId) -> Option<Self> {
        if id == FSVFXID_NULL {
            None
        } else {
            Some(Self { id })
        }
    }

    /// Underlying `FsVfxId`. Mostly useful for FFI interop.
    #[inline]
    pub fn id(&self) -> sys::FsVfxId {
        self.id
    }

    /// Begin (or resume) emission.
    pub fn play(&self) -> bool {
        unsafe { sys::fsVfxPlayInstance(self.id) }
    }

    /// Stop emission. The instance remains valid and can be played again.
    pub fn stop(&self) -> bool {
        unsafe { sys::fsVfxStopInstance(self.id) }
    }

    /// True if the instance is currently playing.
    pub fn is_playing(&self) -> bool {
        unsafe { sys::fsVfxIsInstancePlaying(self.id) }
    }

    /// True once the configured `min_emission_time` has elapsed.
    pub fn is_min_time_passed(&self) -> bool {
        unsafe { sys::fsVfxIsMinTimePassed(self.id) }
    }

    /// True while the host still recognizes the id (the host may invalidate
    /// effects independently — e.g. on flight reload).
    pub fn is_valid(&self) -> bool {
        unsafe { sys::fsVfxIsValid(self.id) }
    }

    /// Update the local offset on a sim-object-attached effect.
    pub fn set_offset(&self, offset: Vec3d) -> bool {
        unsafe { sys::fsVfxSetOffset(self.id, offset) }
    }

    /// Update the world position of a world-spawned effect.
    pub fn set_world_position(&self, lla: Vec3d) -> bool {
        unsafe { sys::fsVfxSetWorldPosition(self.id, lla) }
    }

    /// Update the orientation (pitch/bank/heading).
    pub fn set_rotation(&self, pbh: Vec3d) -> bool {
        unsafe { sys::fsVfxSetRotation(self.id, pbh) }
    }

    /// Detach the host instance from this handle. The effect lives on until
    /// the host destroys it (e.g. when its emission ends naturally).
    pub fn leak(self) -> sys::FsVfxId {
        let id = self.id;
        std::mem::forget(self);
        id
    }
}

impl Drop for VfxInstance {
    fn drop(&mut self) {
        if self.id != FSVFXID_NULL {
            unsafe { sys::fsVfxDestroyInstance(self.id) };
            self.id = FSVFXID_NULL;
        }
    }
}
