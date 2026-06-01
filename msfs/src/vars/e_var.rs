use crate::{
    sys::{FsEVarId, fsVarsEVarGet, fsVarsGetEVarId},
    vars::{Var, VarKind},
};

pub struct EVarKind;

impl VarKind for EVarKind {
    type Id = FsEVarId;

    #[inline]
    fn register(name: *const std::os::raw::c_char) -> Self::Id {
        unsafe { fsVarsGetEVarId(name) }
    }

    #[inline]
    fn get(
        id: Self::Id,
        unit: crate::sys::FsUnitId,
        _param: crate::sys::FsVarParamArray,
        out: *mut f64,
        _target: crate::sys::FsObjectId,
    ) -> crate::sys::FsVarError {
        unsafe { fsVarsEVarGet(id, unit, out) }
    }

    #[inline]
    fn can_set() -> bool {
        false
    }

    fn set(
        _id: Self::Id,
        _unit: crate::sys::FsUnitId,
        _param: crate::sys::FsVarParamArray,
        _value: f64,
        _target: crate::sys::FsObjectId,
    ) -> crate::sys::FsVarError {
        crate::sys::FsVarError_FS_VAR_ERROR_NOT_SUPPORTED
    }
}

pub type EVar = Var<EVarKind>;
