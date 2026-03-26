# Rust Server Internals

## Entry Point: `main.rs`

The entry point handles three distinct responsibilities in sequence:

```rust
#[tokio::main]
async fn main() {
    // Phase 1: Root check & privilege escalation
    if !check_root() {
        Command::new("sudo").arg("-H")
            .arg("/usr/local/bin/melisa")
            .args(&args)
            .status().await;
        exit(status_code);
    }

    // Phase 2: Non-interactive mode (direct argument dispatch)
    if args.len() >= 2 {
        let cmd_string = if args[1] == "-c" {
            args[2..].join(" ")   // SSH: melisa -c "command args"
        } else {
            args[1..].join(" ")   // Direct: melisa --list
        };
        execute_command(&cmd_string, &user, &home).await;
        exit(0);
    }

    // Phase 3: Interactive REPL mode
    display_melisa_banner();
    melisa().await;
}
```

**The `-c` flag pattern** is how SSH invokes remote commands: `ssh host "melisa --list"` becomes `melisa -c "--list"` from the shell's perspective. The parser handles both invocation styles transparently.

---

## The REPL: `cli/melisa_cli.rs`

The interactive shell is built on **rustyline**, providing readline-like editing with history, completion, and validation.

```rust
pub async fn melisa() {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    let helper = MelisaHelper { ... };
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(helper));
    rl.load_history(HISTORY_PATH).ok();

    loop {
        let prompt = Prompt::new().build();
        match rl.readline(&prompt) {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                match execute_command(&line, &user, &home).await {
                    ExecResult::Break => break,
                    ExecResult::ResetHistory => reset_history(&mut rl, HISTORY_PATH).await,
                    ExecResult::Error(e) => eprintln!("{}", e),
                    ExecResult::Continue => {},
                }
            }
            Err(ReadlineError::Interrupted) => continue,  // Ctrl+C
            Err(ReadlineError::Eof) => break,              // Ctrl+D
            Err(e) => { eprintln!("Error: {:?}", e); break; }
        }
    }
    rl.save_history(HISTORY_PATH).ok();
}
```

The `ExecResult` enum allows the executor to signal state changes back to the REPL:

```rust
pub enum ExecResult {
    Continue,           // Normal: loop continues
    Break,              // exit/quit: terminate the REPL
    ResetHistory,       // --clear: purge history, keep looping
    Error(String),      // Display error, keep looping
}
```

---

## The Helper: `cli/helper.rs`

`MelisaHelper` implements the rustyline `Helper` trait bundle via derive macros:

```rust
#[derive(Helper, Validator, Hinter)]
pub struct MelisaHelper {
    #[rustyline(Hinter)]
    pub hinter: HistoryHinter,        // Grey ghost text from history
    pub highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    pub validator: MatchingBracketValidator,
    pub file_completer: FilenameCompleter,
}
```

**Completion logic** uses contextual routing:

```rust
fn complete(&self, line: &str, pos: usize, ctx: &Context) -> Result<(usize, Vec<Pair>)> {
    // Route 1: File paths (cd commands or path with /)
    if line.starts_with("cd ") || line[..pos].contains('/') {
        return self.file_completer.complete(line, pos, ctx);
    }

    // Route 2: History-based completion (reverse traversal, deduped)
    let prefix = &line[..pos];
    let mut seen = HashSet::new();
    // ... reverse traversal through history ...
}
```

**Hint rendering** — the ghost text is displayed in ANSI bright black (grey):

```rust
fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
    Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
}
```

---

## The Loading Spinner: `cli/loading.rs`

Long operations (container creation, distribution list fetch) use a non-blocking spinner:

```rust
pub async fn execute_with_spinner<F, Fut, T>(
    message: &str,
    task: F,
) -> T
where
    F: FnOnce(ProgressBar) -> Fut,
    Fut: Future<Output = T>,
{
    let pb = ProgressBar::new_spinner();
    pb.set_style(/* spinner style */);
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));

    let result = task(pb.clone()).await;

    pb.finish_and_clear();
    result
}
```

