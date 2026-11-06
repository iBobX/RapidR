/*
 * test_sync.exe [ours]: a stress test of tools/winxp/shim/xp_sync.c (lane WINXP).
 *   (no argument)  the functions answer as in a program: Windows' own where Windows has them
 *   ours           xp_sync.c's own emulation even where Windows has its own (to test it on Windows 7+)
 * On plain Windows XP the emulation is used either way. Each test prints PASS or FAIL; a test that does
 * not end within its time is a HANG (exit 3). Exit 0 = all passed. kernel32 only, no C runtime,
 * PE subsystem 5.1 (tools/winxp/shim/tests/build.sh). Ours, from Microsoft's public documentation.
 */
typedef unsigned short wchar16;
typedef unsigned long ULONG;
typedef void *HANDLE;
typedef int BOOL;
#define WINAPI __attribute__((stdcall))
#define INFINITE 0xFFFFFFFFUL

/* the functions under test: called as Rust's std calls them (the import thunk names) */
void WINAPI AcquireSRWLockExclusive(void *);
void WINAPI AcquireSRWLockShared(void *);
void WINAPI ReleaseSRWLockExclusive(void *);
void WINAPI ReleaseSRWLockShared(void *);
unsigned char WINAPI TryAcquireSRWLockExclusive(void *);
BOOL WINAPI SleepConditionVariableSRW(void *, void *, ULONG, ULONG);
void WINAPI WakeConditionVariable(void *);
void WINAPI WakeAllConditionVariable(void *);
BOOL WINAPI InitOnceBeginInitialize(void *, ULONG, BOOL *, void **);
BOOL WINAPI InitOnceComplete(void *, ULONG, void *);
ULONG WINAPI FlsAlloc(void(WINAPI *)(void *));
BOOL WINAPI FlsSetValue(ULONG, void *);
BOOL WINAPI FlsFree(ULONG);
BOOL WINAPI IsThreadAFiber(void);
extern int xp_sync_force_ours;
/* and through the import slot, as C code calls them */
extern void *xp_slot_d_AcquireSRWLockExclusive __asm__("__imp__AcquireSRWLockExclusive@4");
extern void *xp_slot_u_AcquireSRWLockExclusive __asm__("__imp_AcquireSRWLockExclusive");
void WINAPI xp_AcquireSRWLockExclusive(void *);

__declspec(dllimport) HANDLE WINAPI GetStdHandle(ULONG);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, ULONG, ULONG *, void *);
__declspec(dllimport) void WINAPI ExitProcess(unsigned);
__declspec(dllimport) HANDLE WINAPI CreateThread(void *, ULONG, ULONG(WINAPI *)(void *), void *, ULONG, ULONG *);
__declspec(dllimport) ULONG WINAPI WaitForSingleObject(HANDLE, ULONG);
__declspec(dllimport) ULONG WINAPI WaitForMultipleObjects(ULONG, const HANDLE *, BOOL, ULONG);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE);
__declspec(dllimport) void WINAPI Sleep(ULONG);
__declspec(dllimport) ULONG WINAPI GetTickCount(void);
__declspec(dllimport) ULONG WINAPI GetLastError(void);
__declspec(dllimport) void WINAPI SetLastError(ULONG);
__declspec(dllimport) wchar16 *WINAPI GetCommandLineW(void);
__declspec(dllimport) void *WINAPI GetModuleHandleW(const wchar16 *);
__declspec(dllimport) void *WINAPI GetProcAddress(void *, const char *);
__declspec(dllimport) BOOL WINAPI SwitchToThread(void);

void *memset(void *d, int c, unsigned n) { char *a = d; while (n--) *a++ = (char)c; return d; }
void *memcpy(void *d, const void *s, unsigned n) { char *a = d; const char *b = s; while (n--) *a++ = *b++; return d; }

static HANDLE out;
static int failures;
static void say(const char *s) { ULONG n = 0, w; while (s[n]) n++; WriteFile(out, s, n, &w, 0); }
static void num(long v) { char b[16]; int i = 15; int neg = v < 0; unsigned long u = neg ? (unsigned long)-v : (unsigned long)v; b[i] = 0; do { b[--i] = (char)('0' + u % 10); u /= 10; } while (u); if (neg) b[--i] = '-'; say(b + i); }
static void result(const char *name, int ok, const char *detail, long a, long b) {
    say(ok ? "PASS  " : "FAIL  "); say(name); say("  "); say(detail); say(" "); num(a); say(" / "); num(b); say("\r\n");
    if (!ok) failures++;
}

