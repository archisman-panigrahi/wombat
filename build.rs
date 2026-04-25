use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");

    let cargo_lock_path = Path::new("Cargo.lock");
    let cargo_lock = fs::read_to_string(cargo_lock_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", cargo_lock_path.display()));

    let mut in_numbat_package = false;
    let mut numbat_version = None;

    for line in cargo_lock.lines() {
        let trimmed = line.trim();

        if trimmed == "name = \"numbat\"" {
            in_numbat_package = true;
            continue;
        }

        if in_numbat_package && trimmed.starts_with("version = ") {
            let version = trimmed
                .strip_prefix("version = \"")
                .and_then(|value| value.strip_suffix('"'))
                .unwrap_or_else(|| panic!("unexpected numbat version line: {trimmed}"));
            numbat_version = Some(version.to_string());
            break;
        }

        if in_numbat_package && trimmed.is_empty() {
            in_numbat_package = false;
        }
    }

    let numbat_version = numbat_version.expect("could not find numbat version in Cargo.lock");
    println!("cargo:rustc-env=NUMBAT_VERSION={numbat_version}");
}
