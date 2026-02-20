use msfs::io::{self, File, OpenFlags};
use msfs::prelude::*;

const L_ENABLED: &str = "L:INFINITY_IO_DEMO_ENABLED";
const L_DO_READ: &str = "L:INFINITY_IO_DEMO_DO_READ";
const L_DO_WRITE: &str = "L:INFINITY_IO_DEMO_DO_WRITE";

const L_OUT_READ_SIZE: &str = "L:INFINITY_IO_DEMO_READ_SIZE";
const L_OUT_WRITE_SIZE: &str = "L:INFINITY_IO_DEMO_WRITE_SIZE";
const L_OUT_FILE_SIZE: &str = "L:INFINITY_IO_DEMO_FILE_SIZE";
const L_OUT_IS_OPENED: &str = "L:INFINITY_IO_DEMO_IS_OPENED";
const L_OUT_IS_DONE: &str = "L:INFINITY_IO_DEMO_IS_DONE";
const L_OUT_HAS_ERROR: &str = "L:INFINITY_IO_DEMO_HAS_ERROR";

const READ_PATH: &str = "\\work/demo_input.txt";
const WRITE_PATH: &str = "\\work/demo_output.txt";

pub struct IoFullApiSystem {
    l_enabled: LVar,
    l_do_read: LVar,
    l_do_write: LVar,

    l_out_read_size: LVar,
    l_out_write_size: LVar,
    l_out_file_size: LVar,
    l_out_is_opened: LVar,
    l_out_is_done: LVar,
    l_out_has_error: LVar,

    read_file: Option<File>,
    write_file: Option<File>,

    last_read: Vec<u8>,

    accum: f32,
}

impl IoFullApiSystem {
    pub fn new() -> Self {
        let l_enabled = LVar::new(L_ENABLED, "Bool").expect("LVar create failed");
        let l_do_read = LVar::new(L_DO_READ, "Bool").expect("LVar create failed");
        let l_do_write = LVar::new(L_DO_WRITE, "Bool").expect("LVar create failed");

        let l_out_read_size = LVar::new(L_OUT_READ_SIZE, "Number").expect("LVar create failed");
        let l_out_write_size = LVar::new(L_OUT_WRITE_SIZE, "Number").expect("LVar create failed");
        let l_out_file_size = LVar::new(L_OUT_FILE_SIZE, "Number").expect("LVar create failed");
        let l_out_is_opened = LVar::new(L_OUT_IS_OPENED, "Bool").expect("LVar create failed");
        let l_out_is_done = LVar::new(L_OUT_IS_DONE, "Bool").expect("LVar create failed");
        let l_out_has_error = LVar::new(L_OUT_HAS_ERROR, "Bool").expect("LVar create failed");

        Self {
            l_enabled,
            l_do_read,
            l_do_write,
            l_out_read_size,
            l_out_write_size,
            l_out_file_size,
            l_out_is_opened,
            l_out_is_done,
            l_out_has_error,
            read_file: None,
            write_file: None,
            last_read: Vec::new(),
            accum: 0.0,
        }
    }

    /// Kick off an async open-and-read via `io::open_read`.
    fn start_read(&mut self) {
        self.read_file = None;

        match io::open_read(READ_PATH, OpenFlags::RDONLY, 0, -1, |data, _offset| {
            println!("[io_demo] read callback: {} bytes", data.len());
        }) {
            Ok(file) => {
                println!(
                    "[io_demo] open_read started, file size = {}",
                    file.file_size()
                );
                let _ = self.l_out_file_size.set(file.file_size() as f64);
                self.read_file = Some(file);
            }
            Err(e) => {
                println!("[io_demo] open_read failed: {e}");
                let _ = self.l_out_has_error.set(1.0);
            }
        }
    }

