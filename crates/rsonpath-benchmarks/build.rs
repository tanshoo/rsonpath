use eyre::{eyre, Context, Result};
use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(feature = "blaze") {
        setup_blaze()?;
    }

    if cfg!(feature = "jsurfer") {
        setup_jsurfer()?;
    }

    Ok(())
}

fn setup_blaze() -> Result<()> {
    // Run cmake to configure the build
    let cmake_status = Command::new("cmake")
        .arg("-B")
        .arg("build")
        .current_dir("./src/implementations/blazeShim")
        .status()?;
    if !cmake_status.success() {
        return Err(eyre!("cmake configuration failed with status code: {}", cmake_status));
    }

    let make_status = Command::new("cmake")
        .arg("--build")
        .arg("build")
        .arg("--config")
        .arg("Release")
        .current_dir("./src/implementations/blazeShim")
        .status()?;
    if !make_status.success() {
        return Err(eyre!("cmake build failed with status code: {}", make_status));
    }

    let blaze_shim_build = std::path::Path::new("./src/implementations/blazeShim/build").canonicalize()?;

    println!("cargo:rerun-if-changed=src/implementations/blazeShim");
    println!("cargo:rustc-link-search=native={}", blaze_shim_build.display());
    println!("cargo:rustc-link-lib=dylib=blazeShim");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", blaze_shim_build.display());
    println!("cargo:rustc-link-lib=stdc++");
    Ok(())
}

fn setup_jsurfer() -> Result<()> {
    let gradlew_status = Command::new("./gradlew")
        .arg("shadowJar")
        .current_dir("./src/implementations/jsurferShim")
        .status()?;

    if !gradlew_status.success() {
        return Err(eyre!("gradlew execution failed with status code: {}", gradlew_status));
    }

    let java_home = std::env::var("JAVA_HOME").wrap_err("JAVA_HOME env variable not set")?;
    let jar_absolute_path =
        std::path::Path::new("./src/implementations/jsurferShim/lib/jsurferShim.jar").canonicalize()?;

    println!("cargo:rerun-if-changed=src/implementations/jsurferShim");
    println!("cargo:rustc-env=LD_LIBRARY_PATH={java_home}/lib/server");
    println!(
        "cargo:rustc-env=RSONPATH_BENCH_JSURFER_SHIM_JAR_PATH={}",
        jar_absolute_path.display()
    );

    Ok(())
}
