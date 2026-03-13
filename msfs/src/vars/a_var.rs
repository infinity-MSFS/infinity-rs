use crate::{
    sys::{
        FS_OBJECT_ID_USER_AIRCRAFT, FsAVarId, FsObjectId, FsVarError_FS_VAR_ERROR_NONE,
        FsVarParamArray, fsVarsAVarBufferGet, fsVarsAVarBufferSet, fsVarsAVarSet, fsVarsGetAVarId,
    },
    vars::{UnitId, Var, VarError, VarKind, VarParamArray1, VarResult, empty_param_array},
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

impl AVar {
    #[inline]
    pub fn new_string(name: &str) -> VarResult<Self> {
        Self::new(name, "String")
    }

    #[inline]
    pub fn get_buffer_into(&self, target: FsObjectId, buffer: &mut [u8]) -> VarResult<()> {
        self.get_buffer_into_with(empty_param_array(), target, buffer)
    }

    #[inline]
    pub fn get_buffer_into_target(&self, target: FsObjectId, buffer: &mut [u8]) -> VarResult<()> {
        self.get_buffer_into_with(empty_param_array(), target, buffer)
    }

    #[inline]
    pub fn get_buffer_into_indexed(&self, index: u32, buffer: &mut [u8]) -> VarResult<()> {
        self.get_buffer_into_indexed_target(index, FS_OBJECT_ID_USER_AIRCRAFT, buffer)
    }

    #[inline]
    pub fn get_buffer_into_indexed_target(
        &self,
        index: u32,
        target: FsObjectId,
        buffer: &mut [u8],
    ) -> VarResult<()> {
        let mut param = VarParamArray1::index(index);
        self.get_buffer_into_with(param.as_raw_mut(), target, buffer)
    }

    #[inline]
    pub fn get_buffer_into_with(
        &self,
        param: FsVarParamArray,
        target: FsObjectId,
        buffer: &mut [u8],
    ) -> VarResult<()> {
        let err = unsafe {
            fsVarsAVarBufferGet(
                self.raw_id(),
                self.unit().0,
                param,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as i32,
                target,
            )
        };
        if err == FsVarError_FS_VAR_ERROR_NONE {
            Ok(())
        } else {
            Err(VarError::Fs(err))
        }
    }

    #[inline]
    pub fn get_string(&self, capacity: usize) -> VarResult<String> {
        self.get_string_with(empty_param_array(), FS_OBJECT_ID_USER_AIRCRAFT, capacity)
    }

    #[inline]
    pub fn get_string_target(&self, target: FsObjectId, capacity: usize) -> VarResult<String> {
        self.get_string_with(empty_param_array(), target, capacity)
    }

    #[inline]
    pub fn get_string_indexed(&self, index: u32, capacity: usize) -> VarResult<String> {
        self.get_string_indexed_target(index, FS_OBJECT_ID_USER_AIRCRAFT, capacity)
    }

    #[inline]
    pub fn get_string_indexed_target(
        &self,
        index: u32,
        target: FsObjectId,
        capacity: usize,
    ) -> VarResult<String> {
        let mut param = VarParamArray1::index(index);
        self.get_string_with(param.as_raw_mut(), target, capacity)
    }

    #[inline]
    pub fn get_string_with(
        &self,
        params: FsVarParamArray,
        target: FsObjectId,
        capacity: usize,
    ) -> VarResult<String> {
        let mut buf = vec![0u8; capacity.max(1)];
        self.get_buffer_into_with(params, target, &mut buf)?;

        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Ok(String::from_utf8_lossy(&buf[..end]).into_owned())
    }

    #[inline]
    pub fn set_buffer(&self, buffer: &[u8]) -> VarResult<()> {
        self.set_buffer_with(empty_param_array(), FS_OBJECT_ID_USER_AIRCRAFT, buffer)
    }

    #[inline]
    pub fn set_buffer_target(&self, target: FsObjectId, buffer: &[u8]) -> VarResult<()> {
        self.set_buffer_with(empty_param_array(), target, buffer)
    }

    #[inline]
    pub fn set_buffer_indexed(&self, index: u32, buffer: &[u8]) -> VarResult<()> {
        self.set_buffer_indexed_target(index, FS_OBJECT_ID_USER_AIRCRAFT, buffer)
    }

    #[inline]
    pub fn set_buffer_indexed_target(
        &self,
        index: u32,
        target: FsObjectId,
        buffer: &[u8],
    ) -> VarResult<()> {
        let mut params = VarParamArray1::index(index);
        self.set_buffer_with(params.as_raw_mut(), target, buffer)
    }

    #[inline]
    pub fn set_buffer_with(
        &self,
        param: FsVarParamArray,
        target: FsObjectId,
        buffer: &[u8],
    ) -> VarResult<()> {
        let err = unsafe {
            fsVarsAVarBufferSet(
                self.raw_id(),
                self.unit().0,
                param,
                buffer.as_ptr() as *mut _,
                buffer.len() as i32,
                target,
            )
        };

        if err == FsVarError_FS_VAR_ERROR_NONE {
            Ok(())
        } else {
            Err(VarError::Fs(err))
        }
    }

    #[inline]
    pub fn set_string(&self, value: &str) -> VarResult<()> {
        self.set_buffer(value.as_bytes())
    }

    #[inline]
    pub fn set_string_target(&self, target: FsObjectId, value: &str) -> VarResult<()> {
        self.set_buffer_target(target, value.as_bytes())
    }

    #[inline]
    pub fn set_string_indexed(&self, index: u32, value: &str) -> VarResult<()> {
        self.set_buffer_indexed(index, value.as_bytes())
    }

    #[inline]
    pub fn set_string_indexed_target(
        &self,
        index: u32,
        target: FsObjectId,
        value: &str,
    ) -> VarResult<()> {
        self.set_buffer_indexed_target(index, target, value.as_bytes())
    }

    #[inline]
    pub fn set_c_string(&self, value: &str) -> VarResult<()> {
        let mut bytes = Vec::with_capacity(value.len() + 1);
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        self.set_buffer(&bytes)
    }

    #[inline]
    pub fn is_string_unit(&self) -> bool {
        self.unit() == UnitId::from_str("String").unwrap_or(UnitId(-1))
    }
}
