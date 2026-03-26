# Chapter II: The First Saferoom

*Or: How to give someone a clean room without touching their laptop.*

---

The server is live. `--setup` finished without a single `[FAILED]`. Erick closed the physical terminal hours ago and is now back at his desk, SSH'd into the machine from his laptop. He's root. He has the full MELISA prompt.

He types:

```
melisa@saferoom-01:~> melisa --list
```

```
[INFO] Retrieving container inventory...
NAME   STATE   AUTOSTART   GROUPS   IPV4   IPV6   UNPRIVILEGED
```

Empty. Clean. Ready.

Somewhere across the city, Alice is setting up her new job. She's a backend developer — Python and Rust — and she has been told by Erick that she does not need to install anything on her laptop. She was skeptical. Most people are.

Erick is about to prove it.

---

## Searching for a Room

He runs the search command to see what's available:

```
melisa@saferoom-01:~> melisa --search ubuntu
```

The spinner appears — the classic braille animation cycling at 12.5 FPS while MELISA fetches the LXC distribution list. A moment later:

```
[INFO] Synchronizing distribution list...

  CODE                          DISTRIBUTION     RELEASE   ARCH
  ubu-focal-x64                 ubuntu           focal     amd64
  ubu-jammy-x64                 ubuntu           jammy     amd64
  ubu-noble-x64                 ubuntu           noble     amd64
```

Jammy. Ubuntu 22.04. He's used it a hundred times. Stable, well-documented, `apt` works exactly the way you expect it to. He makes his choice.

---

## The Provisioning

```
melisa@saferoom-01:~> melisa --create alice-dev ubu-jammy-x64
```

MELISA goes to work. Under the hood, `lxc-create` is called with the `download` template, pulling the Ubuntu Jammy image from the official LXC image servers. The spinner ticks. Progress lines stream past. Then the network phase — MELISA checks for the `lxcbr0` bridge, verifies it's up, waits for the container to get a DHCP lease.

Then, quietly, it writes a file that nobody asked for but everyone needs:

```
/var/lib/lxc/alice-dev/rootfs/etc/melisa-info
```

```
MELISA_INSTANCE_NAME=alice-dev
MELISA_INSTANCE_ID=f47ac10b-58cc-4372-a567-0e02b2c3d479
DISTRO_SLUG=ubu-jammy-x64
DISTRO_NAME=ubuntu
DISTRO_RELEASE=jammy
ARCHITECTURE=amd64
CREATED_AT=2026-03-20T21:00:14+07:00
```

A birth certificate. Erick likes to think of it that way.

The container is provisioned. DNS is locked with `chattr +i` on `/etc/resolv.conf` — a trick he added after an afternoon spent debugging why a container suddenly couldn't reach the internet after a restart. The answer, predictably, was `systemd-resolved` overwriting the DNS config. The trick fixed it permanently.

```
[SUCCESS] Container 'alice-dev' provisioned successfully.
```

---

## The First Boot

```
melisa@saferoom-01:~> melisa --run alice-dev
```

The container starts. Erick checks:

```
melisa@saferoom-01:~> melisa --list
```

```
NAME        STATE     AUTOSTART   GROUPS   IPV4          IPV6   UNPRIVILEGED
alice-dev   RUNNING   0           -        10.0.3.101    -      false
```

There it is. IP address assigned. Running.

He verifies the metadata:

```
melisa@saferoom-01:~> melisa --info alice-dev
```

```
[INFO] Fetching metadata for container: alice-dev

  MELISA_INSTANCE_NAME  :: alice-dev
  MELISA_INSTANCE_ID    :: f47ac10b-58cc-4372-a567-0e02b2c3d479
  DISTRO_SLUG           :: ubu-jammy-x64
  DISTRO_NAME           :: ubuntu
  DISTRO_RELEASE        :: jammy
  ARCHITECTURE          :: amd64
  CREATED_AT            :: 2026-03-20T21:00:14+07:00
```

