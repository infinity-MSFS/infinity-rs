use crate::sys::*;
use std::mem::MaybeUninit;
use std::os::raw::c_char;

#[deprecated(note="Not implemented yet")]
pub struct MsfsAVar {
    id: FsAVarId,
    unit_id: FsUnitId,
}

#[deprecated(note="Not implemented yet")]
pub struct MsfsBVar {
    id: FsBVarId,
    unit_id: FsUnitId,
}

#[deprecated(note="Not implemented yet")]
pub struct MsfsEVar {
    id: FsEVarId,
    unit_id: FsUnitId,
}

pub struct MsfsLVar {
    id: FsLVarId,
    unit_id: FsUnitId,
}

#[deprecated(note="Not implemented yet")]
pub struct MsfsIVar {
    id: FsIVarId,
    unit_id: FsUnitId,
}

#[deprecated(note="Not implemented yet")]
pub struct MsfsOVar {
    id: FsOVarId,
    unit_id: FsUnitId,
}

#[deprecated(note="Not implemented yet")]
pub struct MsfsZVar {
    id: FsZVarId,
    unit_id: FsUnitId,
}

impl MsfsLVar {
    /// Create a new LVar with the given name and unit. Should be called once during initialization.
    pub fn new(name: &str, unit: &str) -> Self {
        let name_cstr = std::ffi::CString::new(name).unwrap();
        let unit_cstr = std::ffi::CString::new(unit).unwrap();
        let name_ptr: *const c_char = name_cstr.as_ptr();
        let unit_ptr: *const c_char = unit_cstr.as_ptr();
        let id = unsafe { fsVarsRegisterLVar(name_ptr) };
        let unit_id = unsafe { fsVarsGetUnitId(unit_ptr) };
        MsfsLVar { id, unit_id }
    }

    /// Set the value of the LVar. Can be called anytime after creation.
    pub fn set_value(&self, value: f64) {
        unsafe {
            fsVarsLVarSet(self.id, self.unit_id, value);
        }
    }

    /// Get the value of the LVar. Can be called anytime after creation.
    pub fn get_value(&self) -> Option<f64> {
        let mut result = MaybeUninit::<f64>::uninit();
        let err = unsafe { fsVarsLVarGet(self.id, self.unit_id, result.as_mut_ptr()) };
        if err == FsVarError_FS_VAR_ERROR_NONE {
            Some(unsafe { result.assume_init() })
        } else {
            None
        }
    }
}
