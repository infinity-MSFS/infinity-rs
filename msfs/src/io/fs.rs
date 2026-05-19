//! abstractions for file IO to feel more like rust and less like the raw C API
//! # Examples
//! ```no_run
//! use infinity_rs::fs::{self, ReadRequest, WriteRequest};
//!
//! // Fire-and-forget read
//! let req = fs::read("\\work/config.json", |data| {
//!     let text = String::from_utf8_lossy(&data);
//!     infinity_rs::log!("got config: {text}");
//! })?;
//!
//! // Poll in your update loop
//! if req.is_done() { /* ... */ }
//!
//! // One-liner write
//! let req = fs::write("\\work/output.txt", b"hello world")?;
//! ```
//!

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    InProgress,
    Done,
    Error,
}

pub struct ReadRequest {
    file: File,
    result: Rc<RefCell<Option<Vec<u8>>>>,
    /// Buffer MSFS reads into. Kept alive (and un-reallocated) for the whole
    /// async read by holding it here until the request is dropped.
    #[allow(dead_code)]
    buf: Rc<RefCell<Vec<u8>>>,
}

impl ReadRequest {
    pub fn status(&self) -> RequestStatus {
        if self.file.has_error() {
            RequestStatus::Error
        } else if self.file.is_done() {
            RequestStatus::Done
        } else {
            RequestStatus::InProgress
        }
    }

    #[inline]
    pub fn is_done(&self) -> bool {
        self.status() == RequestStatus::Done
    }

    #[inline]
    pub fn has_error(&self) -> bool {
        self.status() == RequestStatus::Error
    }

    pub fn last_error(&self) -> Option<IoError> {
        self.file.last_error()
    }

    pub fn file_size(&self) -> u64 {
        self.file.file_size()
    }

    pub fn take_data(&self) -> Option<Vec<u8>> {
        self.result.borrow_mut().take()
    }

    pub fn take_string(&self) -> Option<Result<String, std::string::FromUtf8Error>> {
        self.take_data().map(String::from_utf8)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WriteOutcome {
    pub byte_offset: i32,
    pub bytes_written: i32,
}

pub struct WriteRequest {
    file: File,
    outcome: Rc<RefCell<Option<WriteOutcome>>>,
    /// Keeps the write buffer alive for the whole async write. MSFS reads
    /// from the pointer handed to `fsIOWrite` *after* the call returns, so the
    /// buffer must outlive the operation, not merely the `write` call.
    #[allow(dead_code)]
    buf: Rc<Vec<u8>>,
}

impl WriteRequest {
    pub fn status(&self) -> RequestStatus {
        if self.file.has_error() {
            RequestStatus::Error
        } else if self.file.is_done() {
            RequestStatus::Done
        } else {
            RequestStatus::InProgress
        }
    }

    #[inline]
    pub fn is_done(&self) -> bool {
        self.status() == RequestStatus::Done
    }

    #[inline]
    pub fn has_error(&self) -> bool {
        self.status() == RequestStatus::Error
    }

    pub fn last_error(&self) -> Option<IoError> {
        self.file.last_error()
    }

    pub fn take_outcome(&self) -> Option<WriteOutcome> {
        self.outcome.borrow_mut().take()
    }
}

pub fn read(path: &str, on_done: impl FnOnce(&[u8]) + 'static) -> IoResult<ReadRequest> {
    let result: Rc<RefCell<Option<Vec<u8>>>> = Rc::new(RefCell::new(None));
    // MSFS reads into this buffer; it must stay alive and not reallocate for
    // the whole async read, so the ReadRequest keeps an Rc to it.
    let buf: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));

    let result_cb = Rc::clone(&result);
    let buf_cb = Rc::clone(&buf);

    // Open, then read exactly `file_size` bytes. The combined open+read
    // (`open_read`) with `bytes_to_read = -1` never completes on MSFS, so we
    // size the buffer from the opened handle and issue an explicit read.
    let file = crate::io::open(path, OpenFlags::RDONLY, move |file: &File| {
        let size = file.file_size() as usize;
        if size == 0 {
            // Empty file (or a failed open reporting size 0) — deliver an
            // empty result rather than issuing a zero-length read.
            *result_cb.borrow_mut() = Some(Vec::new());
            on_done(&[]);
            return;
        }

        buf_cb.borrow_mut().resize(size, 0);
        let result_inner = Rc::clone(&result_cb);
        let mut b = buf_cb.borrow_mut();
        let _ = file.read(b.as_mut_slice(), 0, size as i32, move |data, _offset| {
            *result_inner.borrow_mut() = Some(data.to_vec());
            on_done(data);
        });
    })?;

