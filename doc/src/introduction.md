# Introduction

<div align="center">

```
 ███╗   ███╗███████╗██║     ██║███████╗███████╗
 ████╗ ████║██╔════╝██║     ██║██╔════╝██╔══██╗
 ██╔████╔██║█████╗  ██║     ██║███████╗███████║
 ██║╚██╔╝██║██╔══╝  ██║     ██║╚════██║██╔══██║
 ██║ ╚═╝ ██║███████╗███████╗██║███████║██║  ██║
 ╚═╝     ╚═╝╚══════╝╚══════╝╚═╝╚══════╝╚═╝  ╚═╝
     [ MANAGEMENT ENVIRONMENT LINUX SANDBOX ]
```

**v0.1.2 — delta version** · Built with 🦀 Rust · MIT License

</div>

---

## What is MELISA?

**MELISA** (Management Environment Linux Sandbox) is a high-performance LXC container manager written in Rust, designed to solve one fundamental problem in software development: **host pollution**.

You know the feeling. You want to try a new language, test a risky library, or experiment with a system-level tool — but you're terrified of corrupting your pristine development machine. Or your team has the classic *"works on my machine"* problem. Or you're a teacher who needs to provision identical environments for 30 students in minutes.

MELISA solves all of this by turning a Linux host into a **Secure Orchestration Node** — a machine that carves out isolated, reproducible LXC containers on demand, manages users with fine-grained permissions, and synchronizes collaborative projects through Git-backed pipelines, all controllable from any workstation in the world via a lightweight Bash client.

---

## The Two Pillars

MELISA is built around two components that work in concert:

### 🦀 The Server (Host Engine)
A compiled Rust binary (`melisa`) that runs on a Linux host. It acts as a **jail shell** — when users log in via SSH, they land directly inside the MELISA interactive prompt rather than a standard bash session. The engine manages LXC containers, enforces privilege separation, and orchestrates Git-based project collaboration.

### 🐚 The Client (Remote Manager)
A modular Bash script (`melisa`) installed on any workstation. It wraps SSH to transparently forward commands to the remote MELISA host, allowing developers to manage containers, clone projects, sync code, and execute scripts inside remote containers — all with a single, unified CLI.

---

## Design Philosophy

| Principle | Implementation |
|-----------|----------------|
| **Zero Host Pollution** | All work happens inside LXC containers; the host OS remains clean |
| **Security by Presence** | System initialization requires physical terminal access — remote attackers cannot bootstrap the system |
| **Privilege Separation** | Two roles (Admin / Standard User) with surgically precise `sudoers` rules |
| **Async by Default** | The Rust engine is built on Tokio; no blocking I/O anywhere in the critical path |
| **Reproducibility** | Containers are provisioned from standardized LXC templates with deterministic post-install steps |
| **Git-Native Collaboration** | Projects are bare Git repositories; the push-to-deploy hook auto-syncs all members |

---

## What's Inside This Book?

This documentation is split into two parts:

**Part I — Technical Guide** is your reference manual. Every command, every flag, every configuration option is documented with examples, internal mechanics, and edge-case notes.

**Part II — Story Edition** tells the same story differently. If you're the kind of person who learns better by following a narrative — a sysadmin setting up a server, a team shipping their first project — start here.

---

## Quick Navigation

| I want to... | Go to... |
|---|---|
| Install the server | [Server Installation](./getting-started/server-installation.md) |
| Install the client | [Client Installation](./getting-started/client-installation.md) |
| Create my first container | [Your First Container](./getting-started/first-container.md) |
| See all server commands | [Server CLI Reference](./server-cli/README.md) |
| See all client commands | [Client CLI Reference](./client-cli/README.md) |
| Understand how it works | [Architecture & Internals](./architecture/README.md) |
| Fix a problem | [Troubleshooting](./troubleshooting.md) |

---

> **License:** MELISA is open-source software released under the [MIT License](https://opensource.org/licenses/MIT).
> **Author:** Erick Adriano Sebastian · `ernobaproject@gmail.com`