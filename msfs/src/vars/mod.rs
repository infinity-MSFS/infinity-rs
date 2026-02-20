pub mod a_var;
pub mod l_var;

pub use a_var::AVar;
pub use l_var::LVar;

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

    fn get(
        id: Self::Id,
        unit: FsUnitId,
        param: FsVarParamArray,
        out: *mut f64,
        target: FsObjectId,
    ) -> FsVarError;

    fn set(
        id: Self::Id,
        unit: FsUnitId,
        param: FsVarParamArray,
        value: f64,
        target: FsObjectId,
    ) -> FsVarError;

    fn default_target() -> FsObjectId {
        FS_OBJECT_ID_USER_AIRCRAFT
    }

    fn can_set() -> bool {
        true
    }
}

#[inline]
pub fn empty_param_array() -> FsVarParamArray {
    FsVarParamArray {
        size: 0,
        array: core::ptr::null_mut(),
    }
}

#[derive(Copy, Clone)]
pub struct VarParamArray1 {
    variant: FsVarParamVariant,
}

impl VarParamArray1 {
    #[inline]
    pub fn index(index: u32) -> Self {
        let mut v: FsVarParamVariant = unsafe { core::mem::zeroed() };
        v.type_ = eFsVarParamType_FsVarParamTypeInteger;
        v.__bindgen_anon_1 = FsVarParamVariant__bindgen_ty_1 { intValue: index };
        Self { variant: v }
    }

    #[inline]
    pub fn as_raw_mut(&mut self) -> FsVarParamArray {
        FsVarParamArray {
            size: 1,
            array: &mut self.variant as *mut _,
        }
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
        self.get_with(empty_param_array(), K::default_target())
    }

    #[inline]
    pub fn get_target(&self, target: FsObjectId) -> VarResult<f64> {
        self.get_with(empty_param_array(), target)
    }

    #[inline]
    pub fn get_with(&self, param: FsVarParamArray, target: FsObjectId) -> VarResult<f64> {
        let mut out = MaybeUninit::<f64>::uninit();
        let err = K::get(self.id, self.unit.0, param, out.as_mut_ptr(), target);
        if err == FsVarError_FS_VAR_ERROR_NONE {
            Ok(unsafe { out.assume_init() })
        } else {
            Err(VarError::Fs(err))
        }
    }

    #[inline]
    pub fn get_indexed(&self, index: u32) -> VarResult<f64> {
        self.get_indexed_target(index, K::default_target())
    }

    #[inline]
    pub fn get_indexed_target(&self, index: u32, target: FsObjectId) -> VarResult<f64> {
        let mut param = VarParamArray1::index(index);
        self.get_with(param.as_raw_mut(), target)
    }

    #[inline]
    pub fn set(&self, value: f64) -> VarResult<()> {
        self.set_with(empty_param_array(), value, K::default_target())
    }

    #[inline]
    pub fn set_target(&self, target: FsObjectId, value: f64) -> VarResult<()> {
        self.set_with(empty_param_array(), value, target)
    }

    #[inline]
    pub fn set_with(
        &self,
        param: FsVarParamArray,
        value: f64,
        target: FsObjectId,
    ) -> VarResult<()> {
        if !K::can_set() {
            return Err(VarError::Fs(FsVarError_FS_VAR_ERROR_NOT_SUPPORTED));
        }
        let err = K::set(self.id, self.unit.0, param, value, target);
        if err == FsVarError_FS_VAR_ERROR_NONE {
            Ok(())
        } else {
            Err(VarError::Fs(err))
        }
    }

    #[inline]
    pub fn set_indexed(&self, index: u32, value: f64) -> VarResult<()> {
        self.set_indexed_target(index, K::default_target(), value)
    }

    #[inline]
    pub fn set_indexed_target(&self, index: u32, target: FsObjectId, value: f64) -> VarResult<()> {
        let mut param = VarParamArray1::index(index);
        self.set_with(param.as_raw_mut(), value, target)
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
