fn main() {
    compile_gh_protocol();
    tauri_build::build()
}

fn compile_gh_protocol() {
    let gh_protocol_dir = std::path::Path::new("../libs/gh_protocol");
    
    if !gh_protocol_dir.exists() {
        println!("cargo:warning=gh_protocol directory not found, skipping C library compilation");
        return;
    }

    let msvc_install_dir = std::path::Path::new("E:/Microsoft/VisualStudio/2022/Community/VC/Auxiliary/Build/vcvars64.bat");
    
    if std::env::var("VisualStudioVersion").is_err() {
        println!("cargo:warning=Visual Studio environment not detected, skipping C library compilation");
        println!("cargo:warning=Please run cargo build from Visual Studio Developer Command Prompt");
        return;
    }

    let mut build = cc::Build::new();
    
    build
        .file(gh_protocol_dir.join("src/staticmapimp.c"))
        .file(gh_protocol_dir.join("src/slabmemory.c"))
        .file(gh_protocol_dir.join("src/gh_rpccore.c"))
        .file(gh_protocol_dir.join("src/gh_package.c"))
        .file(gh_protocol_dir.join("src/gh_protocol_api.c"))
        .file(gh_protocol_dir.join("impl/gh3036/gh_data_package.c"))
        .include(gh_protocol_dir.join("inc"))
        .include(gh_protocol_dir.join("impl/gh3036"))
        .define("GH_GYRO_EN", "0")
        .define("GH_GSENSOR_DEBUG_EN", "0")
        .define("GH_MODULE_PROTOCOL_LOG_EN", "0")
        .define("FLEXIBLE_ARRAY", "")
        .warnings(false);
    
    build.compile("gh_protocol");
    
    println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("src").display());
    println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("inc").display());
    println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("impl/gh3036").display());
}