The `ProgressBar` from `indicatif` is passed into the task function so it can call `pb.println()` to output status lines above the spinner without corrupting the display.

---

## Container Operations: `core/container.rs`

All LXC operations go through `tokio::process::Command`:

```rust
// Example: create_new_container
let process = Command::new("sudo")
    .args(&[
        "-n", "lxc-create", "-P", LXC_PATH, "-t", "download", "-n", name,
        "--", "-d", &meta.name, "-r", &meta.release, "-a", &meta.arch
    ])
    .output()
    .await;   // ← Non-blocking: yields to Tokio executor while waiting
```

**Network readiness polling** — dynamic wait instead of a blind sleep:

```rust
async fn wait_for_network_initialization(name: &str) -> bool {
    for attempt in 1..=20 {
        let output = Command::new("sudo")
            .args(&["lxc-info", "-P", LXC_PATH, "-n", name, "-iH"])
            .output().await;

        if let Ok(out) = output {
            let ip = String::from_utf8_lossy(&out.stdout);
            if !ip.trim().is_empty() && ip.trim() != "127.0.0.1" {
                return true;  // Container has a DHCP IP
            }
        }
        sleep(Duration::from_secs(2)).await;
    }
    false  // Timeout after 40 seconds
}
```

---

## Metadata System: `core/metadata.rs`

Error handling uses `thiserror` for typed errors:

```rust
#[derive(thiserror::Error, Debug)]
pub enum MelisaError {
    #[error("IO failure: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path traversal detected in name: {0}")]
    SecurityViolation(String),

    #[error("Metadata not found for container '{0}'. Is it a MELISA container?")]
    MetadataNotFound(String),
}
```

**Atomic write sequence** with tracing instrumentation:

```rust
#[instrument(skip(meta), fields(container_name = %name))]
pub async fn inject_distro_metadata(...) -> Result<(), MelisaError> {
    // 1. Security check
    if name.contains('/') || name.contains('\\') || name == ".." {
        return Err(MelisaError::SecurityViolation(name.to_string()));
    }

    // 2. Write to temp file
    let mut file = fs::OpenOptions::new()
        .write(true).create(true).truncate(true)
        .open(&temp_path).await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await?;
    file.sync_all().await?;  // ← fsync before rename

    // 3. Set permissions
    let perms = Permissions::from_mode(0o644);
    fs::set_permissions(&temp_path, perms).await?;

    // 4. Atomic rename (kernel guarantees atomicity)
    fs::rename(&temp_path, &target_path).await?;

    info!("Metadata successfully injected for container: {}", name);
    Ok(())
}
```

---

## Host OS Detection: `distros/host_distro.rs`

The setup routine detects the host's firewall type to apply the correct configuration:

```rust
pub enum FirewallKind {
    Firewalld,
    Ufw,
    Iptables,
}

pub fn detect_host_distro() -> HostDistroConfig {
    // Checks presence of firewalld, ufw, iptables binaries
    // Returns a HostDistroConfig with package manager and firewall type
}
```

This abstraction allows `setup.rs` to call `configure_firewall(kind)` without knowing which specific tool is present — the firewall configuration is dispatched based on the detected kind.

---

## Dependencies (`Cargo.toml`)

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.50 | Async runtime (full features) |
| `rustyline` | 17.0.2 | REPL with history, completion, highlighting |
| `colored` | 2.1 | ANSI terminal colors |
| `indicatif` | 0.18 | Progress bars and spinners |
| `sysinfo` | 0.38 | System info for the boot dashboard (CPU, RAM, hostname) |
| `chrono` | 0.4 | Timestamps for metadata and history messages |
| `uuid` | 1.0 (v4) | Container instance ID generation |
| `tracing` | 0.1 | Structured logging with `instrument` macro |
| `thiserror` | 2.0 | Typed error derivation |
| `serde` + `toml` | — | Configuration parsing |
| `libc` | 0.2 | Low-level Unix system calls |
| `ctrlc` | 3.4 | Graceful Ctrl+C handling |
| `rand` | 0.8 | Random glitch characters in boot animation |