use std::env;
use std::fs::{self, File};
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

    if !cfg!(feature = "disable-linker-script") {
        if feature_count != 1 {
            panic!("\n\nMust select exactly one package for linker script generation!\nChoices: 'stm32l0x1' or 'stm32l0x2' or 'stm32l0x3'\nAlternatively, pick the mcu-feature that matches your MCU, for example 'mcu-STM32L071KBTx'\n\n");
        }

        let flash_features: Vec<u32> = [
            (8, cfg!(feature = "flash-8")),
            (16, cfg!(feature = "flash-16")),
            (32, cfg!(feature = "flash-32")),
            (64, cfg!(feature = "flash-64")),
            (128, cfg!(feature = "flash-128")),
            (192, cfg!(feature = "flash-192")),
        ]
        .iter()
        .filter(|(_, f)| *f)
        .map(|(f, _)| *f)
        .collect();

        if flash_features.len() != 1 {
            panic!("\n\nMust select exactly one flash size for linker script generation!\n\
            Choices: 'flash-8', 'flash-16', 'flash-32', 'flash-64', 'flash-128' or 'flash-192'\n \
            Alternatively, pick the mcu-feature that matches your MCU, for example 'mcu-STM32L071KBTx'\n\n");
        }

        let flash_size = flash_features[0];

        let ram_features: Vec<u32> = [
            (2, cfg!(feature = "ram-2")),
            (8, cfg!(feature = "ram-8")),
            (20, cfg!(feature = "ram-20")),
        ]
        .iter()
        .filter(|(_, f)| *f)
        .map(|(f, _)| *f)
        .collect();

        if ram_features.len() != 1 {
            panic!("\n\nMust select exactly one ram size for linker script generation!\n\
            Choices: 'ram-2', 'ram-8' or 'ram-20'\n \
            Alternatively, pick the mcu-feature that matches your MCU, for example 'mcu-STM32L071KBTx'\n\n");
        }

        let ram_size = ram_features[0];

        let linker = format!(
            r#"MEMORY
{{
    FLASH : ORIGIN = 0x08000000, LENGTH = {}K
    RAM : ORIGIN = 0x20000000, LENGTH = {}K
}}"#,
            flash_size, ram_size
        );

        File::create(out.join("memory.x"))
            .unwrap()
            .write_all(linker.as_bytes())
            .unwrap();
        println!("cargo:rustc-link-search={}", out.display());
    }

    println!("cargo:rerun-if-changed=build.rs");

    // Copy the binary blob required by the Flash API somewhere the linker can
    // find it, and tell Cargo to link it.

    let blob_name = "flash";
    let blob_file = format!("lib{}.a", blob_name);
    let blob_path = format!("flash-code/{}", blob_file);

    fs::copy(&blob_path, out.join(blob_file)).expect("Failed to copy binary blob for Flash API");

    println!("cargo:rustc-link-lib=static={}", blob_name);
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rerun-if-changed={}", blob_path);
}
