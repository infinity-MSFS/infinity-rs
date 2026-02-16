use crate::{
    sys::{FsLVarId, fsVarsLVarSet, fsVarsRegisterLVar},
    vars::{Var, VarKind},
};

pub struct LVarKind;

impl VarKind for LVarKind {
    type Id = FsLVarId;

    #[inline]
    fn register(name: *const std::os::raw::c_char) -> Self::Id {
        unsafe { fsVarsRegisterLVar(name) }
    }

    #[inline]
    fn get(id: Self::Id, unit: crate::sys::FsUnitId, out: *mut f64) -> crate::sys::FsVarError {
        unsafe { crate::sys::fsVarsLVarGet(id, unit, out) }
    }

    #[inline]
    fn set(id: Self::Id, unit: crate::sys::FsUnitId, value: f64) -> crate::sys::FsVarError {
        unsafe { fsVarsLVarSet(id, unit, value) }
    }
}

pub type LVar = Var<LVarKind>;
