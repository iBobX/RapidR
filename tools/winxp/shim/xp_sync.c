/*
 * Windows XP: the synchronization functions Rust's std (and LLVM's libunwind) use that XP lacks
 * (lane WINXP, 2026-11-05). Compiled ONLY into the Windows XP flavour (tools/winxp/build.sh).
 *
 *   slim reader/writer locks  AcquireSRWLockExclusive, AcquireSRWLockShared, ReleaseSRWLockExclusive,
 *                             ReleaseSRWLockShared, TryAcquireSRWLockExclusive (std's Mutex, libunwind)
 *   condition variables       SleepConditionVariableSRW, WakeConditionVariable, WakeAllConditionVariable
 *   one-time initialization   InitOnceBeginInitialize, InitOnceComplete (std's thread-local keys)
 *   fiber local storage       FlsAlloc, FlsSetValue, FlsFree, IsThreadAFiber (std's thread-local
 *                             destructors: the FLS callback runs them when a thread ends)
 *
 * Each group first looks for Windows' own functions (kernel32.dll; GetModuleHandleW + GetProcAddress):
 * Windows 7 and later, and XP with One-Core-API, have them all and every call goes straight to them.
 * Only where one of a group is missing does the whole group use the functions below (a lock and a
 * condition variable must come from the same implementation; Vista has SRW locks but no
 * TryAcquireSRWLockExclusive, so Vista uses ours too).
 *
 * How ours work (all state in the caller's pointer-sized SRWLOCK / CONDITION_VARIABLE / INIT_ONCE,
 * zero = initial, as Windows' own):
 *   - A lock word: bit 0 = held exclusively, bit 1 = threads are waiting, bits 2.. = shared holders.
 *     Taking a free lock is one compare-and-swap. A thread that has to wait parks: under the lock of
 *     one of 64 buckets (by address) it checks the word again, sets "waiting", queues itself and sleeps
 *     on its own event. Whoever releases a lock whose "waiting" bit is set clears it and wakes every
 *     thread queued on that address, under the same bucket lock (so a waiter is never missed); they
 *     compete again. A new reader waits while readers hold the lock and someone waits (writers are not
 *     starved). Not recursive, as Windows' own.
 *   - A condition variable: the sleeper queues itself on the variable's address BEFORE releasing the
 *     lock (so no wake-up between the two is lost), waits until a Wake takes it off the queue or its
 *     time is up (then it takes itself off; if a Wake took it first, that counts as woken), and takes
 *     the lock again. A queued flag, not the event, is the truth: a stale signal only causes one more look.
 *   - INIT_ONCE: 0 = not done, 1 = being done (others wait), (context | 2) = done.
 *   - FLS: a TLS slot per FLS index, the callback remembered; when a thread ends, this object's TLS
 *     callback (a TLS directory of its own: the program has none) calls each callback whose value on
 *     that thread is not null, once, as Windows calls FLS callbacks. RapidR makes no fibers, so
 *     IsThreadAFiber answers FALSE.
 *   Every function of ours keeps the thread's last-error value as it found it (Windows' own do not set it).
 *
 * Written from public documentation only: Microsoft Learn's pages on the functions above (their
 * behaviour and signatures), on CreateEventW, SetEvent, WaitForSingleObject, GetTickCount, TlsAlloc,
 * TlsGetValue, TlsSetValue, TlsFree, InitializeCriticalSection, EnterCriticalSection,
 * LeaveCriticalSection, Get/SetLastError; the PE/COFF specification (the TLS directory and TLS
 * callbacks, `__imp_` import naming); and the parking idea as generally described (a hashed table of
 * wait queues). No code of One-Core-API, Wine, ReactOS, mingw-w64, Microsoft or anyone else was read or
 * copied. 32-bit x86 only.
 */
#include "xp_answer.h"

typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef void *HANDLE;
typedef int BOOL;
#define WINAPI __attribute__((stdcall))
#define INFINITE 0xFFFFFFFFUL
#define ERROR_TIMEOUT 1460UL
#define ERROR_GEN_FAILURE 31UL
#define ERROR_INVALID_PARAMETER 87UL
#define FLS_OUT_OF_INDEXES 0xFFFFFFFFUL

