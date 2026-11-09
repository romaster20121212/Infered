 
 
 
 
 
 
 
 

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let task = std::env::args().nth(1);
    let result = match task.as_deref() {
        Some("e2e") => e2e(),
        Some("devtest") => devtest(),
        Some(other) => Err(format!("unknown task `{other}` (try: e2e, devtest)")),
        None => Err("usage: cargo xtask <task>  (tasks: e2e, devtest)".to_string()),
    };
    if let Err(message) = result {
        eprintln!("xtask: {message}");
        std::process::exit(1);
    }
}

 
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always has a parent directory")
        .to_path_buf()
}

 
 
 
 
 
 
fn e2e() -> Result<(), String> {
    let root = workspace_root();

    println!("[1/4] building the workspace (release)…");
    build_release(&root)?;

    let release = root.join("target/release");
    let injector = release.join(injector_file_name());
    if !injector.is_file() {
        return Err(format!(
            "injector binary not found at {}",
            injector.display()
        ));
    }
     
     
    println!("[2/4] artifacts staged in {}", release.display());

    println!("[3/4] discovering running Minecraft instances…");
    let processes = list_minecraft(&injector)?;
    let Some((pid, info)) = processes.first() else {
        return Err("no running Minecraft found — start the game, then re-run".to_string());
    };
    println!("      found PID {pid} — {info}");

    println!("[4/4] injecting into PID {pid}…");
    let status = Command::new(&injector)
        .arg("--inject")
        .arg(pid.to_string())
        .current_dir(&release)
        .status()
        .map_err(|e| format!("could not run the injector: {e}"))?;
    if !status.success() {
        return Err("injection failed — see the output above and app.log".to_string());
    }

    println!();
    println!("injection succeeded.");
    println!("verify in-game: focus Minecraft and press Right Shift —");
    println!("the Infered menu should open.");
    Ok(())
}

 
 
 
 
 
fn devtest() -> Result<(), String> {
    let root = workspace_root();
    println!("[1/2] building the workspace (devtest profile)…");
    let status = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args([
            "build",
            "--profile",
            "devtest",
            "-p",
            "agent_loader",
            "-p",
            "injector",
            "-p",
            "client",
            "--features",
            "client/devtest,agent_loader/devtest,injector/devtest",
        ])
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if !status.success() {
        return Err("devtest build failed".to_string());
    }

    let out = root.join("target/devtest");
    println!("[2/2] devtest artifacts in {}", out.display());
    let agent = if cfg!(windows) {
        "agent_loader.dll"
    } else {
        "libagent_loader.so"
    };
    for name in [client_file_name(), agent, injector_file_name()] {
        let path = out.join(name);
        if path.is_file() {
            println!("      {}", path.display());
        }
    }
    println!();
    println!(
        "run the devtest injector from {}; the Dev category holds Dev Mode \
         (test popup and notifications) and the Module Tester.",
        out.display()
    );
    Ok(())
}

 
fn build_release(root: &Path) -> Result<(), String> {
    let status = Command::new(env!("CARGO"))
        .current_dir(root)
        .args([
            "build",
            "--release",
            "-p",
            "agent_loader",
            "-p",
            "client",
            "-p",
            "injector",
        ])
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("release build failed".to_string())
    }
}

 
fn list_minecraft(injector: &Path) -> Result<Vec<(u32, String)>, String> {
    let output = Command::new(injector)
        .arg("--list")
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("could not run the injector: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (pid, info) = line.split_once('\t')?;
            Some((pid.trim().parse().ok()?, info.trim().to_string()))
        })
        .collect())
}

 
fn injector_file_name() -> &'static str {
    if cfg!(windows) {
        "injector.exe"
    } else {
        "injector"
    }
}

 
fn client_file_name() -> &'static str {
    if cfg!(windows) {
        "client.dll"
    } else {
        "libclient.so"
    }
}
