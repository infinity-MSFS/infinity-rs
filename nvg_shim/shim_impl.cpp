#define NANOVG_SW_IMPLEMENTATION

//#define IDE_INCLUDES
#include "nvg/nanovg.h"
#include "nvg/nanovg_sw.h"
///


#if defined(_WIN32)
#define SHIM_EXPORT extern "C" __declspec(dllexport)
#else
#define SHIM_EXPORT extern "C" __attribute__((visibility("default")))
#endif

struct ShimCtx {
    NVGcontext* nvg;
    void* fb = nullptr;
    int w = 0, h = 0;
    int rshift = 0, gshift = 0, bshift = 0, ashift = 0;
};

SHIM_EXPORT ShimCtx* shim_create(int flags) {
    auto* s = new ShimCtx();
    s->nvg = nvgswCreate(flags);
    return s;

}


SHIM_EXPORT void shim_delete(ShimCtx* s) {
   if (!s) return;
    if (s->nvg) nvgswDelete(s->nvg);
    delete s;
}

SHIM_EXPORT NVGcontext* shim_nvg(ShimCtx* s) {
    return s? s->nvg : nullptr;
}

SHIM_EXPORT void shim_set_framebuffer(ShimCtx* s, void* dest, int w, int h, int rshift, int gshift, int bshift, int ashift) {
    if (!s) return;
    s->fb = dest;
    s->w = w;
    s->h = h;
    s->rshift = rshift;
    s->gshift = gshift;
    s->bshift = bshift;
    s->ashift = ashift;
    nvgswSetFramebuffer(s->nvg, dest, w, h, rshift, gshift, bshift, ashift);
}

SHIM_EXPORT void shim_set_framebuffer_rgba8888(ShimCtx* s, void* dest, int w, int h) {
    shim_set_framebuffer(s, dest, w, h, 0, 8, 16, 24);
}

SHIM_EXPORT void* shim_get_framebuffer(ShimCtx* s) {
    return s ? s->fb : nullptr;
}