typedef struct { void *opaque[6]; } CRITICAL_SECTION; /* 24 bytes on x86 */

__declspec(dllimport) void *WINAPI GetModuleHandleW(const wchar16 *);
__declspec(dllimport) void *WINAPI GetProcAddress(void *, const char *);
__declspec(dllimport) HANDLE WINAPI CreateEventW(void *, BOOL, BOOL, const wchar16 *);
__declspec(dllimport) BOOL WINAPI SetEvent(HANDLE);
__declspec(dllimport) ULONG WINAPI WaitForSingleObject(HANDLE, ULONG);
__declspec(dllimport) ULONG WINAPI GetTickCount(void);
__declspec(dllimport) ULONG WINAPI TlsAlloc(void);
__declspec(dllimport) void *WINAPI TlsGetValue(ULONG);
__declspec(dllimport) BOOL WINAPI TlsSetValue(ULONG, void *);
__declspec(dllimport) BOOL WINAPI TlsFree(ULONG);
__declspec(dllimport) void WINAPI InitializeCriticalSection(CRITICAL_SECTION *);
__declspec(dllimport) void WINAPI EnterCriticalSection(CRITICAL_SECTION *);
__declspec(dllimport) void WINAPI LeaveCriticalSection(CRITICAL_SECTION *);
__declspec(dllimport) ULONG WINAPI GetLastError(void);
__declspec(dllimport) void WINAPI SetLastError(ULONG);
__declspec(dllimport) void WINAPI Sleep(ULONG);

#ifdef XP_SYNC_TEST
/* (tools/winxp/shim/tests only: ours even where Windows has its own, to test ours there) */
int xp_sync_force_ours;
#endif

static const wchar16 KERNEL32[] = {'k', 'e', 'r', 'n', 'e', 'l', '3', '2', '.', 'd', 'l', 'l', 0};

static long cas(volatile long *p, long old, long new_) { return __sync_val_compare_and_swap(p, old, new_); }
static void *cas_ptr(void *volatile *p, void *old, void *new_) { return __sync_val_compare_and_swap(p, old, new_); }

/* ------------------------------------------------------------------ the parking lot */

typedef struct Waiter {
    volatile void *addr;
    struct Waiter *next;
    HANDLE event;
    volatile long queued; /* 1 until its waker (or itself, timed out) is done with it */
    int taken;            /* a waker took it off the queue (under the bucket lock) */
} Waiter;

typedef struct {
    CRITICAL_SECTION cs;
    Waiter *head;
} Bucket;

#define BUCKETS 64
static Bucket buckets[BUCKETS];
static volatile long lot_state; /* 0 = not made, 1 = being made, 2 = ready */
static ULONG event_slot;        /* the TLS slot holding each thread's event */

static void lot_init(void) {
    if (lot_state == 2) return;
    if (cas(&lot_state, 0, 1) == 0) {
        int i;
        for (i = 0; i < BUCKETS; i++) InitializeCriticalSection(&buckets[i].cs);
        event_slot = TlsAlloc();
        __sync_synchronize();
        lot_state = 2;
    } else {
        while (lot_state != 2) Sleep(0);
    }
    __sync_synchronize();
}

static Bucket *bucket_of(volatile void *addr) {
    unsigned long h = ((unsigned long)addr >> 2) * 2654435761UL;
    return &buckets[h >> 26]; /* the top 6 bits: 64 buckets */
}

/* Each thread's own auto-reset event (0 if Windows can't make one: then the waiter polls). Events are
 * never closed: when a thread ends its event goes on a free list for the next thread. (A waker may call
 * SetEvent a moment after the woken thread has gone on, even ended: a closed handle's number could
 * belong to something else by then. A stale signal on a recycled event only makes its next waiter look
 * at its queued flag once more.) */
static HANDLE free_events[256];
static volatile long free_count;
static volatile long free_lock;

static HANDLE my_event(void) {
    HANDLE e = TlsGetValue(event_slot);
    if (!e) {
        while (!__sync_bool_compare_and_swap(&free_lock, 0, 1)) Sleep(0);
        if (free_count > 0) e = free_events[--free_count];
        __sync_lock_release(&free_lock);
        if (!e) e = CreateEventW(0, 0, 0, 0);
        if (e) TlsSetValue(event_slot, e);
    }
    return e;
}

