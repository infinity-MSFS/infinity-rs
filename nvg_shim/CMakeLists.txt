cmake_minimum_required(VERSION 3.26)
project(nvg_shim LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 23)

add_library(nvg_shim SHARED
        shim_impl.cpp
)

# Public headers live in this directory (shim) and in nvg/
target_include_directories(nvg_shim PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${CMAKE_CURRENT_SOURCE_DIR}/nvg
)

# Ensure NanoVG API functions are exported from the DLL on Windows.
target_compile_definitions(nvg_shim PRIVATE NANOVG_DLL_EXPORT)

# NanoVG core implementation is compiled into the shim (single source of truth).
target_sources(nvg_shim PRIVATE nvg/nanovg.cpp)