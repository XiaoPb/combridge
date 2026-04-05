fn main() {
    link_gh_protocol();
    tauri_build::build()
}

fn link_gh_protocol() {
    let gh_protocol_dir = std::path::Path::new("../libs/gh_protocol");
    let lib_path = gh_protocol_dir.join("build/Release");
    
    if !lib_path.exists() {
        println!("cargo:warning=gh_protocol.lib not found at {:?}, skipping C library linking", lib_path);
        println!("cargo:warning=Please build gh_protocol first");
        return;
    }

    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=gh_protocol");
    
    println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("inc").display());
    println!("cargo:rerun-if-changed={}", gh_protocol_dir.join("build/Release").display());
    
    println!("cargo:rustc-env=GH_PROTOCOL_LINKED=1");
}