static void my_event_ends(void) {
    HANDLE e = TlsGetValue(event_slot);
    if (!e) return;
    TlsSetValue(event_slot, 0);
    while (!__sync_bool_compare_and_swap(&free_lock, 0, 1)) Sleep(0);
    if (free_count < 256) {
        free_events[free_count++] = e;
        e = 0;
    }
    __sync_lock_release(&free_lock);
    (void)e; /* (more than 256 ended threads' events at once: that one is kept open, not closed) */
}

static void enqueue(Bucket *b, Waiter *w) {
    Waiter **p = &b->head;
    while (*p) p = &(*p)->next;
    w->next = 0;
    *p = w;
}

/* takes the first (one) or every waiter on `addr` off the bucket's queue (the bucket's lock held);
 * returns them linked through `next` */
static Waiter *dequeue(Bucket *b, volatile void *addr, int all) {
    Waiter *out = 0, **tail = &out, **p = &b->head;
    while (*p) {
        Waiter *w = *p;
        if (w->addr == addr) {
            *p = w->next;
            w->next = 0;
            w->taken = 1;
            *tail = w;
            tail = &w->next;
            if (!all) break;
        } else {
            p = &w->next;
        }
    }
    return out;
}

/* wakes the waiters of a list (outside the bucket lock); a waiter's frame may be gone the moment its
 * flag is cleared, so its next and event are read first */
static void wake_list(Waiter *w) {
    while (w) {
        Waiter *next = w->next;
        HANDLE e = w->event;
        __sync_synchronize();
        w->queued = 0;
        __sync_synchronize();
        if (e) SetEvent(e);
        w = next;
    }
}

/* until a waker is done with `w`, or `ms` passed (INFINITE: no limit); 1 = timed out (it took itself
 * off the queue). A waiter whose time is up but that a waker already took (`taken`, decided under the
 * bucket lock) is woken, not timed out, and waits for that waker to finish: the waker still reads it. */
static int wait_queued(Waiter *w, Bucket *b, ULONG ms) {
    ULONG start = GetTickCount();
    while (w->queued) {
        ULONG wait = INFINITE;
        if (ms != INFINITE) {
            ULONG gone = GetTickCount() - start;
            if (gone >= ms) {
                EnterCriticalSection(&b->cs);
                if (!w->taken) {
                    Waiter **p = &b->head;
                    while (*p && *p != w) p = &(*p)->next;
                    if (*p) *p = w->next;
                    w->queued = 0;
                    LeaveCriticalSection(&b->cs);
                    return 1;
                }
                LeaveCriticalSection(&b->cs);
                ms = INFINITE; /* taken: its wake is coming */
                continue;
            }
            wait = ms - gone;
        }
        if (w->event) {
            WaitForSingleObject(w->event, wait);
        } else {
            Sleep(1);
        }
    }
    __sync_synchronize();
    return 0;
}

/* ------------------------------------------------------------------ SRW locks */

#define L_EXCL 1L
#define L_WAIT 2L
#define L_READER 4L
#define READERS(s) ((unsigned long)(s) >> 2)

static int can_take(long s, int exclusive) {
    if (exclusive) return (s & ~L_WAIT) == 0;
    /* shared: no writer holds it, and no one waits while readers hold it (a waiting writer goes first) */
    return !(s & L_EXCL) && !((s & L_WAIT) && READERS(s) > 0);
}

static int try_take(volatile long *l, int exclusive) {
    long s = *l;
    while (can_take(s, exclusive)) {
        long n = exclusive ? (s | L_EXCL) : (s + L_READER);
        long was = cas(l, s, n);
        if (was == s) return 1;
        s = was;
    }
    return 0;
}

/* parks until the lock may be free again (or returns at once if it already is) */
static void park_lock(volatile long *l, int exclusive) {
    Bucket *b = bucket_of(l);
    Waiter w;
    long s;
    w.addr = l;
    w.event = my_event();
    w.queued = 1;
    w.taken = 0;
    EnterCriticalSection(&b->cs);
    for (;;) {
        s = *l;
        if (can_take(s, exclusive)) {
            LeaveCriticalSection(&b->cs);
            return;
        }
        if ((s & L_WAIT) || cas(l, s, s | L_WAIT) == s) break;
    }
    enqueue(b, &w);
    LeaveCriticalSection(&b->cs);
    wait_queued(&w, b, INFINITE);
}