typedef ULONG(WINAPI *ThreadFn)(void *);
/* runs n threads of fn(arg i), waits for all (limit ms): 0 = a hang */
static int run_threads(int n, ThreadFn fn, void **args, ULONG limit, const char *name) {
    HANDLE h[64];
    int i;
    ULONG r;
    for (i = 0; i < n; i++) h[i] = CreateThread(0, 0, fn, args ? args[i] : (void *)(long)i, 0, 0);
    r = WaitForMultipleObjects((ULONG)n, h, 1, limit);
    for (i = 0; i < n; i++) CloseHandle(h[i]);
    if (r == 0x102 /* WAIT_TIMEOUT */) {
        say("HANG  "); say(name); say("  (threads did not end in time)\r\n");
        ExitProcess(3);
    }
    return 1;
}

static long xadd(volatile long *p, long v) { return __sync_fetch_and_add(p, v); }
static unsigned long rnd(unsigned long *s) { *s = *s * 1103515245UL + 12345UL; return (*s >> 16) & 0x7fff; }

/* ---- T1 exclusive counter */
static void *lock1;
static volatile long count1;
static ULONG WINAPI t1(void *a) {
    int i;
    for (i = 0; i < 100000; i++) {
        AcquireSRWLockExclusive(&lock1);
        count1 = count1 + 1; /* (not atomic: the lock is what protects it) */
        ReleaseSRWLockExclusive(&lock1);
    }
    return 0;
}

/* ---- T2 try-lock */
static void *lock2;
static volatile long count2, wins2;
static ULONG WINAPI t2(void *a) {
    int i;
    for (i = 0; i < 50000; i++) {
        if (TryAcquireSRWLockExclusive(&lock2)) {
            count2 = count2 + 1;
            xadd(&wins2, 1);
            ReleaseSRWLockExclusive(&lock2);
        } else {
            __builtin_ia32_pause();
        }
    }
    return 0;
}

/* ---- T3 readers / writers */
static void *lock3;
static volatile long a3, b3, bad3, writers_left3, inside3, most3, reads3;
static ULONG WINAPI t3w(void *a) {
    int i, k;
    for (i = 0; i < 20000; i++) {
        AcquireSRWLockExclusive(&lock3);
        a3 = a3 + 1;
        for (k = 0; k < 20; k++) __builtin_ia32_pause();
        b3 = b3 + 1;
        ReleaseSRWLockExclusive(&lock3);
    }
    xadd(&writers_left3, -1);
    return 0;
}
static ULONG WINAPI t3r(void *a) {
    while (writers_left3 > 0) {
        long now;
        AcquireSRWLockShared(&lock3);
        now = xadd(&inside3, 1) + 1;
        if (now > most3) most3 = now;
        if (a3 != b3) xadd(&bad3, 1);
        xadd(&reads3, 1);
        xadd(&inside3, -1);
        ReleaseSRWLockShared(&lock3);
    }
    return 0;
}

/* ---- T4 bounded queue with condition variables */
#define CAP4 8
static void *lock4, *not_empty4, *not_full4;
static long queue4[CAP4], head4, size4, produced_done4;
static volatile long sum_in4, sum_out4, count_out4, timeouts4;
static ULONG WINAPI t4p(void *a) {
    long i, base = (long)a * 100000;
    for (i = 1; i <= 25000; i++) {
        AcquireSRWLockExclusive(&lock4);
        while (size4 == CAP4) SleepConditionVariableSRW(&not_full4, &lock4, INFINITE, 0);
        queue4[(head4 + size4) % CAP4] = base + i;
        size4++;
        xadd(&sum_in4, base + i);
        ReleaseSRWLockExclusive(&lock4);
        WakeConditionVariable(&not_empty4);
    }
    return 0;
}
static ULONG WINAPI t4c(void *a) {
    int timed = ((long)a & 1) != 0;
    for (;;) {
        long v;
        AcquireSRWLockExclusive(&lock4);
        while (size4 == 0 && !produced_done4) {
            if (!SleepConditionVariableSRW(&not_empty4, &lock4, timed ? 1 : INFINITE, 0)) {
                if (GetLastError() == 1460) xadd(&timeouts4, 1);
            }
        }
        if (size4 == 0 && produced_done4) {
            ReleaseSRWLockExclusive(&lock4);
            return 0;
        }
        v = queue4[head4];
        head4 = (head4 + 1) % CAP4;
        size4--;
        ReleaseSRWLockExclusive(&lock4);
        WakeConditionVariable(&not_full4);
        xadd(&sum_out4, v);
        xadd(&count_out4, 1);
    }
}

