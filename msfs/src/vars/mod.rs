pub mod a_var;
pub mod l_var;

use crate::sys::*;

use std::{ffi::CString, marker::PhantomData, mem::MaybeUninit, os::raw::c_char};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VarError {
    Fs(FsVarError),
    Nul(std::ffi::NulError),
}

impl From<std::ffi::NulError> for VarError {
    fn from(e: std::ffi::NulError) -> Self {
        VarError::Nul(e)
    }
}

pub type VarResult<T> = Result<T, VarError>;

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct UnitId(pub FsUnitId);

impl UnitId {
    pub fn from_str(unit: &str) -> VarResult<Self> {
        let unit_c = CString::new(unit)?;
        let id = unsafe { fsVarsGetUnitId(unit_c.as_ptr() as *const c_char) };
        Ok(UnitId(id))
    }
}

pub trait VarKind {
    type Id: Copy;

    fn register(name: *const c_char) -> Self::Id;

    fn get(id: Self::Id, unit: FsUnitId, out: *mut f64) -> FsVarError;

    fn set(id: Self::Id, unit: FsUnitId, value: f64) -> FsVarError;

    fn can_set() -> bool {
        true
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Var<K: VarKind> {
    id: K::Id,
    unit: UnitId,
    _k: PhantomData<K>,
}

impl<K: VarKind> Var<K> {
    pub fn new(name: &str, unit: &str) -> VarResult<Self> {
        let name_c = CString::new(name)?;
        let unit = UnitId::from_str(unit)?;
        let id = K::register(name_c.as_ptr() as *const c_char);
        Ok(Self {
            id,
            unit,
            _k: PhantomData,
        })
    }

    #[inline]
    pub fn get(&self) -> VarResult<f64> {
        let mut out = MaybeUninit::<f64>::uninit();
        let err = K::get(self.id, self.unit.0, out.as_mut_ptr());
        if err == FsVarError_FS_VAR_ERROR_NONE {
            Ok(unsafe { out.assume_init() })
        } else {
            Err(VarError::Fs(err))
        }
    }

    #[inline]
    pub fn set(&self, value: f64) -> VarResult<()> {
        if !K::can_set() {
            return Err(VarError::Fs(FsVarError_FS_VAR_ERROR_NOT_SUPPORTED));
        }
        let err = unsafe { K::set(self.id, self.unit.0, value) };
        if err == FsVarError_FS_VAR_ERROR_NONE {
            Ok(())
        } else {
            Err(VarError::Fs(err))
        }
    }

    #[inline]
    pub fn unit(&self) -> UnitId {
        self.unit
    }

    #[inline]
    pub fn raw_id(&self) -> K::Id {
        self.id
    }
}