static void take(volatile long *l, int exclusive) {
    int spins = 0;
    ULONG err;
    if (try_take(l, exclusive)) return;
    err = GetLastError();
    lot_init();
    for (;;) {
        if (try_take(l, exclusive)) break;
        if (spins < 40) { spins++; __builtin_ia32_pause(); continue; }
        park_lock(l, exclusive);
    }
    SetLastError(err);
}

/* the "waiting" bit was set: under the bucket lock, apply `release` (an exclusive holder's bit, or one
 * reader), and if the lock is then free (no holder), clear "waiting" and wake everyone queued on it */
static void release_slow(volatile long *l, int exclusive) {
    Bucket *b = bucket_of(l);
    Waiter *list = 0;
    ULONG err = GetLastError();
    long s, n;
    lot_init();
    EnterCriticalSection(&b->cs);
    s = *l;
    for (;;) {
        n = exclusive ? (s & ~L_EXCL) : (s - L_READER);
        if ((n & L_WAIT) && !(n & L_EXCL) && READERS(n) == 0) n &= ~L_WAIT;
        {
            long was = cas(l, s, n);
            if (was == s) break;
            s = was;
        }
    }
    if ((s & L_WAIT) && !(n & L_WAIT)) list = dequeue(b, l, 1 /* all */);
    LeaveCriticalSection(&b->cs);
    wake_list(list);
    SetLastError(err);
}

static void give(volatile long *l, int exclusive) {
    long s = *l;
    for (;;) {
        long n;
        if (s & L_WAIT) {
            release_slow(l, exclusive);
            return;
        }
        n = exclusive ? (s & ~L_EXCL) : (s - L_READER);
        {
            long was = cas(l, s, n);
            if (was == s) return;
            s = was;
        }
    }
}

/* ------------------------------------------------------------------ condition variables */

static BOOL cv_sleep(void *cv, volatile long *lock, ULONG ms, ULONG flags) {
    int shared = (flags & 1) != 0; /* CONDITION_VARIABLE_LOCKMODE_SHARED */
    ULONG err = GetLastError();
    Bucket *b;
    Waiter w;
    int timed_out;
    lot_init();
    b = bucket_of(cv);
    w.addr = cv;
    w.event = my_event();
    w.queued = 1;
    w.taken = 0;
    EnterCriticalSection(&b->cs);
    enqueue(b, &w);
    LeaveCriticalSection(&b->cs);
    give(lock, !shared);
    timed_out = wait_queued(&w, b, ms);
    take(lock, !shared);
    if (timed_out) {
        SetLastError(ERROR_TIMEOUT);
        return 0;
    }
    SetLastError(err);
    return 1;
}

static void cv_wake(void *cv, int all) {
    Bucket *b;
    Waiter *list;
    ULONG err = GetLastError();
    lot_init();
    b = bucket_of(cv);
    EnterCriticalSection(&b->cs);
    list = dequeue(b, cv, all);
    LeaveCriticalSection(&b->cs);
    wake_list(list);
    SetLastError(err);
}

/* ------------------------------------------------------------------ which implementation */

typedef void(WINAPI *LockFn)(void *);
typedef unsigned char(WINAPI *TryFn)(void *);
typedef BOOL(WINAPI *SleepCvFn)(void *, void *, ULONG, ULONG);
static struct {
    LockFn acq_ex, acq_sh, rel_ex, rel_sh, wake, wake_all;
    TryFn try_ex;
    SleepCvFn sleep_cv;
} win_sync;
static volatile long sync_impl; /* 0 = not decided, 1 = Windows', 2 = ours */

