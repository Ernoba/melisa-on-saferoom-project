# Core Concepts

Understanding MELISA's mental model will make you significantly more effective with its tooling. This section explains the four fundamental building blocks of the system.

## The Four Pillars

```
┌─────────────────────────────────────────────────────────────┐
│                        MELISA HOST                          │
│                                                             │
│  ┌───────────┐   ┌───────────────┐   ┌──────────────────┐  │
│  │  USERS    │   │  CONTAINERS   │   │    PROJECTS      │  │
│  │           │   │               │   │                  │  │
│  │ • Admin   │   │ • ubuntu-box  │   │ • /opt/melisa/   │  │
│  │ • Standard│   │ • debian-lab  │   │   projects/      │  │
│  │           │   │ • arch-exp    │   │   myapp.git/     │  │
│  └───────────┘   └───────────────┘   └──────────────────┘  │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 SECURITY MODEL                      │   │
│  │  Sudoers rules · SUID binary · Jail shell · chmod   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

- **[Containers & Isolation](./containers.md)** — The fundamental unit of work in MELISA. Each container is a fully isolated Linux environment with its own filesystem, network, and processes.

- **[Users & Roles](./users-and-roles.md)** — MELISA manages its own user registry on top of Linux system users. Two roles (Admin, Standard) with surgically precise `sudoers` policies.

- **[Projects & Collaboration](./projects.md)** — Git-backed shared workspaces that synchronize automatically across all team members when anyone pushes code.

- **[Security Model](./security-model.md)** — How MELISA uses SUID bits, jail shells, privilege separation, and namespace isolation to keep the host system safe.