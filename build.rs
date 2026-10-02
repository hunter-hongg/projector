fn main() {
    // libgit2-sys 0.17 (bundled libgit2 1.8) calls advapi32 functions
    // (OpenProcessToken, CheckTokenMembership, GetNamedSecurityInfoW, …) in
    // fs_path.c but does not emit a link directive for advapi32 on MSVC, so the
    // final binary link fails with LNK2019 "__imp_OpenProcessToken" and friends.
    // Link advapi32 from here so Windows builds (cargo install / CI release) resolve.
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