static int windows_sync(void) {
    long m = sync_impl;
    if (!m) {
        void *k = GetModuleHandleW(KERNEL32);
        ULONG err = GetLastError();
        win_sync.acq_ex = (LockFn)GetProcAddress(k, "AcquireSRWLockExclusive");
        win_sync.acq_sh = (LockFn)GetProcAddress(k, "AcquireSRWLockShared");
        win_sync.rel_ex = (LockFn)GetProcAddress(k, "ReleaseSRWLockExclusive");
        win_sync.rel_sh = (LockFn)GetProcAddress(k, "ReleaseSRWLockShared");
        win_sync.try_ex = (TryFn)GetProcAddress(k, "TryAcquireSRWLockExclusive");
        win_sync.sleep_cv = (SleepCvFn)GetProcAddress(k, "SleepConditionVariableSRW");
        win_sync.wake = (LockFn)GetProcAddress(k, "WakeConditionVariable");
        win_sync.wake_all = (LockFn)GetProcAddress(k, "WakeAllConditionVariable");
        SetLastError(err);
        m = (win_sync.acq_ex && win_sync.acq_sh && win_sync.rel_ex && win_sync.rel_sh && win_sync.try_ex &&
             win_sync.sleep_cv && win_sync.wake && win_sync.wake_all)
                ? 1
                : 2;
#ifdef XP_SYNC_TEST
        if (xp_sync_force_ours) m = 2;
#endif
        __sync_synchronize();
        sync_impl = m;
    }
    return m == 1;
}

void WINAPI xp_AcquireSRWLockExclusive(void *l) {
    if (windows_sync()) win_sync.acq_ex(l); else take((volatile long *)l, 1);
}
void WINAPI xp_AcquireSRWLockShared(void *l) {
    if (windows_sync()) win_sync.acq_sh(l); else take((volatile long *)l, 0);
}
void WINAPI xp_ReleaseSRWLockExclusive(void *l) {
    if (windows_sync()) win_sync.rel_ex(l); else give((volatile long *)l, 1);
}
void WINAPI xp_ReleaseSRWLockShared(void *l) {
    if (windows_sync()) win_sync.rel_sh(l); else give((volatile long *)l, 0);
}
unsigned char WINAPI xp_TryAcquireSRWLockExclusive(void *l) {
    if (windows_sync()) return win_sync.try_ex(l);
    return (unsigned char)try_take((volatile long *)l, 1);
}
BOOL WINAPI xp_SleepConditionVariableSRW(void *cv, void *l, ULONG ms, ULONG flags) {
    if (windows_sync()) return win_sync.sleep_cv(cv, l, ms, flags);
    return cv_sleep(cv, (volatile long *)l, ms, flags);
}
void WINAPI xp_WakeConditionVariable(void *cv) {
    if (windows_sync()) win_sync.wake(cv); else cv_wake(cv, 0);
}
void WINAPI xp_WakeAllConditionVariable(void *cv) {
    if (windows_sync()) win_sync.wake_all(cv); else cv_wake(cv, 1);
}

XP_ANSWER(xp_AcquireSRWLockExclusive, AcquireSRWLockExclusive, 4)
XP_ANSWER(xp_AcquireSRWLockShared, AcquireSRWLockShared, 4)
XP_ANSWER(xp_ReleaseSRWLockExclusive, ReleaseSRWLockExclusive, 4)
XP_ANSWER(xp_ReleaseSRWLockShared, ReleaseSRWLockShared, 4)
XP_ANSWER(xp_TryAcquireSRWLockExclusive, TryAcquireSRWLockExclusive, 4)
XP_ANSWER(xp_SleepConditionVariableSRW, SleepConditionVariableSRW, 16)
XP_ANSWER(xp_WakeConditionVariable, WakeConditionVariable, 4)
XP_ANSWER(xp_WakeAllConditionVariable, WakeAllConditionVariable, 4)

/* ------------------------------------------------------------------ InitOnce */

typedef BOOL(WINAPI *OnceBeginFn)(void *, ULONG, BOOL *, void **);
typedef BOOL(WINAPI *OnceDoneFn)(void *, ULONG, void *);
static OnceBeginFn win_once_begin;
static OnceDoneFn win_once_done;
static volatile long once_impl;

static int windows_once(void) {
    long m = once_impl;
    if (!m) {
        void *k = GetModuleHandleW(KERNEL32);
        ULONG err = GetLastError();
        win_once_begin = (OnceBeginFn)GetProcAddress(k, "InitOnceBeginInitialize");
        win_once_done = (OnceDoneFn)GetProcAddress(k, "InitOnceComplete");
        SetLastError(err);
        m = (win_once_begin && win_once_done) ? 1 : 2;
#ifdef XP_SYNC_TEST
        if (xp_sync_force_ours) m = 2;
#endif
        __sync_synchronize();
        once_impl = m;
    }
    return m == 1;
}

