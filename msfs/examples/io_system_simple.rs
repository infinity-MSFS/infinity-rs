//! for users that want to use a more abstracted API, the io::fs module provides a more rust-like feel for file io while still maintaining most of the power of the full API.
//! this example demonstrates reading and writing files when a L var changes
//!
use msfs::io::fs;
use msfs::prelude::*;

const L_GO: &str = "L:INFINITY_FS_DEMO_GO";
const L_STATUS: &str = "L:INFINITY_FS_DEMO_STATUS";
const L_BYTES: &str = "L:INFINITY_FS_DEMO_BYTES_READ";

const INPUT_PATH: &str = "\\work/hello.txt";
const OUTPUT_PATH: &str = "\\work/hello_copy.txt";

const STATUS_IDLE: f64 = 0.0;
const STATUS_READING: f64 = 1.0;
const STATUS_WRITING: f64 = 2.0;
const STATUS_DONE: f64 = 3.0;
const STATUS_ERROR: f64 = -1.0;

pub struct FsDemoSystem {
    l_go: LVar,
    l_status: LVar,
    l_bytes: LVar,

    read_req: Option<fs::ReadRequest>,
    write_req: Option<fs::WriteRequest>,
}

impl FsDemoSystem {
    pub fn new() -> Self {
        Self {
            l_go: LVar::new(L_GO, "Bool").expect("LVar"),
            l_status: LVar::new(L_STATUS, "Number").expect("LVar"),
            l_bytes: LVar::new(L_BYTES, "Number").expect("LVar"),
            read_req: None,
            write_req: None,
        }
    }
}

impl System for FsDemoSystem {
    fn init(&mut self, _ctx: &Context, _install: &SystemInstall) -> bool {
        let _ = self.l_go.set(0.0);
        let _ = self.l_status.set(STATUS_IDLE);
        let _ = self.l_bytes.set(0.0);
        true
    }
    fn update(&mut self, _ctx: &Context, _dt: f32) -> bool {
        let go = self.l_go.get().unwrap_or(0.0) >= 0.5;
        if go && self.read_req.is_none() && self.write_req.is_none() {
            let _ = self.l_go.set(0.0);

            match fs::read(INPUT_PATH, |data| {
                println!("[fs_demo] read {} bytes", data.len());
            }) {
                Ok(req) => {
                    self.read_req = Some(req);
                    let _ = self.l_status.set(STATUS_READING);
                }
                Err(e) => {
                    println!("[fs_demo] read failed: {e}");
                    let _ = self.l_status.set(STATUS_ERROR);
                }
            }
        }

        if let Some(ref req) = self.read_req {
            if req.has_error() {
                println!("[fs_demo] read error: {:?}", req.last_error());
                let _ = self.l_status.set(STATUS_ERROR);
                self.read_req = None;
            } else if req.is_done() {
                if let Some(data) = req.take_data() {
                    let _ = self.l_bytes.set(data.len() as f64);

                    match fs::write(OUTPUT_PATH, &data) {
                        Ok(wreq) => {
                            self.write_req = Some(wreq);
                            let _ = self.l_status.set(STATUS_WRITING);
                        }
                        Err(e) => {
                            println!("[fs_demo] write failed: {e}");
                            let _ = self.l_status.set(STATUS_ERROR);
                        }
                    }
                }
                self.read_req = None;
            }
        }

        if let Some(ref req) = self.write_req {
            if req.has_error() {
                println!("[fs_demo] write error: {:?}", req.last_error());
                let _ = self.l_status.set(STATUS_ERROR);
                self.write_req = None;
            } else if req.is_done() {
                println!("[fs_demo] copy complete!");
                let _ = self.l_status.set(STATUS_DONE);
                self.write_req = None;
            }
        }

        true
    }

    fn kill(&mut self, ctx: &Context) -> bool {
        self.read_req = None;
        self.write_req = None;
        true
    }
}

msfs::export_system!(
    name = fs_demo,
    state = FsDemoSystem,
    ctor = FsDemoSystem::new()
);
