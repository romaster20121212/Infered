 
 
 
 

#[cfg(windows)]
fn main() {
    use std::path::PathBuf;
    use std::{env, fs};

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        && env::var("PROFILE").as_deref() == Ok("release")
        && env::var_os("CARGO_FEATURE_DEVTEST").is_none()
    {
        println!("cargo:rustc-link-arg-cdylib=/DEBUG:NONE");
    }
    println!("cargo:rerun-if-env-changed=JAVA_HOME");
    println!("cargo:rerun-if-env-changed=JVM_LIB_DIR");
     
    if let Ok(dir) = env::var("JVM_LIB_DIR") {
        let p = PathBuf::from(&dir);
        if p.exists() {
            link_jvm(&p);
            println!("cargo:warning=Using JVM_LIB_DIR={}", dir);
            return;
        }
    }

     
    if let Ok(java_home) = env::var("JAVA_HOME") {
        let p = PathBuf::from(java_home);
        if let Some(found) = search_jvm_in(&p) {
            link_jvm(&found);
            println!(
                "cargo:warning=Found jvm.lib via JAVA_HOME at {}",
                found.display()
            );
            return;
        }
    }

     
    let common = [
        r"C:\Program Files\Java",
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft",
        r"C:\Program Files\Zulu",
        r"C:\Program Files\Amazon Corretto",
        r"C:\Program Files (x86)\Java",
    ];

    for root in &common {
        let rootp = PathBuf::from(root);
        if let Ok(entries) = fs::read_dir(&rootp) {
            for e in entries.flatten() {
                if let Some(found) = search_jvm_in(&e.path()) {
                    link_jvm(&found);
                    println!("cargo:warning=Found jvm.lib in {}", found.display());
                    return;
                }
            }
        }
    }

     
    if let Some(found) = find_jvm_from_registry() {
        link_jvm(&found);
        println!(
            "cargo:warning=Found jvm.lib via Windows Registry at {}",
            found.display()
        );
        return;
    }

     
    if let Some(found) = find_jvm_from_path_java() {
        link_jvm(&found);
        println!(
            "cargo:warning=Found jvm.lib via PATH java.exe at {}",
            found.display()
        );
        return;
    }

     
    panic!(
        "build.rs: could not find jvm.lib. 
    - Set JAVA_HOME to your JDK root (e.g. C:\\Program Files\\Java\\jdk-21)
    - Or set JVM_LIB_DIR directly to the folder containing jvm.lib"
    );
}

#[cfg(not(windows))]
fn main() {
     
}

#[cfg(windows)]
 
fn link_jvm(dir: &std::path::Path) {
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=dylib=jvm");
}

#[cfg(windows)]
 
fn search_jvm_in(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let tries = [
        root.join("lib").join("jvm.lib"),
        root.join("lib").join("amd64").join("jvm.lib"),
        root.join("lib").join("x86_64").join("jvm.lib"),
        root.join("lib").join("server").join("jvm.lib"),
        root.join("lib").join("client").join("jvm.lib"),
    ];
    for candidate in &tries {
        if candidate.exists() {
            return candidate.parent().map(|p| p.to_path_buf());
        }
    }
    None
}

 
 
#[cfg(windows)]
fn find_jvm_from_registry() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    use std::process::Command;

    const KEYS: &[&str] = &[
        r"HKLM\SOFTWARE\JavaSoft\JDK",
        r"HKLM\SOFTWARE\Eclipse Adoptium\JDK",
        r"HKLM\SOFTWARE\Microsoft\JDK",
    ];

    for key in KEYS {
        let output = Command::new("reg")
            .args(["query", key, "/s"])
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("JavaHome") {
                let parts: Vec<_> = line.split_whitespace().collect();
                if let Some(path) = parts.last() {
                    let p = PathBuf::from(path);
                    if let Some(found) = search_jvm_in(&p) {
                        return Some(found);
                    }
                }
            }
        }
    }
    None
}

 
 
 
#[cfg(windows)]
fn find_jvm_from_path_java() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    use std::process::Command;

    let output = Command::new("where").arg("java").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let path = PathBuf::from(line.trim());
        if !path.is_file() {
            continue;
        }
         
        if let Some(jdk_root) = path.parent().and_then(|bin| bin.parent()) {
            if let Some(found) = search_jvm_in(jdk_root) {
                return Some(found);
            }
        }
    }
    None
}