#define INIT_ONCE_CHECK_ONLY 1UL
#define INIT_ONCE_ASYNC 2UL
#define INIT_ONCE_INIT_FAILED 4UL

BOOL WINAPI xp_InitOnceBeginInitialize(void *once, ULONG flags, BOOL *pending, void **context) {
    void *volatile *p = (void *volatile *)once;
    if (windows_once()) return win_once_begin(once, flags, pending, context);
    if (flags & INIT_ONCE_ASYNC) { /* (not used by RapidR's programs: refused, never half done) */
        SetLastError(ERROR_INVALID_PARAMETER);
        return 0;
    }
    for (;;) {
        void *v = *p;
        if ((unsigned long)v & 2) { /* done */
            __sync_synchronize();
            *pending = 0;
            if (context) *context = (void *)((unsigned long)v & ~3UL);
            return 1;
        }
        if (flags & INIT_ONCE_CHECK_ONLY) { /* not done yet (not used by RapidR's programs) */
            SetLastError(ERROR_GEN_FAILURE);
            return 0;
        }
        if (v == 0 && cas_ptr(p, 0, (void *)1) == 0) {
            *pending = 1; /* this caller does it */
            return 1;
        }
        Sleep(v == 0 ? 0 : 1); /* someone else is doing it */
    }
}

BOOL WINAPI xp_InitOnceComplete(void *once, ULONG flags, void *context) {
    void *volatile *p = (void *volatile *)once;
    if (windows_once()) return win_once_done(once, flags, context);
    if (*p != (void *)1 || ((unsigned long)context & 3)) {
        SetLastError(ERROR_INVALID_PARAMETER);
        return 0;
    }
    __sync_synchronize();
    *p = (flags & INIT_ONCE_INIT_FAILED) ? (void *)0 : (void *)((unsigned long)context | 2);
    __sync_synchronize();
    return 1;
}

XP_ANSWER(xp_InitOnceBeginInitialize, InitOnceBeginInitialize, 16)
XP_ANSWER(xp_InitOnceComplete, InitOnceComplete, 12)

/* ------------------------------------------------------------------ FLS */

typedef void(WINAPI *FlsCallback)(void *);
typedef ULONG(WINAPI *FlsAllocFn)(FlsCallback);
typedef BOOL(WINAPI *FlsSetFn)(ULONG, void *);
typedef BOOL(WINAPI *FlsFreeFn)(ULONG);
typedef BOOL(WINAPI *FiberFn)(void);
static FlsAllocFn win_fls_alloc;
static FlsSetFn win_fls_set;
static FlsFreeFn win_fls_free;
static volatile long fls_impl;

#define FLS_MAX 64
static struct {
    volatile ULONG slot; /* the TLS slot (FLS index = TLS slot), 0xFFFFFFFF = free entry */
    FlsCallback callback;
} fls_table[FLS_MAX];
static volatile long fls_count;
static volatile long fls_lock;

static int windows_fls(void) {
    long m = fls_impl;
    if (!m) {
        void *k = GetModuleHandleW(KERNEL32);
        ULONG err = GetLastError();
        win_fls_alloc = (FlsAllocFn)GetProcAddress(k, "FlsAlloc");
        win_fls_set = (FlsSetFn)GetProcAddress(k, "FlsSetValue");
        win_fls_free = (FlsFreeFn)GetProcAddress(k, "FlsFree");
        SetLastError(err);
        m = (win_fls_alloc && win_fls_set && win_fls_free) ? 1 : 2;
#ifdef XP_SYNC_TEST
        if (xp_sync_force_ours) m = 2;
#endif
        __sync_synchronize();
        fls_impl = m;
    }
    return m == 1;
}

static void fls_lock_take(void) { while (!__sync_bool_compare_and_swap(&fls_lock, 0, 1)) Sleep(0); }
static void fls_lock_give(void) { __sync_lock_release(&fls_lock); }