/* ---- T6 wake all / T7 shared-mode waits */
static void *lock6, *cv6;
static volatile long flag6, woke6;
static ULONG WINAPI t6(void *a) {
    AcquireSRWLockExclusive(&lock6);
    while (!flag6) SleepConditionVariableSRW(&cv6, &lock6, INFINITE, 0);
    ReleaseSRWLockExclusive(&lock6);
    xadd(&woke6, 1);
    return 0;
}
static void *lock7, *cv7;
static volatile long flag7, woke7, waiting7;
static ULONG WINAPI t7(void *a) {
    AcquireSRWLockShared(&lock7);
    xadd(&waiting7, 1);
    while (!flag7) SleepConditionVariableSRW(&cv7, &lock7, INFINITE, 1 /* CONDITION_VARIABLE_LOCKMODE_SHARED */);
    ReleaseSRWLockShared(&lock7);
    xadd(&woke7, 1);
    return 0;
}

/* ---- T8 InitOnce */
static void *once8;
static volatile long pending8, right8, wrong8;
static ULONG WINAPI t8(void *a) {
    BOOL pending = 0;
    void *ctx = 0;
    if (!InitOnceBeginInitialize(&once8, 0, &pending, &ctx)) { xadd(&wrong8, 1); return 0; }
    if (pending) {
        xadd(&pending8, 1);
        Sleep(50);
        InitOnceComplete(&once8, 0, (void *)0x1000);
        xadd(&right8, 1);
    } else if (ctx == (void *)0x1000) {
        xadd(&right8, 1);
    } else {
        xadd(&wrong8, 1);
    }
    return 0;
}

/* ---- T9 FLS destructors */
static ULONG key9;
static volatile long calls9, sum9;
static void WINAPI dtor9(void *v) {
    xadd(&calls9, 1);
    xadd(&sum9, (long)v);
}
static ULONG WINAPI t9(void *a) {
    long i = (long)a;
    if (i < 6) FlsSetValue(key9, (void *)(i + 1)); /* 1..6 */
    if (i == 6) FlsSetValue(key9, (void *)100);
    /* i == 7: never sets: no call */
    return 0;
}

/* ---- T10 last error kept */
static void *lock10;
static volatile long kept10, lost10;
static ULONG WINAPI t10(void *a) {
    int i;
    for (i = 0; i < 20000; i++) {
        SetLastError(4321);
        AcquireSRWLockExclusive(&lock10);
        ReleaseSRWLockExclusive(&lock10);
        if (GetLastError() == 4321) xadd(&kept10, 1); else xadd(&lost10, 1);
    }
    return 0;
}

/* ---- T12 a random mix: must simply end */
static void *lock12, *cv12;
static volatile long shared12;
static ULONG WINAPI t12(void *a) {
    unsigned long seed = 12345 + (unsigned long)(long)a * 7919;
    int i;
    for (i = 0; i < 30000; i++) {
        switch (rnd(&seed) % 6) {
        case 0: AcquireSRWLockExclusive(&lock12); shared12++; ReleaseSRWLockExclusive(&lock12); break;
        case 1: AcquireSRWLockShared(&lock12); (void)shared12; ReleaseSRWLockShared(&lock12); break;
        case 2: if (TryAcquireSRWLockExclusive(&lock12)) { shared12++; ReleaseSRWLockExclusive(&lock12); } break;
        case 3: AcquireSRWLockExclusive(&lock12); SleepConditionVariableSRW(&cv12, &lock12, rnd(&seed) % 3, 0); ReleaseSRWLockExclusive(&lock12); break;
        case 4: AcquireSRWLockShared(&lock12); SleepConditionVariableSRW(&cv12, &lock12, rnd(&seed) % 2, 1); ReleaseSRWLockShared(&lock12); break;
        default: if (rnd(&seed) & 1) WakeConditionVariable(&cv12); else WakeAllConditionVariable(&cv12); break;
        }
    }
    return 0;
}

/* ---- T13 timed waits racing wakes (the waiter's time runs out while a waker takes it) */
static void *lock13, *cv13;
static volatile long stop13, waits13, wakes13;
static ULONG WINAPI t13w(void *a) {
    while (!stop13) {
        AcquireSRWLockExclusive(&lock13);
        SleepConditionVariableSRW(&cv13, &lock13, ((long)a & 1) ? 0 : 1, 0);
        ReleaseSRWLockExclusive(&lock13);
        xadd(&waits13, 1);
    }
    return 0;
}
static ULONG WINAPI t13k(void *a) {
    while (!stop13) {
        if ((long)a & 1) WakeAllConditionVariable(&cv13); else WakeConditionVariable(&cv13);
        xadd(&wakes13, 1);
    }
    return 0;
}

