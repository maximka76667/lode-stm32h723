// This file was automatically generated.

fn main() {
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    #[cfg(feature = "defmt")]
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    if let Ok(contents) = std::fs::read_to_string(".env") {
        println!("cargo:warning=.env found");
        for line in contents.lines() {
            if let Some(val) = line.strip_prefix("API_KEY=") {
                let masked = format!("{}{}", "*".repeat(val.len().saturating_sub(4)), &val[val.len().saturating_sub(4)..]);
            println!("cargo:warning=API_KEY found: {masked}");
                println!("cargo:rustc-env=API_KEY={val}");
            }
        }
    } else {
        println!("cargo:warning=.env not found");
    }
}
