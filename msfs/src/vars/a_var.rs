use crate::{
    sys::{FsAVarId, fsVarsAVarSet, fsVarsGetAVarId},
    vars::{Var, VarKind},
};

pub struct AVarKind;

impl VarKind for AVarKind {
    type Id = FsAVarId;

    #[inline]
    fn register(name: *const std::os::raw::c_char) -> Self::Id {
        unsafe { fsVarsGetAVarId(name) }
    }

    #[inline]
    fn get(
        id: Self::Id,
        unit: crate::sys::FsUnitId,
        param: crate::sys::FsVarParamArray,
        out: *mut f64,
        target: crate::sys::FsObjectId,
    ) -> crate::sys::FsVarError {
        unsafe { crate::sys::fsVarsAVarGet(id, unit, param, out, target) }
    }

    #[inline]
    fn set(
        id: Self::Id,
        unit: crate::sys::FsUnitId,
        param: crate::sys::FsVarParamArray,
        value: f64,
        target: crate::sys::FsObjectId,
    ) -> crate::sys::FsVarError {
        unsafe { fsVarsAVarSet(id, unit, param, value, target) }
    }
}

pub type AVar = Var<AVarKind>;
