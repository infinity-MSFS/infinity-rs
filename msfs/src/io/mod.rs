use crate::sys::*;
use std::{
    f32::consts::E,
    ffi::CString,
    os::raw::{c_char, c_void},
    ptr::NonNull,
};

pub mod fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoError {
    Nul(std::ffi::NulError),
    BadParams,
    FileNotFound,
    AccessNotAllowed,
    FileNotOpened,
    ReadNotAllowed,
    PartialReadImpossible,
    OperationImpossible,
    Unknown(u32),
}

impl From<std::ffi::NulError> for IoError {
    fn from(e: std::ffi::NulError) -> Self {
        IoError::Nul(e)
    }
}

impl IoError {
    fn from_raw(code: FsIOErr) -> Option<Self> {
        match code {
            FsIOErr_FsIOErr_Success => None,
            FsIOErr_FsIOErr_BadParams => Some(IoError::BadParams),
            FsIOErr_FsIOErr_FileNotFound => Some(IoError::FileNotFound),
            FsIOErr_FsIOErr_AccessNotAllowed => Some(IoError::AccessNotAllowed),
            FsIOErr_FsIOErr_FileNotOpened => Some(IoError::FileNotOpened),
            FsIOErr_FsIOErr_ReadNotAllowed => Some(IoError::ReadNotAllowed),
            FsIOErr_FsIOErr_PartialReadImpossible => Some(IoError::PartialReadImpossible),
            FsIOErr_FsIOErr_OperationImpossible => Some(IoError::OperationImpossible),
            other => Some(IoError::Unknown(other)),
        }
    }

    fn check(code: FsIOErr) -> IoResult<()> {
        match Self::from_raw(code) {
            None => Ok(()),
            Some(err) => Err(err),
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Nul(e) => write!(f, "null byte in string: {e}"),
            IoError::BadParams => write!(f, "bad parameters"),
            IoError::FileNotFound => write!(f, "file not found"),
            IoError::AccessNotAllowed => write!(f, "access not allowed"),
            IoError::FileNotOpened => write!(f, "file not opened"),
            IoError::ReadNotAllowed => write!(f, "read not allowed"),
            IoError::PartialReadImpossible => write!(f, "partial read impossible"),
            IoError::OperationImpossible => write!(f, "operation impossible"),
            IoError::Unknown(c) => write!(f, "unknown IO error ({c:#X})"),
        }
    }
}

pub type IoResult<T> = Result<T, IoError>;

bitflags::bitflags! {
    pub struct OpenFlags: u32 {
        const NONE    = _FsIOOpenFlags_FsIOOpenFlag_NONE;
        const RDONLY  = _FsIOOpenFlags_FsIOOpenFlag_RDONLY;
        const WRONLY  = _FsIOOpenFlags_FsIOOpenFlag_WRONLY;
        const RDWR    = _FsIOOpenFlags_FsIOOpenFlag_RDWR;
        const CREAT   = _FsIOOpenFlags_FsIOOpenFlag_CREAT;
        const TRUNC   = _FsIOOpenFlags_FsIOOpenFlag_TRUNC;
        const HIDDEN  = _FsIOOpenFlags_FsIOOpenFlag_HIDDEN;
    }
}

