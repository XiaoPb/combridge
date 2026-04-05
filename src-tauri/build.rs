fn main() {
    link_gh_protocol();
    tauri_build::build()
}

fn link_gh_protocol() {
    let gh_protocol_dir = std::path::Path::new("../libs/gh_protocol");
    
    // Windows: 库文件在 build/Release/ 目录
    // Linux/macOS: 库文件在 build/ 目录
    let (lib_path, lib_exists) = if cfg!(target_os = "windows") {
        let path = gh_protocol_dir.join("build/Release");
        let exists = path.join("gh_protocol.lib").exists();
        (path, exists)
    } else {
        let path = gh_protocol_dir.join("build");
        let exists = path.join("libgh_protocol.a").exists();
        (path, exists)
    };

    if !lib_exists {
        println!("cargo:warning=gh_protocol library not found at {:?}", lib_path);
        println!("cargo:warning=Please build gh_protocol first");
        println!("cargo:warning=Run: cd libs/gh_protocol && mkdir -p build && cd build && cmake .. && cmake --build . --config Release");
        return;
    }

    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=gh_protocol");
    
    // Linux 需要显式链接数学库
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=m");
    }
    
    // 设置环境变量表示 C 库已链接
    println!("cargo:rustc-env=GH_PROTOCOL_LINKED=1");
    
    // 监听文件变化
    println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("inc").display());
    if cfg!(target_os = "windows") {
        println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("build/Release").display());
    } else {
        println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("build").display());
    }
}
