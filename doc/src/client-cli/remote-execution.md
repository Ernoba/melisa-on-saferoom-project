# Remote Execution

These commands let you run code and transfer files into containers on the remote MELISA host without manually SSHing in and navigating the server shell.

---

## `melisa run <container> <file>`

Executes a local script file inside a remote container. The script is **streamed** over SSH — it is never stored permanently on the host or the container.

```bash
melisa run mybox ./setup.sh
melisa run devlab ./scripts/install_deps.py
melisa run nodeenv ./app/index.js
```

### How It Works

```bash
cat "$file" | ssh root@192.168.1.100 "melisa --send $container $interpreter -"
```

The file content is piped from your local machine's `cat` directly into SSH, which forwards it to the server's `melisa --send` command, which pipes it into the container's interpreter via `lxc-attach`.

**Zero footprint:** The script never touches the host filesystem. It flows: `local disk → SSH pipe → container stdin → interpreter`.

### Dynamic Interpreter Resolution

The interpreter is chosen based on the file extension:

| Extension | Interpreter |
|-----------|-------------|
| `.sh` | `bash` |
| `.py` | `python3` |
| `.js` | `node` |
| (any other) | `bash` |

---

## `melisa run-tty <container> <file>`

Like `run`, but allocates a **pseudo-TTY** for interactive scripts — scripts that need to display progress bars, prompt for user input, use `curses`, or otherwise require a real terminal.

```bash
melisa run-tty mybox ./interactive_installer.sh
melisa run-tty devlab ./scripts/setup_wizard.py
```

### How It Works (3-Phase)

**Phase 1 — Upload to /tmp**

```bash
tar -czf - -C "$dir" "$filename" | \
  ssh root@192.168.1.100 "melisa --upload $container /tmp"
```

The script is compressed and uploaded to the container's `/tmp/` directory.

**Phase 2 — Interactive Execution**

```bash
ssh -t root@192.168.1.100 "melisa --send $container $interpreter /tmp/$filename"
```

The `-t` flag forces TTY allocation. Your terminal is fully connected to the container process — stdin, stdout, stderr, and terminal size all pass through.

**Phase 3 — Cleanup**

```bash
ssh root@192.168.1.100 "melisa --send $container rm -f /tmp/$filename"
```

Automatically removes the script file from the container after execution. The container stays clean.

### When to Use `run-tty` vs `run`

| Scenario | Use |
|----------|-----|
| Script produces output, no input needed | `run` |
| Script uses `input()`, `read`, or interactive menus | `run-tty` |
| Script uses `tput`, progress bars, colors | `run-tty` |
| CI/CD automated execution | `run` |
| Developer walkthrough / wizard | `run-tty` |

---

## `melisa upload <container> <local_dir> <dest_path>`

Compresses a local directory and extracts it inside a container at the specified path.

```bash
melisa upload mybox ./src/build/ /opt/myapp/
melisa upload devlab ./config/ /etc/myconfig/
melisa upload nodeenv ./node_modules/ /app/node_modules/
```

### How It Works

```bash
tar -czf - -C "$local_dir" . | \
  ssh root@192.168.1.100 "melisa --upload $container $dest_path"
```

On the server side:

```bash
lxc-attach -n mybox -- bash -c \
  "mkdir -p /opt/myapp/ && tar -xzf - -C /opt/myapp/"
```

**Notes:**
- The destination directory is created automatically if it doesn't exist
- All files from `local_dir` are extracted directly into `dest_path` (contents, not the directory itself — equivalent to `cp -r local_dir/* dest_path/`)
- Large directories are handled efficiently via streaming — nothing is buffered entirely in memory

---

## `melisa shell`

Opens a direct interactive SSH shell to the MELISA host (not a container — the host itself):

```bash
melisa shell
```

```
[INFO] Establishing secure shell connection to root@192.168.1.100...
melisa@hostname:~>
```

This lands you in the MELISA interactive prompt on the server. Equivalent to running `ssh root@192.168.1.100` manually, but uses the configured multiplexed connection.

---

## `melisa --list` / `melisa --active` (via forwarding)

Any command not recognized locally is forwarded to the active MELISA server via `exec_forward`:

```bash
# These are forwarded transparently to the server:
melisa --list
melisa --active
melisa --info mybox
melisa --run mybox
melisa --stop mybox
melisa --user
melisa --projects
```

The forwarding function:

```bash
exec_forward() {
    ensure_connected
    ssh -t "$CONN" "melisa $*"
}
```

The `-t` flag allocates a TTY so interactive server commands work correctly (deletion confirmations, password prompts, etc.).

---

## Complete Remote Operations Diagram

```
Your Machine                    MELISA Host              Container
─────────────                   ───────────              ─────────

melisa run mybox setup.sh
  │
  ├─ cat setup.sh ──────────────► melisa --send mybox bash -
  │                                        │
  │                                        └─ lxc-attach mybox bash ─► interpreter
  │                                                                         │
  └──────────────────────────────────────────────────────── output ◄────────┘

melisa run-tty mybox wizard.py
  │
  ├─ tar | ssh upload ──────────► melisa --upload mybox /tmp
  │                                        │
  │                                        └─ extract to /tmp/wizard.py
  │
  ├─ ssh -t ─────────────────────► melisa --send mybox python3 /tmp/wizard.py
  │    ▲                                   │
  │    │ (interactive TTY)                 └─ lxc-attach mybox python3 /tmp/wizard.py
  │    └─────────────────────────────────────────────────────────────────────────┘
  │
  └─ ssh cleanup ─────────────────► melisa --send mybox rm -f /tmp/wizard.py

melisa upload mybox ./src /opt/app
  │
  └─ tar | ssh ──────────────────► melisa --upload mybox /opt/app
                                           │
                                           └─ mkdir /opt/app && tar extract
```