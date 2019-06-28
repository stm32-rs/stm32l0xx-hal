use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let linker = match (cfg!(feature = "stm32l0x1"), cfg!(feature = "stm32l0x2")) {
        (false, false) | (true, true) => {
            panic!("\n\nMust select exactly one package for linker script generation!\nChoices: 'stm32l0x1' or 'stm32l0x2'\n\n");
        }
        (true, false) => {
            include_bytes!("memory_l0x1.x").as_ref()
        }
        (false, true) => {
            include_bytes!("memory_l0x2.x").as_ref()
        }
    };

    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(linker)
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=memory_l0x1.x");
    println!("cargo:rerun-if-changed=memory_l0x2.x");
}