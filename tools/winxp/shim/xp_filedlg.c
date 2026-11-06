/*
 * The Open / Save / folder dialogs on Windows XP (lane WINXP-RECHECK, 2026-11-06). 32-bit x86 only.
 * Ours, clean-room (tools/winxp/clean-room.md).
 *
 * RapidR's dialogs (Studio's File > Open / Save As, a program's ROpenDialog / RSaveDialog, folder
 * pickers) go through rfd, which asks COM for Windows Vista's "Common Item Dialog":
 * CoCreateInstance(CLSID_FileOpenDialog or CLSID_FileSaveDialog). Plain Windows XP has no such class:
 * the call fails, rfd reports "cancelled", and the dialog answered nothing.
 *
 * In the XP flavour only, CoCreateInstance is answered here: Windows' own first, always (every other
 * class, and these two wherever Windows has them: Vista and later, XP with One-Core-API). Only when
 * Windows could not make one of those two dialogs does this object hand back a dialog of its own with the
 * same COM interface (IFileOpenDialog / IFileSaveDialog), shown with XP's own dialogs:
 * GetOpenFileNameW / GetSaveFileNameW (comdlg32) and, for folders, SHBrowseForFolderW (shell32). Its
 * results are IShellItem / IShellItemArray objects of ours that answer GetDisplayName with the path.
 * SHCreateItemFromParsingName (Vista's, which rfd uses for the starting folder) is answered the same
 * way: Windows' own first, else a shell item of ours holding the path.
 *
 * Written from Microsoft Learn's pages for IModalWindow, IFileDialog, IFileOpenDialog,
 * IFileSaveDialog, IShellItem, IShellItemArray, FILEOPENDIALOGOPTIONS, SIGDN, COMDLG_FILTERSPEC,
 * OPENFILENAMEW, GetOpenFileNameW, GetSaveFileNameW, CommDlgExtendedError, BROWSEINFOW,
 * SHBrowseForFolderW, BrowseCallbackProc, SHGetPathFromIDListW, CoCreateInstance, CoTaskMemAlloc;
 * the method order and the interface and class IDs as the windows / windows-sys crates declare them
 * (MIT/Apache-2.0), and which methods rfd 0.17 calls (its win_cid backend, MIT). No other
 * implementation of these dialogs was read.
 */
#include "xp_common.h"

typedef struct {
    unsigned long a;
    unsigned short b, c;
    unsigned char d[8];
} GUID;
typedef void *HWND;

#define S_OK ((HRESULT)0)
#define S_FALSE ((HRESULT)1)
#define E_FAIL ((HRESULT)0x80004005L)
#define E_NOINTERFACE ((HRESULT)0x80004002L)
#define E_POINTER ((HRESULT)0x80004003L)
#define E_OUTOFMEMORY ((HRESULT)0x8007000EL)
#define E_INVALIDARG ((HRESULT)0x80070057L)
#define HR_CANCELLED ((HRESULT)0x800704C7L) /* HRESULT_FROM_WIN32(ERROR_CANCELLED): the Vista dialog's "cancelled" */
#define HR_NOT_FOUND ((HRESULT)0x80070002L) /* HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) */
#define REGDB_E_CLASSNOTREG ((HRESULT)0x80040154L)

/* FILEOPENDIALOGOPTIONS (the ones with an OPENFILENAME twin have the same value there) */
#define FOS_OVERWRITEPROMPT 0x2UL
#define FOS_NOCHANGEDIR 0x8UL
#define FOS_PICKFOLDERS 0x20UL
#define FOS_ALLOWMULTISELECT 0x200UL
#define FOS_PATHMUSTEXIST 0x800UL
#define FOS_FILEMUSTEXIST 0x1000UL
#define FOS_CREATEPROMPT 0x2000UL
#define FOS_NOREADONLYRETURN 0x8000UL
#define FOS_NODEREFERENCELINKS 0x100000UL
#define FOS_DONTADDTORECENT 0x2000000UL
#define FOS_FORCESHOWHIDDEN 0x10000000UL
/* OPENFILENAME's own */
#define OFN_HIDEREADONLY 0x4UL
#define OFN_NOCHANGEDIR 0x8UL
#define OFN_EXPLORER 0x80000UL
#define OFN_ENABLESIZING 0x800000UL
/* SIGDN */
#define SIGDN_NORMALDISPLAY 0UL
#define SIGDN_PARENTRELATIVEPARSING 0x80018001UL
#define SIGDN_PARENTRELATIVEEDITING 0x80031001UL
#define SIGDN_PARENTRELATIVEFORADDRESSBAR 0x8007C001UL
#define SIGDN_PARENTRELATIVE 0x80080001UL
#define SIGDN_FILESYSPATH 0x80058000UL
/* SHBrowseForFolder */
#define BIF_RETURNONLYFSDIRS 0x1U
#define BIF_EDITBOX 0x10U
#define BIF_NEWDIALOGSTYLE 0x40U
#define BFFM_INITIALIZED 1U
#define BFFM_SETSELECTIONW 0x467U /* WM_USER + 103 */
/* SFGAO */
#define SFGAO_FOLDER 0x20000000UL
#define SFGAO_FILESYSTEM 0x40000000UL

