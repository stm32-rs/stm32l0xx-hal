use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut feature_count = 0;

    if cfg!(feature = "stm32l0x1") {
        feature_count += 1;
    }

    if cfg!(feature = "stm32l0x2") {
        feature_count += 1;
    }

    if cfg!(feature = "stm32l0x3") {
        feature_count += 1;
    }

    if feature_count != 1 {
        panic!("\n\nMust select exactly one package for linker script generation!\nChoices: 'stm32l0x1' or 'stm32l0x2' or 'stm32l0x3'\n\n");
    }

    if !cfg!(feature = "disable-linker-script") {
        let linker = if cfg!(feature = "stm32l0x1") {
            include_bytes!("memory_l0x1.x").as_ref()
        } else if cfg!(feature = "stm32l0x2") {
            include_bytes!("memory_l0x2.x").as_ref()
        } else if cfg!(feature = "stm32l0x3") {
            include_bytes!("memory_l0x3.x").as_ref()
        } else {
            unreachable!();
        };

        File::create(out.join("memory.x"))
            .unwrap()
            .write_all(linker)
            .unwrap();
        println!("cargo:rustc-link-search={}", out.display());

        println!("cargo:rerun-if-changed=memory_l0x1.x");
        println!("cargo:rerun-if-changed=memory_l0x2.x");
        println!("cargo:rerun-if-changed=memory_l0x3.x");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