void test_main(void) {
    wchar16 *c = GetCommandLineW();
    static const wchar16 K32[] = {'k', 'e', 'r', 'n', 'e', 'l', '3', '2', '.', 'd', 'l', 'l', 0};
    int real_here = GetProcAddress(GetModuleHandleW(K32), "TryAcquireSRWLockExclusive") != 0;
    ULONG start;
    int i;
    out = GetStdHandle((ULONG)-11);
    for (; *c; c++) if (c[0] == 'o' && c[1] == 'u' && c[2] == 'r' && c[3] == 's') xp_sync_force_ours = 1;
    say("test_sync: ");
    say(xp_sync_force_ours || !real_here ? "xp_sync's own emulation" : "Windows' own functions");
    say("\r\n");

    result("T0 slots", xp_slot_d_AcquireSRWLockExclusive == (void *)xp_AcquireSRWLockExclusive && xp_slot_u_AcquireSRWLockExclusive == (void *)xp_AcquireSRWLockExclusive, "import slots aimed at ours", 1, 1);

    start = GetTickCount();
    run_threads(8, t1, 0, 120000, "T1");
    result("T1 exclusive", count1 == 800000, "count", count1, 800000);

    run_threads(4, t2, 0, 120000, "T2");
    AcquireSRWLockExclusive(&lock2);
    i = TryAcquireSRWLockExclusive(&lock2);
    ReleaseSRWLockExclusive(&lock2);
    result("T2 try-lock", count2 == wins2 && wins2 > 0 && i == 0, "count = wins, and held lock refuses the same thread", count2, wins2);

    writers_left3 = 3;
    {
        void *args[8] = {0, 0, 0, 0, 0, 0, 0, 0};
        HANDLE h[8];
        ULONG r;
        for (i = 0; i < 3; i++) h[i] = CreateThread(0, 0, t3w, args[i], 0, 0);
        for (i = 3; i < 8; i++) h[i] = CreateThread(0, 0, t3r, args[i], 0, 0);
        r = WaitForMultipleObjects(8, h, 1, 180000);
        for (i = 0; i < 8; i++) CloseHandle(h[i]);
        if (r == 0x102) { say("HANG  T3\r\n"); ExitProcess(3); }
    }
    result("T3 readers/writers", bad3 == 0 && a3 == 60000 && b3 == 60000 && reads3 > 0, "torn reads, writes", bad3, a3);
    say("      (most readers inside at once: "); num(most3); say(", reads: "); num(reads3); say(")\r\n");

    {
        HANDLE h[8];
        ULONG r;
        for (i = 0; i < 4; i++) h[i] = CreateThread(0, 0, t4p, (void *)(long)i, 0, 0);
        for (i = 4; i < 8; i++) h[i] = CreateThread(0, 0, t4c, (void *)(long)i, 0, 0);
        r = WaitForMultipleObjects(4, h, 1, 180000);
        if (r == 0x102) { say("HANG  T4 producers\r\n"); ExitProcess(3); }
        AcquireSRWLockExclusive(&lock4);
        produced_done4 = 1;
        ReleaseSRWLockExclusive(&lock4);
        WakeAllConditionVariable(&not_empty4);
        r = WaitForMultipleObjects(4, h + 4, 1, 60000);
        for (i = 0; i < 8; i++) CloseHandle(h[i]);
        if (r == 0x102) { say("HANG  T4 consumers\r\n"); ExitProcess(3); }
    }
    result("T4 queue", count_out4 == 100000 && sum_out4 == sum_in4, "items out, sums equal", count_out4, 100000);
    say("      (1 ms waits that timed out: "); num(timeouts4); say(")\r\n");

    {
        void *l = 0, *cv = 0;
        ULONG t0, gone, err;
        BOOL ok;
        AcquireSRWLockExclusive(&l);
        t0 = GetTickCount();
        SetLastError(0);
        ok = SleepConditionVariableSRW(&cv, &l, 200, 0);
        err = GetLastError();
        gone = GetTickCount() - t0;
        i = TryAcquireSRWLockExclusive(&l); /* still held by us after the wait */
        ReleaseSRWLockExclusive(&l);
        result("T5 timeout", !ok && err == 1460 && gone >= 150 && gone < 2000 && i == 0, "ms waited (150..2000), error 1460", (long)gone, (long)err);
    }

    {
        HANDLE h[12];
        ULONG r;
        for (i = 0; i < 12; i++) h[i] = CreateThread(0, 0, t6, 0, 0, 0);
        Sleep(300);
        AcquireSRWLockExclusive(&lock6);
        flag6 = 1;
        ReleaseSRWLockExclusive(&lock6);
        WakeAllConditionVariable(&cv6);
        r = WaitForMultipleObjects(12, h, 1, 20000);
        for (i = 0; i < 12; i++) CloseHandle(h[i]);
        result("T6 wake all", r != 0x102 && woke6 == 12, "woken", woke6, 12);
    }
    {
        HANDLE h[4];
        ULONG r;
        for (i = 0; i < 4; i++) h[i] = CreateThread(0, 0, t7, 0, 0, 0);
        while (waiting7 < 4) Sleep(10);
        Sleep(100);
        AcquireSRWLockExclusive(&lock7); /* every waiter gave its shared hold back while waiting */
        flag7 = 1;
        ReleaseSRWLockExclusive(&lock7);
        WakeAllConditionVariable(&cv7);
        r = WaitForMultipleObjects(4, h, 1, 20000);
        for (i = 0; i < 4; i++) CloseHandle(h[i]);
        result("T7 shared-mode waits", r != 0x102 && woke7 == 4, "woken", woke7, 4);
    }

    run_threads(16, t8, 0, 60000, "T8");
    {
        void *once = 0, *ctx = 0;
        BOOL p1 = 0, p2 = 0, p3 = 0;
        InitOnceBeginInitialize(&once, 0, &p1, &ctx);
        InitOnceComplete(&once, 4 /* INIT_ONCE_INIT_FAILED */, 0);
        InitOnceBeginInitialize(&once, 0, &p2, &ctx); /* failed: begun again */
        InitOnceComplete(&once, 0, (void *)0x2000);
        InitOnceBeginInitialize(&once, 0, &p3, &ctx);
        result("T8 init once", pending8 == 1 && right8 == 16 && wrong8 == 0 && p1 && p2 && !p3 && ctx == (void *)0x2000, "one did it, all saw it; failed, then again", pending8, right8);
    }

    key9 = FlsAlloc(dtor9);
    {
        ULONG unused = FlsAlloc(dtor9);
        BOOL freed = FlsFree(unused);
        run_threads(8, t9, 0, 30000, "T9");
        /* the thread has ended when its handle is signalled: its callbacks have run */
        result("T9 FLS destructors", calls9 == 7 && sum9 == 21 + 100 && freed && key9 != 0xFFFFFFFFUL, "calls (7 threads set a value, 1 did not), sum 121", calls9, sum9);
        result("T9 IsThreadAFiber", IsThreadAFiber() == 0, "not a fiber", 0, 0);
    }

    run_threads(4, t10, 0, 60000, "T10");
    result("T10 last error kept", lost10 == 0 && kept10 == 80000, "kept, lost", kept10, lost10);

    {
        void *l = 0;
        int a, b;
        AcquireSRWLockShared(&l);
        AcquireSRWLockShared(&l); /* a second shared hold of one thread, no writer waiting: allowed */
        a = TryAcquireSRWLockExclusive(&l);
        ReleaseSRWLockShared(&l);
        ReleaseSRWLockShared(&l);
        b = TryAcquireSRWLockExclusive(&l);
        if (b) ReleaseSRWLockExclusive(&l);
        result("T11 recursion", a == 0 && b == 1, "exclusive refused while shared, then free", a, b);
    }

    run_threads(6, t12, 0, 240000, "T12");
    result("T12 random mix", 1, "ended (no hang)", 0, 0);

    {
        HANDLE h[10];
        ULONG r;
        for (i = 0; i < 8; i++) h[i] = CreateThread(0, 0, t13w, (void *)(long)i, 0, 0);
        for (i = 8; i < 10; i++) h[i] = CreateThread(0, 0, t13k, (void *)(long)i, 0, 0);
        Sleep(8000);
        stop13 = 1;
        WakeAllConditionVariable(&cv13);
        r = WaitForMultipleObjects(10, h, 1, 60000);
        for (i = 0; i < 10; i++) CloseHandle(h[i]);
        if (r == 0x102) { say("HANG  T13\r\n"); ExitProcess(3); }
        result("T13 timed waits vs wakes", waits13 > 0 && wakes13 > 0, "waits, wakes (8 s, no crash)", waits13, wakes13);
    }

    say("\r\n"); say(failures ? "RESULT: FAILED " : "RESULT: all passed "); num(failures); say(" in "); num((long)(GetTickCount() - start)); say(" ms\r\n");
    ExitProcess(failures ? 1 : 0);
}
