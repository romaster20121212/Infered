# Infered V1

**Infered V1** is a Minecraft **1.21.4 Fabric** client built around a native Rust core. It is not a standalone game or a resource pack: the injector attaches the native client to an already-running Minecraft Java process.

> **REQUIRES: Minecraft: Java Edition 1.21.4 with Fabric Loader and Fabric API installed.**
>
> ## Required game setup
>
> - Minecraft: Java Edition **1.21.4**
> - **Fabric Loader** for 1.21.4
> - **Fabric API** installed in the Fabric profile's `mods` folder
>
> It will **not** work on vanilla Minecraft, on other Minecraft versions, or on a Fabric profile that does not have Fabric API installed.

> **NOTE: The Fabric mod version does not work.** The `src/main` mod source is included for reference/development only. Use the injector and native client described below.

## What it is

The project builds three runtime pieces that work together:

| File | Purpose |
| --- | --- |
| `injector.exe` | Desktop injector. Finds the running Minecraft JVM, requests Administrator/root privileges, injects the agent, and tells it to load the client. |
| `agent_loader.dll` | Native agent loaded into Minecraft. Starts a small loopback control server and loads/reloads the native client. |
| `client.dll` | The native client itself. Installs JVMTI/JNI hooks, renders the in-game UI, and runs the modules. |

On Linux the same outputs are `injector`, `libagent_loader.so`, and `libclient.so`; on macOS they use `.dylib`.

The repository also contains the source for a separate Fabric mod in `src/main`, plus the shared crates used by the native build. **The Fabric mod version does not work; only the injector/native client path is functional.**

## Requirements

### To run

- Windows 10/11 x64 is the primary supported platform. Linux support is included in the source; macOS support is experimental.
- Minecraft Java Edition 1.21.4 with Fabric Loader and Fabric API (see above). The bundled Fabric mod metadata is tested against Fabric Loader 0.16.10 and Fabric API 0.119.4+1.21.4.
- A 64-bit Java 21 runtime, which is what Minecraft 1.21.4 and Fabric require anyway.
- Administrator privileges on Windows, or root on Linux, to inject into another process.

### To build

- A recent Rust toolchain and Cargo (Rust 1.85 or newer). The project has been built with nightly; stable should work as long as it supports edition 2024.
- **JDK 21**, not just a JRE. The build compiles the Java bridge classes and the JVMTI C shim, so `javac` and the JNI headers are required. Set `JAVA_HOME` to the JDK root.
- A C/C++ linker:
  - Windows: Visual Studio Build Tools with the "Desktop development with C++" workload (MSVC).
  - Linux: GCC or Clang plus the usual build tools.
  - macOS: Xcode Command Line Tools.
- An internet connection for the first build so Cargo can download crates.

## Build

From the repository root, run:

```sh
cargo build --release
```

The release profile uses LTO and a single codegen unit, so the first build can take several minutes.

On Windows, set `JAVA_HOME` before building:

```powershell
$env:JAVA_HOME = "C:\Program Files\Eclipse Adoptium\jdk-21.0.12.101-hotspot"
cargo build --release
```

On Linux:

```sh
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
cargo build --release
```

If the build cannot find `jvm.lib` on Windows, point `JVM_LIB_DIR` at the folder that contains it:

```powershell
$env:JVM_LIB_DIR = "$env:JAVA_HOME\lib"
cargo build --release
```

The build products are in `target/release/`:

- `target/release/injector.exe`
- `target/release/client.dll`
- `target/release/agent_loader.dll`

Keep all three files together in the same folder. The injector looks for the two DLLs next to its own executable.

## Usage

1. Install Fabric Loader for Minecraft 1.21.4 and put the Fabric API jar in the profile's `mods` folder.
2. Launch the Fabric 1.21.4 profile and wait until the game reaches the main menu or a world.
3. Copy `injector.exe`, `client.dll`, and `agent_loader.dll` into one folder.
4. Run `injector.exe` as Administrator. On Linux, run the injector with `sudo`.
5. Select the Minecraft process in the injector window and click **Inject**.
6. Focus Minecraft and press **Right Shift** to open the Infered menu.

The injector also has a command line:

```text
injector.exe --list              # list detected Minecraft processes
injector.exe --inject <pid>      # inject into a specific process
injector.exe --tui               # run the terminal UI
```

Logs are written to `app.log`, and runtime configuration is written to `infered_config.json`. Both are created in the injector's working directory, so run the injector from the folder where you want those files to live.

## How it works

1. The injector finds a running Java process whose command line identifies Minecraft.
2. It injects `agent_loader` into that JVM.
3. The agent starts a control server on `127.0.0.1:7878`.
4. The injector sends the absolute path of `client.dll` to the agent.
5. The agent loads the client into Minecraft, and the client installs its JVMTI/JNI hooks and starts the in-game UI.

The control server accepts unauthenticated commands and binds to loopback only. Keep it on the local machine and close Minecraft when you are done. Release binaries are unsigned; only run builds you made yourself or trust.

## Troubleshooting

- **`jvm.lib` not found**: set `JAVA_HOME` to a JDK 21 installation, or set `JVM_LIB_DIR` to the directory containing `jvm.lib`.
- **`javac` not found / JNI headers missing**: install a full JDK, not a JRE, and point `JAVA_HOME` at it.
- **`client.dll beside injector executable`**: keep `injector.exe`, `client.dll`, and `agent_loader.dll` in the same folder.
- **Injection fails or no Minecraft process is listed**: start Minecraft first, make sure the game is the 64-bit Java 21 build, and run the injector as Administrator/root.
- **Menu does not open**: confirm the game is on Minecraft 1.21.4 with Fabric Loader and Fabric API, then check `app.log` for loader or mapping errors.

## Project layout

```text
injector/          injector GUI/TUI, process discovery, DLL injection
agent_loader/      native agent, local control server, client loader
client/            native client, JVMTI/JNI hooks, renderer, modules
protocol/          injector <-> agent command protocol
macros/            module/macro support used by the client
mapping_derive/    mapping macros
xtask/             developer tasks (cargo xtask ...)
vendor/ilhook/     patched third-party x86/x64 hook library
src/main/          Fabric mod source and resources (separate optional mod build)
assets/            embedded fonts
logo's/            embedded client and injector artwork
java_mappings.json, mappings*.json  bundled Minecraft mappings
```

The Fabric mod source is separate from the native build and **does not work in-game**. The Gradle build can still be run for development/reference with JDK 21:

```powershell
.\gradlew build
```

The jar is written to `build/libs/`, but it will not function as a client. Use the native `cargo build --release` output instead.

## Copyright and permission

Copyright © 2026 TheFinalBlow.

> **COPYRIGHT NOTICE:** The Infered source code may not be copied, reproduced, or redistributed without permission. Permission to use, copy, modify, and redistribute this source code is granted only under the terms of the GNU General Public License v3.0.

## License

Infered is released under the GNU General Public License v3.0 (GPL-3.0-only). See [LICENSE](LICENSE). Third-party components keep their own licenses:

- `vendor/ilhook/` is MIT licensed; see `vendor/ilhook/LICENSE`.
- Embedded fonts are SIL Open Font License 1.1; see the `OFL.txt` files in `assets/fonts/` and `assets/ui-fonts/`.

Minecraft-derived textures are included so the client can render vanilla items without reading the game's resource files at runtime. Confirm that you have permission to redistribute every bundled asset before publishing a fork or release.
