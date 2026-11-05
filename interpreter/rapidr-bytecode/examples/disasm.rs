//! `cargo run -p rapidr-bytecode --example disasm -- file.rrbc`: a module's
//! code as text (rapidr_bytecode::disasm), for working on the VM and the JIT.

fn main() {
    let path = std::env::args().nth(1).expect("usage: disasm <file.rrbc>");
    let bytes = std::fs::read(&path).expect("read");
    let m = rapidr_bytecode::Module::from_bytes(&bytes).expect("decode");
    print!("{}", rapidr_bytecode::disasm::module(&m));
}
