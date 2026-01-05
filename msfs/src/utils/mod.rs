use crate::sys::{
    FsCRC, FsVarParamArray, FsVarParamVariant, FsVarParamVariant__bindgen_ty_1, eFsVarParamType,
    eFsVarParamType_FsVarParamTypeCRC, eFsVarParamType_FsVarParamTypeDouble,
    eFsVarParamType_FsVarParamTypeInteger, eFsVarParamType_FsVarParamTypeString,
};
use core::{ffi::c_char, ptr, slice};
use std::mem;

#[derive(Debug, Copy, Clone)]
pub enum FsParamArg {
    Crc(FsCRC),
    Str(*const c_char),
    Index(u32),
    Double(f64),
}

#[derive(Debug)]
pub enum FsParamError {
    ArgCountMismatch { fmt_len: usize, args_len: usize },
    UnknonwnFormatChar { ch: char, index: usize },
    UnknonwnError { msg: String },
}

pub struct FsVarParamArrayOwned {
    raw: FsVarParamArray,
}
impl FsVarParamArrayOwned {
    #[inline]
    pub fn as_raw(&self) -> FsVarParamArray {
        self.raw
    }

    #[inline]
    pub fn as_raw_ptr(&self) -> *const FsVarParamArray {
        &self.raw as *const _
    }

    #[inline]
    pub fn as_raw_mut_ptr(&mut self) -> *mut FsVarParamArray {
        &mut self.raw as *mut _
    }
}

/// Prefer this over manually calling fs_destroy_param_array
impl Drop for FsVarParamArrayOwned {
    fn drop(&mut self) {
        unsafe {
            let len = self.raw.size as usize;
            if len != 0 && !self.raw.array.is_null() {
                let slice = slice::from_raw_parts_mut(self.raw.array, len);
                drop(Box::from_raw(slice));
            }
            self.raw.size = 0;
            self.raw.array = std::ptr::null_mut();
        }
    }
}

/// Prefer to use FsVarParamArrayOwned which automatically cleans up the array on drop.
pub unsafe fn fs_destroy_param_array(p: &mut FsVarParamArray) {
    let len = p.size as usize;
    if len != 0 && !p.array.is_null() {
        let slice = slice::from_raw_parts_mut(p.array, len);
        drop(Box::from_raw(slice));
    }
    p.size = 0;
    p.array = ptr::null_mut();
}

#[inline]
fn make_variant(ch: char, arg: FsParamArg) -> Result<FsVarParamVariant, char> {
    let mut var: FsVarParamVariant = unsafe { mem::zeroed() };

    match (ch, arg) {
        ('c', FsParamArg::Crc(x)) => {
            var.type_ = eFsVarParamType_FsVarParamTypeCRC;
            var.__bindgen_anon_1 = FsVarParamVariant__bindgen_ty_1 { CRCValue: x };
        }
        ('s', FsParamArg::Str(p)) => {
            var.type_ = eFsVarParamType_FsVarParamTypeString;
            var.__bindgen_anon_1 = FsVarParamVariant__bindgen_ty_1 { stringValue: p };
        }
        ('i', FsParamArg::Index(x)) => {
            var.type_ = eFsVarParamType_FsVarParamTypeInteger;
            var.__bindgen_anon_1 = FsVarParamVariant__bindgen_ty_1 { intValue: x };
        }
        ('f', FsParamArg::Double(x)) => {
            var.type_ = eFsVarParamType_FsVarParamTypeDouble;
            var.__bindgen_anon_1 = FsVarParamVariant__bindgen_ty_1 { doubleValue: x };
        }
        _ => return Err(ch),
    }

    Ok(var)
}

pub fn fs_create_param_array(
    fmt: &str,
    args: &[FsParamArg],
) -> Result<FsVarParamArrayOwned, FsParamError> {
    let fmt_len = fmt.chars().count();
    if fmt_len != args.len() {
        return Err(FsParamError::ArgCountMismatch {
            fmt_len,
            args_len: args.len(),
        });
    }
    let mut v: Vec<FsVarParamVariant> = Vec::with_capacity(fmt_len);

    for (ch, arg) in fmt.chars().zip(args.iter().copied()) {
        let var = make_variant(ch, arg)
            .map_err(|bad| bad)
            .map_err(|e| FsParamError::UnknonwnError { msg: e.to_string() })?;
        v.push(var);
    }

    let mut boxed: Box<[FsVarParamVariant]> = v.into_boxed_slice();
    let raw = FsVarParamArray {
        size: boxed.len() as _,
        array: boxed.as_mut_ptr(),
    };
    core::mem::forget(boxed);

    Ok(FsVarParamArrayOwned { raw })
}
