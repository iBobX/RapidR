/*
 * Windows XP: DWriteCreateFactory, for a Windows that has no DirectWrite (lane WINXP, 2026-11-05).
 * Compiled ONLY into the Windows XP flavour (tools/winxp/build.sh).
 *
 * Why. RapidR's text system (crates/rapidr-ui-proto/src/text.rs) asks the font library (fontique) for
 * the system's fonts as well as registering RapidR's own Liberation fonts. On Windows fontique asks
 * DirectWrite, and it unwraps the answer: on plain XP (no dwrite.dll) "not available" stops every
 * RapidR window and RapidR Studio with a panic (seen on the XP VM:
 * "fontique-0.11.1/src/backend/dwrite.rs:54 ... HRESULT(0x80004001) Not implemented").
 *
 * What this does. Windows' own DWriteCreateFactory first (Windows 7+, Vista SP2 with the platform
 * update, XP with One-Core-API's). Where there is none, it hands out an EMPTY factory: a static object
 * whose system font collection has no families. fontique then finds no system fonts and RapidR draws
 * with its own bundled fonts (the same faces as everywhere else). Every other method of the two
 * interfaces answers E_NOTIMPL (each pops its own arguments: x86 stdcall), and QueryInterface gives
 * only IUnknown and the interface itself (so fontique's IDWriteFactory2 font fallback is "none").
 * The objects are static: AddRef / Release only count, nothing is ever freed.
 *
 * Written from the interfaces' public definitions: Microsoft Learn's IDWriteFactory and
 * IDWriteFontCollection pages (methods, parameters), and the method order and interface IDs as the
 * `windows` crate declares them (windows-0.62.2, MIT/Apache-2.0; fontique uses it). No DirectWrite code
 * of anyone else (Wine, ReactOS, One-Core-API, ...) was read or copied. 32-bit x86 only.
 */
#include "xp_common.h"

typedef struct { unsigned long a; unsigned short b, c; unsigned char d[8]; } GUID;

#define E_NOINTERFACE ((HRESULT)0x80004002L)
#define E_INVALIDARG ((HRESULT)0x80070057L)
#define E_POINTER ((HRESULT)0x80004003L)

static const GUID IID_IUnknown_ = {0x00000000, 0x0000, 0x0000, {0xC0, 0, 0, 0, 0, 0, 0, 0x46}};
static const GUID IID_Factory = {0xb859ee5a, 0xd838, 0x4b5b, {0xa2, 0xe8, 0x1a, 0xdc, 0x7d, 0x93, 0xdb, 0x48}};
static const GUID IID_Collection = {0xa84cee02, 0x3eea, 0x4eee, {0xa8, 0x27, 0x87, 0xc1, 0xa0, 0x2a, 0x0f, 0xcc}};

static int same_guid(const GUID *x, const GUID *y) {
    const unsigned char *p = (const unsigned char *)x, *q = (const unsigned char *)y;
    int i;
    for (i = 0; i < 16; i++) if (p[i] != q[i]) return 0;
    return 1;
}

typedef struct { void **vtbl; } Object;
static volatile long refs;

/* ---- IUnknown, shared by both objects (each object answers for its own IID) */
static HRESULT WINAPI qi(Object *self, const GUID *iid, void **out, const GUID *own) {
    if (!out) return E_POINTER;
    if (same_guid(iid, &IID_IUnknown_) || same_guid(iid, own)) {
        *out = self;
        __sync_fetch_and_add(&refs, 1);
        return 0;
    }
    *out = 0;
    return E_NOINTERFACE;
}
static ULONG WINAPI add_ref(Object *self) { (void)self; return (ULONG)__sync_add_and_fetch(&refs, 1); }
static ULONG WINAPI release(Object *self) { (void)self; return (ULONG)__sync_sub_and_fetch(&refs, 1) | 1; }

/* "not implemented", popping n bytes of arguments (this included) */
#define NOT_IMPL(name, bytes_sig) static HRESULT WINAPI name bytes_sig { return E_NOTIMPL; }

/* ---- the empty font collection */
static Object collection;
static HRESULT WINAPI col_qi(Object *self, const GUID *iid, void **out) { return qi(self, iid, out, &IID_Collection); }
static ULONG WINAPI col_count(Object *self) { (void)self; return 0; }
static HRESULT WINAPI col_family(Object *self, unsigned index, void **out) {
    (void)self; (void)index;
    if (out) *out = 0;
    return E_INVALIDARG; /* there is no family `index` */
}
static HRESULT WINAPI col_find(Object *self, const wchar16 *name, unsigned *index, BOOL *exists) {
    (void)self; (void)name;
    if (index) *index = 0xFFFFFFFFu; /* UINT_MAX: not found, as documented */
    if (exists) *exists = 0;
    return 0;
}
static HRESULT WINAPI col_from_face(Object *self, void *face, void **out) {
    (void)self; (void)face;
    if (out) *out = 0;
    return E_NOTIMPL;
}
static void *collection_vtbl[] = {(void *)col_qi, (void *)add_ref, (void *)release, (void *)col_count,
                                  (void *)col_family, (void *)col_find, (void *)col_from_face};