ULONG WINAPI xp_FlsAlloc(FlsCallback callback) {
    ULONG slot;
    int i;
    if (windows_fls()) return win_fls_alloc(callback);
    slot = TlsAlloc();
    if (slot == FLS_OUT_OF_INDEXES) return slot;
    fls_lock_take();
    for (i = 0; i < fls_count && fls_table[i].slot != FLS_OUT_OF_INDEXES; i++) {}
    if (i == FLS_MAX) {
        fls_lock_give();
        TlsFree(slot);
        return FLS_OUT_OF_INDEXES;
    }
    fls_table[i].callback = callback;
    fls_table[i].slot = slot;
    if (i == fls_count) fls_count++;
    fls_lock_give();
    return slot;
}

BOOL WINAPI xp_FlsSetValue(ULONG index, void *value) {
    if (windows_fls()) return win_fls_set(index, value);
    return TlsSetValue(index, value);
}

/* FlsFree: the callback for this thread's value (Windows calls it for every thread's; RapidR's
 * programs free only indexes no thread has set), then the slot */
BOOL WINAPI xp_FlsFree(ULONG index) {
    FlsCallback cb = 0;
    void *value;
    int i;
    if (windows_fls()) return win_fls_free(index);
    fls_lock_take();
    for (i = 0; i < fls_count; i++) {
        if (fls_table[i].slot == index) {
            cb = fls_table[i].callback;
            fls_table[i].slot = FLS_OUT_OF_INDEXES;
            break;
        }
    }
    fls_lock_give();
    if (i == fls_count) {
        SetLastError(ERROR_INVALID_PARAMETER);
        return 0;
    }
    value = TlsGetValue(index);
    if (value && cb) {
        TlsSetValue(index, 0);
        cb(value);
    }
    return TlsFree(index);
}

/* a thread is ending: each FLS callback whose value is set on it, once, as Windows calls them (the
 * value cleared before its callback runs) */
static void fls_thread_ends(void) {
    int i;
    if (fls_impl != 2) return;
    for (i = 0; i < fls_count; i++) {
        ULONG slot;
        FlsCallback cb;
        void *value;
        fls_lock_take();
        slot = fls_table[i].slot;
        cb = fls_table[i].callback;
        fls_lock_give();
        if (slot == FLS_OUT_OF_INDEXES || !cb) continue;
        value = TlsGetValue(slot);
        if (!value) continue;
        TlsSetValue(slot, 0);
        cb(value);
    }
}

BOOL WINAPI xp_IsThreadAFiber(void) {
    static FiberFn win_fiber;
    static volatile long looked;
    if (!looked) {
        ULONG err = GetLastError();
        win_fiber = (FiberFn)GetProcAddress(GetModuleHandleW(KERNEL32), "IsThreadAFiber");
        SetLastError(err);
        __sync_synchronize();
        looked = 1;
    }
    return win_fiber ? win_fiber() : 0; /* (XP: RapidR's programs make no fibers) */
}

XP_ANSWER(xp_FlsAlloc, FlsAlloc, 4)
XP_ANSWER(xp_FlsSetValue, FlsSetValue, 8)
XP_ANSWER(xp_FlsFree, FlsFree, 4)
XP_ANSWER(xp_IsThreadAFiber, IsThreadAFiber, 0)

/* ------------------------------------------------------------------ the TLS directory and callback */

static void WINAPI xp_tls_callback(void *module, ULONG reason, void *reserved) {
    (void)module;
    (void)reserved;
    if (reason == 3) { /* DLL_THREAD_DETACH: a thread ends */
        ULONG err = GetLastError();
        fls_thread_ends();
        if (lot_state == 2) my_event_ends();
        SetLastError(err);
    }
}

/* IMAGE_TLS_DIRECTORY32 (PE/COFF specification, "The .tls Section"): the linker puts the program's TLS
 * directory where the symbol _tls_used is. No thread-local data of its own (4 bytes), one callback. */
typedef void(WINAPI *TlsCallback)(void *, ULONG, void *);
static ULONG xp_tls_index;
static char xp_tls_template[4];
static TlsCallback xp_tls_callbacks[2] = {xp_tls_callback, 0};
const struct {
    void *start, *end, *index, *callbacks;
    ULONG zero_fill, characteristics;
} xp_tls_directory __asm__("__tls_used") = {xp_tls_template, xp_tls_template + sizeof xp_tls_template, &xp_tls_index,
                                             xp_tls_callbacks, 0, 0};
