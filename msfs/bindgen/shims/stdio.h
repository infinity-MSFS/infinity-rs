// Intentionally empty.
//
// This file shadows <stdio.h> on bindgen's include path so SDK headers
// that transitively `#include <stdio.h>` (MSFS_WindowsTypes.h, stb_*,
// SimParamArrayHelper.h) resolve to nothing instead of the full libc
// header. Without this shim bindgen would generate the entire FILE/
// fpos_t/printf surface into msfs-sys.rs.
//
// Do not delete. Do not add declarations.
