/*
 * XP_ANSWER(ours, Function, bytes): makes `ours` (a stdcall function of this object whose arguments take
 * `bytes` bytes) the answer to every name a caller of Windows' `Function` can use in RapidR's programs:
 *   _Function@bytes        the import thunk: Rust's std calls it (its bindings are not dllimport);
 *   __imp__Function@bytes  the import slot that C code (dllimport, decorated stdcall) loads;
 *   __imp_Function         the slot of the crates built with --cfg windows_raw_dylib (undecorated).
 * An object's own definitions come before any library's, so the import library's entry (and, when no
 * other function of it is left, the DLL) never gets into the program's import table.
 * 32-bit x86 only (stdcall decoration). Lane WINXP; ours, from the PE/COFF import naming.
 */
#ifndef XP_ANSWER_H
#define XP_ANSWER_H
#define XP_STR2(x) #x
#define XP_STR(x) XP_STR2(x)
#define XP_ANSWER(ours, name, bytes)                                                                      \
    __asm__(".text\n\t.globl _" #name "@" #bytes "\n\t.p2align 4\n_" #name "@" #bytes ":\n\tjmp _" #ours \
            "@" #bytes "\n");                                                                             \
    void *xp_slot_d_##name __asm__("__imp__" #name "@" #bytes) = (void *)ours;                            \
    void *xp_slot_u_##name __asm__("__imp_" #name) = (void *)ours;
#endif