    /// Open a file, then issue an async read on the returned handle.
    fn start_read_two_step(&mut self) {
        self.read_file = None;

        match io::open(READ_PATH, OpenFlags::RDONLY, |file| {
            println!("[io_demo] open callback, file size = {}", file.file_size());
        }) {
            Ok(file) => {
                let _ = self.l_out_file_size.set(file.file_size() as f64);

                let size = file.file_size() as usize;
                let mut buf = vec![0u8; size];
                let res = file.read(&mut buf, 0, size as i32, |data, _offset| {
                    println!("[io_demo] read callback (two-step): {} bytes", data.len());
                });

                if let Err(e) = res {
                    println!("[io_demo] read failed: {e}");
                    let _ = self.l_out_has_error.set(1.0);
                }

                self.read_file = Some(file);
            }
            Err(e) => {
                println!("[io_demo] open failed: {e}");
                let _ = self.l_out_has_error.set(1.0);
            }
        }
    }

    /// Write `last_read` data to the output file.
    fn start_write(&mut self) {
        self.write_file = None;

        if self.last_read.is_empty() {
            println!("[io_demo] nothing to write (read first!)");
            return;
        }

        let payload = self.last_read.clone();

        match io::open(
            WRITE_PATH,
            OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC,
            |_file| {
                println!("[io_demo] write-file open callback");
            },
        ) {
            Ok(file) => {
                let len = payload.len();
                let res = file.write(&payload, 0, move |_offset, written| {
                    println!("[io_demo] write callback: {written} bytes written");
                });

                if let Err(e) = res {
                    println!("[io_demo] write failed: {e}");
                    let _ = self.l_out_has_error.set(1.0);
                } else {
                    let _ = self.l_out_write_size.set(len as f64);
                }

                self.write_file = Some(file);
            }
            Err(e) => {
                println!("[io_demo] open-for-write failed: {e}");
                let _ = self.l_out_has_error.set(1.0);
            }
        }
    }

    fn update_status(&mut self) {
        if let Some(ref f) = self.read_file {
            let _ = self
                .l_out_is_opened
                .set(if f.is_opened() { 1.0 } else { 0.0 });
            let _ = self.l_out_is_done.set(if f.is_done() { 1.0 } else { 0.0 });
            let _ = self
                .l_out_has_error
                .set(if f.has_error() { 1.0 } else { 0.0 });

            // Once done, drop the handle (closes the file).
            if f.is_done() || f.has_error() {
                self.read_file = None;
            }
        }

        if let Some(ref f) = self.write_file {
            if f.is_done() || f.has_error() {
                if f.has_error() {
                    println!("[io_demo] write file error: {:?}", f.last_error());
                }
                self.write_file = None;
            }
        }
    }

    fn tick(&mut self) {
        let do_read = self.l_do_read.get().unwrap_or(0.0) >= 0.5;
        if do_read && self.read_file.is_none() {
            let _ = self.l_do_read.set(0.0);
            self.start_read();
        }

        let do_write = self.l_do_write.get().unwrap_or(0.0) >= 0.5;
        if do_write && self.write_file.is_none() {
            let _ = self.l_do_write.set(0.0);
            self.start_write();
        }
        self.update_status();
    }
}

impl System for IoFullApiSystem {
    fn init(&mut self, _ctx: &Context, _install: &SystemInstall) -> bool {
        let _ = self.l_enabled.set(1.0);
        let _ = self.l_do_read.set(0.0);
        let _ = self.l_do_write.set(0.0);
        let _ = self.l_out_read_size.set(0.0);
        let _ = self.l_out_write_size.set(0.0);
        let _ = self.l_out_file_size.set(0.0);
        let _ = self.l_out_is_opened.set(0.0);
        let _ = self.l_out_is_done.set(0.0);
        let _ = self.l_out_has_error.set(0.0);
        true
    }

    fn update(&mut self, _ctx: &Context, dt: f32) -> bool {
        self.accum += dt;

        if self.accum >= 0.25 {
            self.accum = 0.0;
            let enabled = self.l_enabled.get().unwrap_or(0.0) >= 0.5;
            if enabled {
                self.tick();
            }
        }

        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        self.read_file = None;
        self.write_file = None;
        true
    }
}

msfs::export_system!(
    name = io_full_api,
    state = IoFullApiSystem,
    ctor = IoFullApiSystem::new()
);