    Ok(ReadRequest { file, result, buf })
}

pub fn read_to_string(
    path: &str,
    on_done: impl FnOnce(Result<&str, std::str::Utf8Error>) + 'static,
) -> IoResult<ReadRequest> {
    read(path, move |data| on_done(std::str::from_utf8(data)))
}

pub fn write(path: &str, data: &[u8]) -> IoResult<WriteRequest> {
    write_impl(
        path,
        data,
        OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC,
        0,
    )
}

pub fn append(path: &str, data: &[u8]) -> IoResult<WriteRequest> {
    let outcome: Rc<RefCell<Option<WriteOutcome>>> = Rc::new(RefCell::new(None));
    let buf: Rc<Vec<u8>> = Rc::new(data.to_vec());

    let outcome_clone = Rc::clone(&outcome);
    let buf_clone = Rc::clone(&buf);
    let file = crate::io::open(path, OpenFlags::WRONLY | OpenFlags::CREAT, move |file| {
        let offset = file.file_size() as i32;
        let oc = Rc::clone(&outcome_clone);
        let _ = file.write(&buf_clone, offset, move |off, written| {
            *oc.borrow_mut() = Some(WriteOutcome {
                byte_offset: off,
                bytes_written: written,
            });
        });
    })?;

    Ok(WriteRequest { file, outcome, buf })
}

pub fn create_new(path: &str, data: &[u8]) -> IoResult<WriteRequest> {
    write_impl(path, data, OpenFlags::WRONLY | OpenFlags::CREAT, 0)
}

pub fn open(path: &str, flags: OpenFlags) -> IoResult<FileHandle> {
    let ready = Rc::new(RefCell::new(false));
    let ready_clone = Rc::clone(&ready);

    let file = crate::io::open(path, flags, move |_file| {
        *ready_clone.borrow_mut() = true;
    })?;

    Ok(FileHandle { file, ready })
}

pub struct FileHandle {
    file: File,
    ready: Rc<RefCell<bool>>,
}

impl FileHandle {
    pub fn is_ready(&self) -> bool {
        *self.ready.borrow()
    }

    pub fn status(&self) -> RequestStatus {
        if self.file.has_error() {
            RequestStatus::Error
        } else if self.file.is_done() {
            RequestStatus::Done
        } else {
            RequestStatus::InProgress
        }
    }

    pub fn file_size(&self) -> u64 {
        self.file.file_size()
    }

    pub fn last_error(&self) -> Option<IoError> {
        self.file.last_error()
    }

    pub fn read(
        &self,
        buf: &mut [u8],
        offset: i32,
        len: i32,
        on_done: impl FnOnce(&[u8], i32) + 'static,
    ) -> IoResult<()> {
        self.file.read(buf, offset, len, on_done)
    }

    pub fn write(
        &self,
        data: &[u8],
        offset: i32,
        on_done: impl FnOnce(i32, i32) + 'static,
    ) -> IoResult<()> {
        self.file.write(data, offset, on_done)
    }

    pub fn close(self) -> IoResult<()> {
        self.file.close()
    }
}

fn write_impl(path: &str, data: &[u8], flags: OpenFlags, offset: i32) -> IoResult<WriteRequest> {
    let outcome: Rc<RefCell<Option<WriteOutcome>>> = Rc::new(RefCell::new(None));
    let buf: Rc<Vec<u8>> = Rc::new(data.to_vec());

    let outcome_clone = Rc::clone(&outcome);
    let buf_clone = Rc::clone(&buf);
    let file = crate::io::open(path, flags, move |file| {
        let oc = Rc::clone(&outcome_clone);
        let _ = file.write(&buf_clone, offset, move |off, written| {
            *oc.borrow_mut() = Some(WriteOutcome {
                byte_offset: off,
                bytes_written: written,
            });
        });
    })?;

    Ok(WriteRequest { file, outcome, buf })
}