Everything matches. He enters the container to set it up:

```
melisa@saferoom-01:~> melisa --use alice-dev
```

```
root@alice-dev:/#
```

He installs the tools Alice will need — Python 3, pip, the project's dependencies. Everything goes into this container, not into the host. The host stays pristine.

---

## Creating Alice

Erick exits the container, returns to the MELISA prompt, and creates Alice's account.

```
melisa@saferoom-01:~> melisa --add alice
```

```
--- Provisioning New MELISA User: alice ---
Select Access Level for alice:
  1) Administrator (Full Management: Users, Projects, & LXC)
  2) Standard User (Project & LXC Management Only)
Enter choice (1/2): 2

[ACTION] Please set the authentication password for alice:
New password:
Retype new password:
passwd: password updated successfully
[SUCCESS] User account 'alice' successfully created.
[SUCCESS] Privilege configuration deployed successfully.
```

MELISA just did several things in sequence. Created a Linux system user. Set the login shell to `/usr/local/bin/melisa` — not bash. Set the home directory to mode `700`. Generated a custom sudoers file at `/etc/sudoers.d/melisa_alice` with exactly the binaries Alice is permitted to run as root: `lxc-*`, `git`, `melisa`, `mkdir -p`, `rm -f`, `bash -c`, `tee`, `chattr`. Nothing more.

He sends Alice a message with three things: the server IP, her username, and her temporary password.

---

## Alice's First Login

Alice opens her terminal. She has `ssh` and nothing else. No Docker. No VirtualBox. No Rust compiler. She types:

```bash
ssh alice@192.168.1.100
```

She expects a bash prompt. She gets this instead:

```
  ██╗ ... ███╗   ███╗███████╗██║     ██║███████╗███████╗
  ...
      [ MANAGEMENT ENVIRONMENT LINUX SANDBOX ]

  ┌─── SYSTEM TELEMETRY & STATUS ──────────────────────────────────────┐
  │ TIMESTAMP  :: 2026-03-20 21:15:47
  │ KERNEL_ID  :: FEDORA LINUX
  │ HOST_NODE  :: SAFEROOM-01
  ...
  └────────────────────────────────────────────────────────────────────┘

  >>> ALL SYSTEMS OPERATIONAL. SECURE SESSION GRANTED.
  ENTER COMMAND:
melisa@alice:~>
```

She stares at it for a moment. Then she types the first thing that comes naturally:

```
melisa@alice:~> melisa --list
```

```
[INFO] Retrieving container inventory...
NAME        STATE     AUTOSTART   GROUPS   IPV4          IPV6   UNPRIVILEGED
alice-dev   RUNNING   0           -        10.0.3.101    -      false
```

There's her room. Already running. Already set up.

```
melisa@alice:~> melisa --use alice-dev
```

```
root@alice-dev:/#
```

She's inside. She runs `python3 --version`. It works. She runs `pip list`. Everything is there. She types `ls /etc/melisa-info` and reads the birth certificate. She notes her instance ID.

She exits, returns to the MELISA prompt, and sends Erick a message:

*"Ok. I believe you now."*

---

## What Just Happened (The Dry Version)

From Erick's side: five commands. `--create`, `--run`, `--info`, `--use` (for setup), `--add`. Total time: under ten minutes, most of it waiting for the Ubuntu image download.

From Alice's side: one SSH command and two MELISA commands. She never touched the host. She never interacted with LXC directly. She got a fully isolated Ubuntu environment — her own process namespace, her own filesystem, her own network — running inside a Fedora host she doesn't have root access to.

The container is a clean room. When Alice eventually breaks it (developers always do), Erick will run `--delete alice-dev` and `--create alice-dev ubu-jammy-x64` and it will be as if nothing happened.

**Next:** [Chapter III — The Team Assembles](./chapter-3-the-team.md)