struct OpenCb(Box<dyn FnOnce(File) + 'static>);

struct ReadCb(Box<dyn FnOnce(&[u8], i32) + 'static>);

struct WriteCb(Box<dyn FnOnce(i32, i32) + 'static>);

extern "C" fn open_trampoline(file: FsIOFile, user: *mut c_void) {
    if user.is_null() {
        return;
    }

    let cb = unsafe { Box::from_raw(user as *mut OpenCb) };
    (cb.0)(File(file));
}

extern "C" fn read_trampoline(
    _file: FsIOFile,
    buf: *mut c_char,
    byte_offset: i32,
    bytes_read: i32,
    user: *mut c_void,
) {
    if user.is_null() {
        return;
    }
    let cb = unsafe { Box::from_raw(user as *mut ReadCb) };
    let slice = if buf.is_null() || bytes_read <= 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(buf as *const u8, bytes_read as usize) }
    };
    (cb.0)(slice, byte_offset);
}

extern "C" fn write_trampoline(
    _file: FsIOFile,
    _buf: *const c_char,
    byte_offset: i32,
    bytes_written: i32,
    user: *mut c_void,
) {
    if user.is_null() {
        return;
    }
    let cb = unsafe { Box::from_raw(user as *mut WriteCb) };
    (cb.0)(byte_offset, bytes_written);
}

#[derive(Debug)]
pub struct File(FsIOFile);

impl File {
    #[inline]
    pub fn raw(&self) -> FsIOFile {
        self.0
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 as u32 != FS_IO_ERROR_FILE
    }

    #[inline]
    pub fn is_opened(&self) -> bool {
        unsafe { fsIOIsOpened(self.0) }
    }

    #[inline]
    pub fn in_progress(&self) -> bool {
        unsafe { fsIOInProgress(self.0) }
    }

    #[inline]
    pub fn is_done(&self) -> bool {
        unsafe { fsIOIsDone(self.0) }
    }

    #[inline]
    pub fn has_error(&self) -> bool {
        unsafe { fsIOHasError(self.0) }
    }

    pub fn last_error(&self) -> Option<IoError> {
        IoError::from_raw(unsafe { fsIOGetLastError(self.0) })
    }

    #[inline]
    pub fn file_size(&self) -> u64 {
        unsafe { fsIOGetFileSize(self.0) }
    }

    pub fn read(
        &self,
        buf: &mut [u8],
        byte_offset: i32,
        bytes_to_read: i32,
        on_done: impl FnOnce(&[u8], i32) + 'static,
    ) -> IoResult<()> {
        let cb = Box::into_raw(Box::new(ReadCb(Box::new(on_done))));
        let code = unsafe {
            fsIORead(
                self.0,
                buf.as_mut_ptr() as *mut c_char,
                byte_offset,
                bytes_to_read,
                Some(read_trampoline),
                cb as *mut c_void,
            )
        };
        if let Some(e) = IoError::from_raw(code) {
            unsafe {
                drop(Box::from_raw(cb));
            }
            return Err(e);
        }
        Ok(())
    }

    pub fn write(
        &self,
        data: &[u8],
        byte_offset: i32,
        on_done: impl FnOnce(i32, i32) + 'static,
    ) -> IoResult<()> {
        let cb = Box::into_raw(Box::new(WriteCb(Box::new(on_done))));
        let code = unsafe {
            fsIOWrite(
                self.0,
                data.as_ptr() as *const c_char,
                byte_offset,
                data.len() as i32,
                Some(write_trampoline),
                cb as *mut c_void,
            )
        };
        if let Some(e) = IoError::from_raw(code) {
            unsafe {
                drop(Box::from_raw(cb));
            }
            return Err(e);
        }
        Ok(())
    }

    pub fn close(self) -> IoResult<()> {
        let code = unsafe { fsIOClose(self.0) };
        std::mem::forget(self);
        IoError::check(code)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        if self.is_valid() && self.is_opened() {
            let _ = unsafe { fsIOClose(self.0) };
        }
    }
}

pub fn open(path: &str, flags: OpenFlags, on_done: impl FnOnce(File) + 'static) -> IoResult<File> {
    let path_c = CString::new(path)?;
    let cb = Box::into_raw(Box::new(OpenCb(Box::new(on_done))));
    let raw = unsafe {
        fsIOOpen(
            path_c.as_ptr(),
            flags.bits(),
            Some(open_trampoline),
            cb as *mut c_void,
        )
    };
    if raw as u32 == FS_IO_ERROR_FILE {
        unsafe {
            drop(Box::from_raw(cb));
        }
        return Err(IoError::FileNotFound);
    }
    Ok(File(raw))
}

pub fn open_read(
    path: &str,
    flags: OpenFlags,
    byte_offset: i32,
    bytes_to_read: i32,
    on_done: impl FnOnce(&[u8], i32) + 'static,
) -> IoResult<File> {
    let path_c = CString::new(path)?;
    let cb = Box::into_raw(Box::new(ReadCb(Box::new(on_done))));
    let raw = unsafe {
        fsIOOpenRead(
            path_c.as_ptr(),
            flags.bits(),
            byte_offset,
            bytes_to_read,
            Some(read_trampoline),
            cb as *mut c_void,
        )
    };
    if raw as u32 == FS_IO_ERROR_FILE {
        unsafe {
            drop(Box::from_raw(cb));
        }
        return Err(IoError::FileNotFound);
    }
    Ok(File(raw))
}