/* ---- the factory: GetSystemFontCollection answers the empty collection; the rest E_NOTIMPL */
static Object factory;
static HRESULT WINAPI fac_qi(Object *self, const GUID *iid, void **out) { return qi(self, iid, out, &IID_Factory); }
static HRESULT WINAPI fac_system_fonts(Object *self, void **out, BOOL check) {
    (void)self; (void)check;
    if (!out) return E_POINTER;
    collection.vtbl = collection_vtbl;
    __sync_fetch_and_add(&refs, 1);
    *out = &collection;
    return 0;
}
NOT_IMPL(f4, (void *t, void *a, void *b, unsigned c, void *d))                                   /* CreateCustomFontCollection */
NOT_IMPL(f5, (void *t, void *a))                                                                  /* RegisterFontCollectionLoader */
NOT_IMPL(f6, (void *t, void *a))                                                                  /* UnregisterFontCollectionLoader */
NOT_IMPL(f7, (void *t, void *a, void *b, void *c))                                                /* CreateFontFileReference */
NOT_IMPL(f8, (void *t, void *a, unsigned b, void *c, void *d))                                    /* CreateCustomFontFileReference */
NOT_IMPL(f9, (void *t, int a, unsigned b, void *c, unsigned d, int e, void *f))                   /* CreateFontFace */
NOT_IMPL(f10, (void *t, void *a))                                                                 /* CreateRenderingParams */
NOT_IMPL(f11, (void *t, void *a, void *b))                                                        /* CreateMonitorRenderingParams */
NOT_IMPL(f12, (void *t, float a, float b, float c, int d, int e, void *f))                        /* CreateCustomRenderingParams */
NOT_IMPL(f13, (void *t, void *a))                                                                 /* RegisterFontFileLoader */
NOT_IMPL(f14, (void *t, void *a))                                                                 /* UnregisterFontFileLoader */
NOT_IMPL(f15, (void *t, void *a, void *b, int c, int d, int e, float f, void *g, void *h))        /* CreateTextFormat */
NOT_IMPL(f16, (void *t, void *a))                                                                 /* CreateTypography */
NOT_IMPL(f17, (void *t, void *a))                                                                 /* GetGdiInterop */
NOT_IMPL(f18, (void *t, void *a, unsigned b, void *c, float d, float e, void *f))                 /* CreateTextLayout */
NOT_IMPL(f19, (void *t, void *a, unsigned b, void *c, float d, float e, float f, void *g, BOOL h, void *i)) /* CreateGdiCompatibleTextLayout */
NOT_IMPL(f20, (void *t, void *a, void *b))                                                        /* CreateEllipsisTrimmingSign */
NOT_IMPL(f21, (void *t, void *a))                                                                 /* CreateTextAnalyzer */
NOT_IMPL(f22, (void *t, int a, void *b, BOOL c, void *d))                                         /* CreateNumberSubstitution */
NOT_IMPL(f23, (void *t, void *a, float b, void *c, int d, int e, float f, float g, void *h))      /* CreateGlyphRunAnalysis */
static void *factory_vtbl[] = {(void *)fac_qi, (void *)add_ref, (void *)release, (void *)fac_system_fonts,
                               (void *)f4, (void *)f5, (void *)f6, (void *)f7, (void *)f8, (void *)f9,
                               (void *)f10, (void *)f11, (void *)f12, (void *)f13, (void *)f14, (void *)f15,
                               (void *)f16, (void *)f17, (void *)f18, (void *)f19, (void *)f20, (void *)f21,
                               (void *)f22, (void *)f23};

#ifdef XP_DWRITE_TEST
int xp_dwrite_force_ours; /* tools/winxp/shim/tests: the empty factory even where DirectWrite exists */
#endif

HRESULT WINAPI xp_DWriteCreateFactory(int type, const GUID *iid, void **out) {
    void *real = REAL(void *, W_DWRITE, "DWriteCreateFactory");
#ifdef XP_DWRITE_TEST
    if (xp_dwrite_force_ours) real = 0;
#endif
    (void)type;
    if (real) return ((HRESULT(WINAPI *)(int, const GUID *, void **))real)(type, iid, out);
    if (!out) return E_POINTER;
    factory.vtbl = factory_vtbl;
    return qi(&factory, iid, out, &IID_Factory);
}

XP_ANSWER(xp_DWriteCreateFactory, DWriteCreateFactory, 12)