static const GUID CLSID_FileOpenDialog = {0xdc1c5a9c, 0xe88a, 0x4dde, {0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7}};
static const GUID CLSID_FileSaveDialog = {0xc0b4e2f3, 0xba21, 0x4773, {0x8d, 0xba, 0x33, 0x5e, 0xc9, 0x46, 0xeb, 0x8b}};
static const GUID IID_IUnknown = {0x00000000, 0x0000, 0x0000, {0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46}};
static const GUID IID_IModalWindow = {0xb4db1657, 0x70d7, 0x485e, {0x8e, 0x3e, 0x6f, 0xcb, 0x5a, 0x5c, 0x18, 0x02}};
static const GUID IID_IFileDialog = {0x42f85136, 0xdb7e, 0x439c, {0x85, 0xf1, 0xe4, 0x07, 0x5d, 0x13, 0x5f, 0xc8}};
static const GUID IID_IFileOpenDialog = {0xd57c7288, 0xd4ad, 0x4768, {0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60}};
static const GUID IID_IFileSaveDialog = {0x84bccd23, 0x5fde, 0x4cdb, {0xae, 0xa4, 0xaf, 0x64, 0xb8, 0x3d, 0x78, 0xab}};
static const GUID IID_IShellItem = {0x43826d1e, 0xe718, 0x42ee, {0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe}};
static const GUID IID_IShellItemArray = {0xb63ea76d, 0x1f85, 0x456f, {0xa1, 0x9c, 0x48, 0x15, 0x9e, 0xfa, 0x85, 0x8b}};

__declspec(dllimport) HANDLE WINAPI GetProcessHeap(void);
__declspec(dllimport) void *WINAPI HeapAlloc(HANDLE, ULONG, unsigned long);
__declspec(dllimport) BOOL WINAPI HeapFree(HANDLE, ULONG, void *);
__declspec(dllimport) ULONG WINAPI GetFileAttributesW(const wchar16 *);

static const wchar16 W_OLE32[] = {'o', 'l', 'e', '3', '2', '.', 'd', 'l', 'l', 0};
static const wchar16 W_COMDLG32[] = {'c', 'o', 'm', 'd', 'l', 'g', '3', '2', '.', 'd', 'l', 'l', 0};

/* the test programs force this object's own dialog where Windows has the real one (tests/test_filedlg.c) */
int xp_filedlg_force_ours;

/* ---------------------------------------------------------------- small helpers (no C runtime) */

static int same_guid(const GUID *x, const GUID *y) {
    const unsigned char *p = (const unsigned char *)x, *q = (const unsigned char *)y;
    int i;
    if (!x || !y) return 0;
    for (i = 0; i < 16; i++)
        if (p[i] != q[i]) return 0;
    return 1;
}
static void *mem(unsigned long n) { return HeapAlloc(GetProcessHeap(), 8 /* HEAP_ZERO_MEMORY */, n ? n : 1); }
static void unmem(void *p) {
    if (p) HeapFree(GetProcessHeap(), 0, p);
}
static unsigned long wlen(const wchar16 *s) {
    unsigned long n = 0;
    if (s)
        while (s[n]) n++;
    return n;
}
static wchar16 *wdup(const wchar16 *s) {
    unsigned long n = wlen(s), i;
    wchar16 *d;
    if (!s) return 0;
    d = (wchar16 *)mem((n + 1) * 2);
    if (!d) return 0;
    for (i = 0; i < n; i++) d[i] = s[i];
    d[n] = 0;
    return d;
}
static void wset(wchar16 **slot, const wchar16 *s) {
    unmem(*slot);
    *slot = s ? wdup(s) : 0;
}
/* a copy in COM task memory (the caller frees it with CoTaskMemFree, as with Windows' own) */
static HRESULT task_copy(const wchar16 *s, wchar16 **out) {
    void *(WINAPI * alloc)(unsigned long) = REAL(void *(WINAPI *)(unsigned long), W_OLE32, "CoTaskMemAlloc");
    unsigned long n = wlen(s), i;
    wchar16 *d;
    if (!out) return E_POINTER;
    *out = 0;
    if (!alloc) return E_OUTOFMEMORY;
    d = (wchar16 *)alloc((n + 1) * 2);
    if (!d) return E_OUTOFMEMORY;
    for (i = 0; i < n; i++) d[i] = s[i];
    d[n] = 0;
    *out = d;
    return S_OK;
}
static int ci_equal(const wchar16 *a, const wchar16 *b) {
    for (;; a++, b++) {
        wchar16 x = *a, y = *b;
        if (x >= 'A' && x <= 'Z') x += 32;
        if (y >= 'A' && y <= 'Z') y += 32;
        if (x != y) return 0;
        if (!x) return 1;
    }
}
/* a path's last part (after the last \ or /) */
static const wchar16 *last_part(const wchar16 *p) {
    const wchar16 *r = p;
    for (; *p; p++)
        if (*p == '\\' || *p == '/') r = p + 1;
    return r;
}

/* ---------------------------------------------------------------- IShellItem: a path */

typedef struct {
    const void *const *vtbl;
    volatile LONG refs;
    wchar16 *path;
} Item;
static const void *const item_vtbl[8];

static Item *new_item(const wchar16 *path) {
    Item *it = (Item *)mem(sizeof(Item));
    if (!it) return 0;
    it->vtbl = item_vtbl;
    it->refs = 1;
    it->path = wdup(path);
    if (!it->path) {
        unmem(it);
        return 0;
    }
    return it;
}
static HRESULT WINAPI item_qi(Item *self, const GUID *iid, void **out) {
    if (!out) return E_POINTER;
    if (same_guid(iid, &IID_IUnknown) || same_guid(iid, &IID_IShellItem)) {
        __sync_add_and_fetch(&self->refs, 1);
        *out = self;
        return S_OK;
    }
    *out = 0;
    return E_NOINTERFACE;
}
static ULONG WINAPI item_addref(Item *self) { return (ULONG)__sync_add_and_fetch(&self->refs, 1); }
static ULONG WINAPI item_release(Item *self) {
    LONG n = __sync_sub_and_fetch(&self->refs, 1);
    if (n == 0) {
        unmem(self->path);
        unmem(self);
    }
    return (ULONG)n;
}
static HRESULT WINAPI item_bind(Item *self, void *bc, const GUID *bhid, const GUID *iid, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI item_parent(Item *self, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI item_name(Item *self, ULONG sigdn, wchar16 **out) {
    switch (sigdn) {
    case SIGDN_NORMALDISPLAY:
    case SIGDN_PARENTRELATIVEPARSING:
    case SIGDN_PARENTRELATIVEEDITING:
    case SIGDN_PARENTRELATIVEFORADDRESSBAR:
    case SIGDN_PARENTRELATIVE: {
        const wchar16 *p = last_part(self->path);
        return task_copy(*p ? p : self->path, out);
    }
    default: /* the file system path, and the absolute parsing / editing names, are the path */
        return task_copy(self->path, out);
    }
}
static HRESULT WINAPI item_attributes(Item *self, ULONG mask, ULONG *attrs) {
    ULONG a = GetFileAttributesW(self->path), r = SFGAO_FILESYSTEM;
    if (a != 0xFFFFFFFFUL && (a & 0x10UL /* FILE_ATTRIBUTE_DIRECTORY */)) r |= SFGAO_FOLDER;
    if (!attrs) return E_POINTER;
    *attrs = r & mask;
    return *attrs == mask ? S_OK : S_FALSE;
}
/* the other item's file system path, whoever made it (ours or Windows') */
static wchar16 *path_of(void *item) {
    typedef HRESULT(WINAPI * NameFn)(void *, ULONG, wchar16 **);
    void (WINAPI *task_free)(void *) = REAL(void (WINAPI *)(void *), W_OLE32, "CoTaskMemFree");
    wchar16 *s = 0, *d;
    if (!item) return 0;
    if (*(const void *const **)item == item_vtbl) return wdup(((Item *)item)->path);
    if (((NameFn)(*(void ***)item)[5])(item, SIGDN_FILESYSPATH, &s) < 0 || !s) return 0;
    d = wdup(s);
    if (task_free) task_free(s);
    return d;
}
static HRESULT WINAPI item_compare(Item *self, void *other, ULONG hint, int *order) {
    wchar16 *o = path_of(other);
    int eq;
    if (!order) return E_POINTER;
    if (!o) return E_INVALIDARG;
    eq = ci_equal(self->path, o);
    unmem(o);
    *order = eq ? 0 : 1;
    return eq ? S_OK : S_FALSE;
}
static const void *const item_vtbl[8] = {
    (void *)item_qi, (void *)item_addref, (void *)item_release, (void *)item_bind,
    (void *)item_parent, (void *)item_name, (void *)item_attributes, (void *)item_compare,
};

/* the answer to SHCreateItemFromParsingName where Windows has none: an item of ours for a path that
 * exists (Windows' own fails for one that does not: "file not found") */
static HRESULT item_for_path(const wchar16 *path, const GUID *iid, void **out) {
    Item *it;
    HRESULT hr;
    if (!out) return E_POINTER;
    *out = 0;
    if (!path || !*path) return E_INVALIDARG;
    if (GetFileAttributesW(path) == 0xFFFFFFFFUL) return HR_NOT_FOUND;
    it = new_item(path);
    if (!it) return E_OUTOFMEMORY;
    hr = item_qi(it, iid, out);
    item_release(it);
    return hr;
}

HRESULT WINAPI xp_SHCreateItemFromParsingName(const wchar16 *path, void *bc, const GUID *iid, void **out) {
    typedef HRESULT(WINAPI * Fn)(const wchar16 *, void *, const GUID *, void **);
    Fn real = xp_filedlg_force_ours ? (Fn)0 : REAL(Fn, W_SHELL32, "SHCreateItemFromParsingName");
    if (real) return real(path, bc, iid, out);
    return item_for_path(path, iid, out);
}

/* ---------------------------------------------------------------- IShellItemArray: the picked paths */

typedef struct {
    const void *const *vtbl;
    volatile LONG refs;
    ULONG count;
    wchar16 **paths;
} Items;
static const void *const items_vtbl[10];

static void free_paths(wchar16 **paths, ULONG count) {
    ULONG i;
    if (!paths) return;
    for (i = 0; i < count; i++) unmem(paths[i]);
    unmem(paths);
}
static HRESULT WINAPI items_qi(Items *self, const GUID *iid, void **out) {
    if (!out) return E_POINTER;
    if (same_guid(iid, &IID_IUnknown) || same_guid(iid, &IID_IShellItemArray)) {
        __sync_add_and_fetch(&self->refs, 1);
        *out = self;
        return S_OK;
    }
    *out = 0;
    return E_NOINTERFACE;
}
static ULONG WINAPI items_addref(Items *self) { return (ULONG)__sync_add_and_fetch(&self->refs, 1); }
static ULONG WINAPI items_release(Items *self) {
    LONG n = __sync_sub_and_fetch(&self->refs, 1);
    if (n == 0) {
        free_paths(self->paths, self->count);
        unmem(self);
    }
    return (ULONG)n;
}
static HRESULT WINAPI items_bind(Items *self, void *bc, const GUID *bhid, const GUID *iid, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI items_store(Items *self, ULONG flags, const GUID *iid, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI items_descriptions(Items *self, const void *key, const GUID *iid, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI items_attributes(Items *self, ULONG how, ULONG mask, ULONG *attrs) {
    if (attrs) *attrs = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI items_count(Items *self, ULONG *n) {
    if (!n) return E_POINTER;
    *n = self->count;
    return S_OK;
}
static HRESULT WINAPI items_at(Items *self, ULONG i, void **out) {
    Item *it;
    if (!out) return E_POINTER;
    *out = 0;
    if (i >= self->count) return E_INVALIDARG;
    it = new_item(self->paths[i]);
    if (!it) return E_OUTOFMEMORY;
    *out = it;
    return S_OK;
}
static HRESULT WINAPI items_enum(Items *self, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static const void *const items_vtbl[10] = {
    (void *)items_qi, (void *)items_addref, (void *)items_release, (void *)items_bind, (void *)items_store,
    (void *)items_descriptions, (void *)items_attributes, (void *)items_count, (void *)items_at, (void *)items_enum,
};

/* ---------------------------------------------------------------- the dialog */

typedef struct {
    const wchar16 *name, *spec;
} FilterSpec; /* COMDLG_FILTERSPEC */

typedef struct {
    const void *const *vtbl;
    volatile LONG refs;
    int save;
    ULONG options;
    wchar16 *title, *file_name, *def_ext, *folder, *default_folder;
    wchar16 *filter; /* OPENFILENAME's: "name\0spec\0" ... "\0" */
    ULONG filter_index; /* 1-based, 0: the first */
    ULONG count;        /* the results */
    wchar16 **results;
} Dialog;
static const void *const open_vtbl[29];
static const void *const save_vtbl[32];

static HRESULT WINAPI dlg_qi(Dialog *self, const GUID *iid, void **out) {
    if (!out) return E_POINTER;
    if (same_guid(iid, &IID_IUnknown) || same_guid(iid, &IID_IModalWindow) || same_guid(iid, &IID_IFileDialog) ||
        same_guid(iid, self->save ? &IID_IFileSaveDialog : &IID_IFileOpenDialog)) {
        __sync_add_and_fetch(&self->refs, 1);
        *out = self;
        return S_OK;
    }
    *out = 0;
    return E_NOINTERFACE;
}
static ULONG WINAPI dlg_addref(Dialog *self) { return (ULONG)__sync_add_and_fetch(&self->refs, 1); }
static ULONG WINAPI dlg_release(Dialog *self) {
    LONG n = __sync_sub_and_fetch(&self->refs, 1);
    if (n == 0) {
        unmem(self->title);
        unmem(self->file_name);
        unmem(self->def_ext);
        unmem(self->folder);
        unmem(self->default_folder);
        unmem(self->filter);
        free_paths(self->results, self->count);
        unmem(self);
    }
    return (ULONG)n;
}

/* the results: replaced by `n` paths (taken) */
static void set_results(Dialog *self, wchar16 **paths, ULONG n) {
    free_paths(self->results, self->count);
    self->results = paths;
    self->count = n;
}
static HRESULT one_result(Dialog *self, const wchar16 *path) {
    wchar16 **v = (wchar16 **)mem(sizeof(wchar16 *));
    if (!v) return E_OUTOFMEMORY;
    v[0] = wdup(path);
    if (!v[0]) {
        unmem(v);
        return E_OUTOFMEMORY;
    }
    set_results(self, v, 1);
    return S_OK;
}

/* an extension a file name can be given: not empty, no wildcard */
static int concrete_ext(const wchar16 *e) {
    if (!e || !*e) return 0;
    for (; *e; e++)
        if (*e == '*' || *e == '?' || *e == ';') return 0;
    return 1;
}

/* OPENFILENAMEW (Windows 2000 and later: 88 bytes on 32-bit) */
typedef struct {
    ULONG lStructSize;
    HWND hwndOwner;
    HANDLE hInstance;
    const wchar16 *lpstrFilter;
    wchar16 *lpstrCustomFilter;
    ULONG nMaxCustFilter;
    ULONG nFilterIndex;
    wchar16 *lpstrFile;
    ULONG nMaxFile;
    wchar16 *lpstrFileTitle;
    ULONG nMaxFileTitle;
    const wchar16 *lpstrInitialDir;
    const wchar16 *lpstrTitle;
    ULONG Flags;
    unsigned short nFileOffset;
    unsigned short nFileExtension;
    const wchar16 *lpstrDefExt;
    long lCustData;
    void *lpfnHook;
    const wchar16 *lpTemplateName;
    void *pvReserved;
    ULONG dwReserved;
    ULONG FlagsEx;
} OpenFileName;
typedef char ofn_size_check[sizeof(OpenFileName) == 88 ? 1 : -1];

#define FNERR_INVALIDFILENAME 0x3002UL
#define FILE_BUFFER 32768UL /* characters: room for many picked files */

static HRESULT show_file(Dialog *self, HWND owner) {
    typedef BOOL(WINAPI * DlgFn)(OpenFileName *);
    typedef ULONG(WINAPI * ErrFn)(void);
    static const wchar16 all_files[] = {'A', 'l', 'l', ' ', 'F', 'i', 'l', 'e', 's', 0, '*', '.', '*', 0, 0};
    DlgFn fn = self->save ? REAL(DlgFn, W_COMDLG32, "GetSaveFileNameW") : REAL(DlgFn, W_COMDLG32, "GetOpenFileNameW");
    ErrFn why = REAL(ErrFn, W_COMDLG32, "CommDlgExtendedError");
    OpenFileName ofn;
    wchar16 *buf;
    unsigned long n, i;
    BOOL ok;
    if (!fn) return E_NOTIMPL;
    buf = (wchar16 *)mem(FILE_BUFFER * 2);
    if (!buf) return E_OUTOFMEMORY;
    if (self->file_name) {
        n = wlen(self->file_name);
        if (n >= FILE_BUFFER) n = FILE_BUFFER - 1;
        for (i = 0; i < n; i++) buf[i] = self->file_name[i];
    }
    {
        char *p = (char *)&ofn;
        for (i = 0; i < sizeof ofn; i++) p[i] = 0;
    }
    ofn.lStructSize = sizeof ofn;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = self->filter ? self->filter : all_files;
    ofn.nFilterIndex = self->filter_index ? self->filter_index : 1;
    ofn.lpstrFile = buf;
    ofn.nMaxFile = FILE_BUFFER;
    ofn.lpstrInitialDir = self->folder ? self->folder : self->default_folder;
    ofn.lpstrTitle = self->title;
    ofn.lpstrDefExt = concrete_ext(self->def_ext) ? self->def_ext : 0;
    /* the options both dialogs share have the same values; the Vista dialog never changes the
     * process's current folder, so this one never does either */
    ofn.Flags = (self->options & (FOS_OVERWRITEPROMPT | FOS_ALLOWMULTISELECT | FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST |
                                  FOS_CREATEPROMPT | FOS_NOREADONLYRETURN | FOS_NODEREFERENCELINKS |
                                  FOS_DONTADDTORECENT | FOS_FORCESHOWHIDDEN)) |
                OFN_EXPLORER | OFN_HIDEREADONLY | OFN_NOCHANGEDIR | OFN_ENABLESIZING;
    if (self->save) ofn.Flags &= ~FOS_ALLOWMULTISELECT;
    ok = fn(&ofn);
    if (!ok && buf[0] && why && why() == FNERR_INVALIDFILENAME) {
        /* XP refuses to open with a name it cannot take (quotes, a wildcard...); the Vista dialog shows
         * any text: open with an empty name instead */
        buf[0] = 0;
        ok = fn(&ofn);
    }
    if (!ok) {
        ULONG e = why ? why() : 1;
        unmem(buf);
        return e == 0 ? HR_CANCELLED : E_FAIL;
    }
    self->filter_index = ofn.nFilterIndex;
    if ((ofn.Flags & FOS_ALLOWMULTISELECT) && buf[wlen(buf) + 1]) {
        /* "folder\0name\0name\0\0": the folder, then each name in it */
        const wchar16 *dir = buf, *p;
        unsigned long dn = wlen(dir), count = 0, k = 0;
        wchar16 **v;
        for (p = buf + dn + 1; *p; p += wlen(p) + 1) count++;
        v = (wchar16 **)mem(count * sizeof(wchar16 *));
        if (!v) {
            unmem(buf);
            return E_OUTOFMEMORY;
        }
        for (p = buf + dn + 1; *p; p += wlen(p) + 1) {
            unsigned long pn = wlen(p), j, at = 0;
            int slash = dn > 0 && dir[dn - 1] == '\\';
            wchar16 *full = (wchar16 *)mem((dn + 1 + pn + 1) * 2);
            if (!full) break;
            for (j = 0; j < dn; j++) full[at++] = dir[j];
            if (!slash) full[at++] = '\\';
            for (j = 0; j < pn; j++) full[at++] = p[j];
            full[at] = 0;
            v[k++] = full;
        }
        set_results(self, v, k);
        unmem(buf);
        return k == count ? S_OK : E_OUTOFMEMORY;
    }
    {
        HRESULT hr = one_result(self, buf);
        unmem(buf);
        return hr;
    }
}

/* BROWSEINFOW */
typedef struct {
    HWND hwndOwner;
    const void *pidlRoot;
    wchar16 *pszDisplayName;
    const wchar16 *lpszTitle;
    unsigned ulFlags;
    void *lpfn;
    long lParam;
    int iImage;
} BrowseInfo;

static int WINAPI browse_callback(HWND hwnd, unsigned msg, long lp, long data) {
    Dialog *self = (Dialog *)data;
    if (msg == BFFM_INITIALIZED) {
        typedef long(WINAPI * SendFn)(HWND, unsigned, unsigned, long);
        typedef BOOL(WINAPI * TextFn)(HWND, const wchar16 *);
        SendFn send = REAL(SendFn, W_USER32, "SendMessageW");
        TextFn text = REAL(TextFn, W_USER32, "SetWindowTextW");
        const wchar16 *start = self->folder ? self->folder : self->default_folder;
        if (self->title && text) text(hwnd, self->title);
        if (start && send) send(hwnd, BFFM_SETSELECTIONW, 1, (long)start);
    }
    return 0;
}

static HRESULT show_folder(Dialog *self, HWND owner) {
    typedef void *(WINAPI * BrowseFn)(BrowseInfo *);
    typedef BOOL(WINAPI * PathFn)(const void *, wchar16 *);
    BrowseFn browse = REAL(BrowseFn, W_SHELL32, "SHBrowseForFolderW");
    PathFn to_path = REAL(PathFn, W_SHELL32, "SHGetPathFromIDListW");
    void (WINAPI *task_free)(void *) = REAL(void (WINAPI *)(void *), W_OLE32, "CoTaskMemFree");
    wchar16 name[260], path[262];
    BrowseInfo bi;
    void *pidl;
    BOOL ok;
    if (!browse || !to_path) return E_NOTIMPL;
    name[0] = 0;
    path[0] = 0;
    bi.hwndOwner = owner;
    bi.pidlRoot = 0;
    bi.pszDisplayName = name;
    bi.lpszTitle = self->title;
    bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_EDITBOX;
    bi.lpfn = (void *)browse_callback;
    bi.lParam = (long)self;
    bi.iImage = 0;
    pidl = browse(&bi);
    if (!pidl) return HR_CANCELLED;
    ok = to_path(pidl, path);
    if (task_free) task_free(pidl);
    if (!ok || !path[0]) return E_FAIL; /* a place that is not a folder on disk (e.g. Control Panel) */
    return one_result(self, path);
}

static HRESULT WINAPI dlg_show(Dialog *self, HWND owner) {
    if (self->options & FOS_PICKFOLDERS) return show_folder(self, owner);
    return show_file(self, owner);
}
static HRESULT WINAPI dlg_set_types(Dialog *self, unsigned n, const FilterSpec *specs) {
    unsigned long total = 1, i, at = 0;
    wchar16 *f;
    if (n == 0 || !specs) return E_INVALIDARG;
    for (i = 0; i < n; i++) total += wlen(specs[i].name) + 1 + wlen(specs[i].spec) + 1;
    f = (wchar16 *)mem(total * 2);
    if (!f) return E_OUTOFMEMORY;
    for (i = 0; i < n; i++) {
        const wchar16 *s;
        for (s = specs[i].name; s && *s; s++) f[at++] = *s;
        f[at++] = 0;
        for (s = specs[i].spec; s && *s; s++) f[at++] = *s;
        f[at++] = 0;
    }
    f[at] = 0;
    unmem(self->filter);
    self->filter = f;
    return S_OK;
}
static HRESULT WINAPI dlg_set_type_index(Dialog *self, unsigned i) {
    self->filter_index = i;
    return S_OK;
}
static HRESULT WINAPI dlg_get_type_index(Dialog *self, unsigned *i) {
    if (!i) return E_POINTER;
    *i = self->filter_index ? self->filter_index : 1;
    return S_OK;
}
static HRESULT WINAPI dlg_advise(Dialog *self, void *events, ULONG *cookie) {
    if (cookie) *cookie = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI dlg_unadvise(Dialog *self, ULONG cookie) { return E_NOTIMPL; }
static HRESULT WINAPI dlg_set_options(Dialog *self, ULONG fos) {
    self->options = fos;
    return S_OK;
}
static HRESULT WINAPI dlg_get_options(Dialog *self, ULONG *fos) {
    if (!fos) return E_POINTER;
    *fos = self->options;
    return S_OK;
}
static HRESULT WINAPI dlg_set_default_folder(Dialog *self, void *item) {
    wchar16 *p = path_of(item);
    if (!p) return E_INVALIDARG;
    unmem(self->default_folder);
    self->default_folder = p;
    return S_OK;
}
static HRESULT WINAPI dlg_set_folder(Dialog *self, void *item) {
    wchar16 *p = path_of(item);
    if (!p) return E_INVALIDARG;
    unmem(self->folder);
    self->folder = p;
    return S_OK;
}
static HRESULT WINAPI dlg_get_folder(Dialog *self, void **out) {
    const wchar16 *f = self->folder ? self->folder : self->default_folder;
    Item *it;
    if (!out) return E_POINTER;
    *out = 0;
    if (!f) return E_FAIL;
    it = new_item(f);
    if (!it) return E_OUTOFMEMORY;
    *out = it;
    return S_OK;
}
static HRESULT WINAPI dlg_get_selection(Dialog *self, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI dlg_set_file_name(Dialog *self, const wchar16 *name) {
    wset(&self->file_name, name);
    return S_OK;
}
static HRESULT WINAPI dlg_get_file_name(Dialog *self, wchar16 **out) {
    static const wchar16 none[] = {0};
    return task_copy(self->file_name ? self->file_name : none, out);
}
static HRESULT WINAPI dlg_set_title(Dialog *self, const wchar16 *title) {
    wset(&self->title, title);
    return S_OK;
}
static HRESULT WINAPI dlg_set_ok_label(Dialog *self, const wchar16 *label) { return S_OK; } /* XP's own labels stay */
static HRESULT WINAPI dlg_set_name_label(Dialog *self, const wchar16 *label) { return S_OK; }
static HRESULT WINAPI dlg_get_result(Dialog *self, void **out) {
    Item *it;
    if (!out) return E_POINTER;
    *out = 0;
    if (self->count == 0) return E_FAIL;
    it = new_item(self->results[0]);
    if (!it) return E_OUTOFMEMORY;
    *out = it;
    return S_OK;
}
static HRESULT WINAPI dlg_add_place(Dialog *self, void *item, int where) { return S_OK; } /* XP's places bar is fixed */
static HRESULT WINAPI dlg_set_default_ext(Dialog *self, const wchar16 *ext) {
    wset(&self->def_ext, ext && *ext == '.' ? ext + 1 : ext);
    return S_OK;
}
static HRESULT WINAPI dlg_close(Dialog *self, HRESULT hr) { return E_NOTIMPL; } /* only from event handlers, which this has none of */
static HRESULT WINAPI dlg_set_client_guid(Dialog *self, const GUID *g) { return S_OK; }
static HRESULT WINAPI dlg_clear_client_data(Dialog *self) { return S_OK; }
static HRESULT WINAPI dlg_set_filter(Dialog *self, void *filter) { return E_NOTIMPL; }
/* IFileOpenDialog */
static HRESULT WINAPI dlg_get_results(Dialog *self, void **out) {
    Items *v;
    ULONG i;
    if (!out) return E_POINTER;
    *out = 0;
    if (self->count == 0) return E_FAIL;
    v = (Items *)mem(sizeof(Items));
    if (!v) return E_OUTOFMEMORY;
    v->vtbl = items_vtbl;
    v->refs = 1;
    v->paths = (wchar16 **)mem(self->count * sizeof(wchar16 *));
    if (!v->paths) {
        unmem(v);
        return E_OUTOFMEMORY;
    }
    for (i = 0; i < self->count; i++) {
        v->paths[i] = wdup(self->results[i]);
        if (!v->paths[i]) {
            v->count = i;
            items_release(v);
            return E_OUTOFMEMORY;
        }
    }
    v->count = self->count;
    *out = v;
    return S_OK;
}
static HRESULT WINAPI dlg_get_selected(Dialog *self, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
/* IFileSaveDialog */
static HRESULT WINAPI dlg_set_save_as_item(Dialog *self, void *item) {
    wchar16 *p = path_of(item);
    if (!p) return E_INVALIDARG;
    unmem(self->file_name);
    self->file_name = p; /* a full path: XP's dialog opens in its folder with its name */
    return S_OK;
}
static HRESULT WINAPI dlg_set_properties(Dialog *self, void *store) { return E_NOTIMPL; }
static HRESULT WINAPI dlg_set_collected(Dialog *self, void *list, BOOL append) { return E_NOTIMPL; }
static HRESULT WINAPI dlg_get_properties(Dialog *self, void **out) {
    if (out) *out = 0;
    return E_NOTIMPL;
}
static HRESULT WINAPI dlg_apply_properties(Dialog *self, void *item, void *store, HWND hwnd, void *sink) { return E_NOTIMPL; }

#define DIALOG_METHODS                                                                                        \
    (void *)dlg_qi, (void *)dlg_addref, (void *)dlg_release, (void *)dlg_show, (void *)dlg_set_types,         \
        (void *)dlg_set_type_index, (void *)dlg_get_type_index, (void *)dlg_advise, (void *)dlg_unadvise,     \
        (void *)dlg_set_options, (void *)dlg_get_options, (void *)dlg_set_default_folder,                     \
        (void *)dlg_set_folder, (void *)dlg_get_folder, (void *)dlg_get_selection, (void *)dlg_set_file_name, \
        (void *)dlg_get_file_name, (void *)dlg_set_title, (void *)dlg_set_ok_label, (void *)dlg_set_name_label, \
        (void *)dlg_get_result, (void *)dlg_add_place, (void *)dlg_set_default_ext, (void *)dlg_close,        \
        (void *)dlg_set_client_guid, (void *)dlg_clear_client_data, (void *)dlg_set_filter
static const void *const open_vtbl[29] = {DIALOG_METHODS, (void *)dlg_get_results, (void *)dlg_get_selected};
static const void *const save_vtbl[32] = {DIALOG_METHODS, (void *)dlg_set_save_as_item, (void *)dlg_set_properties,
                                        (void *)dlg_set_collected, (void *)dlg_get_properties,
                                        (void *)dlg_apply_properties};

static HRESULT new_dialog(int save, const GUID *iid, void **out) {
    Dialog *d = (Dialog *)mem(sizeof(Dialog));
    HRESULT hr;
    if (!d) return E_OUTOFMEMORY;
    d->vtbl = save ? save_vtbl : open_vtbl;
    d->refs = 1;
    d->save = save;
    /* as a new Vista dialog starts: Open wants a file that exists, Save asks before replacing one */
    d->options = save ? (FOS_OVERWRITEPROMPT | FOS_NOCHANGEDIR | FOS_PATHMUSTEXIST | FOS_NOREADONLYRETURN)
                      : (FOS_NOCHANGEDIR | FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST);
    hr = dlg_qi(d, iid, out);
    dlg_release(d);
    return hr;
}

HRESULT WINAPI xp_CoCreateInstance(const GUID *clsid, void *outer, ULONG context, const GUID *iid, void **out) {
    typedef HRESULT(WINAPI * Fn)(const GUID *, void *, ULONG, const GUID *, void **);
    Fn real = REAL(Fn, W_OLE32, "CoCreateInstance");
    int open = same_guid(clsid, &CLSID_FileOpenDialog), save = same_guid(clsid, &CLSID_FileSaveDialog);
    HRESULT hr = REGDB_E_CLASSNOTREG;
    if (real && !(xp_filedlg_force_ours && (open || save))) hr = real(clsid, outer, context, iid, out);
    if (hr >= 0 || outer || !(open || save)) return hr;
    /* Windows could not make the Vista dialog (plain XP): ours */
    if (!out) return E_POINTER;
    return new_dialog(save, iid, out);
}

XP_ANSWER(xp_CoCreateInstance, CoCreateInstance, 20)
XP_ANSWER(xp_SHCreateItemFromParsingName, SHCreateItemFromParsingName, 16)
