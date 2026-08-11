# Tauri Server Client

[![CI](https://github.com/xcvzlabs/tauri-server-client/actions/workflows/ci.yaml/badge.svg?style=flat&colorA=18181B&colorB=34d399)](https://github.com/xcvzlabs/tauri-server-client/actions/workflows/ci.yaml)
[![Release](https://github.com/xcvzlabs/tauri-server-client/actions/workflows/release.yaml/badge.svg?style=flat&colorA=18181B&colorB=34d399)](https://github.com/xcvzlabs/tauri-server-client/actions/workflows/release.yaml)

A LAN-based client-server desktop application built with Tauri and Rust, with a Nuxt + Vue frontend.

## Stack

- [Tauri v2](https://v2.tauri.app) (Rust) for the desktop shell
- [Nuxt 4](https://nuxt.com) + Vue 3 for the frontend
- [Bun](https://bun.sh) as the runtime and package manager
- [Valibot](https://valibot.dev) for validation
- [Vitest](https://vitest.dev) for tests
- [oxlint](https://oxc.rs) + [oxfmt](https://oxc.rs) for linting and formatting

## Setup

Install dependencies:

```bash
bun install
```

## Development

Start the app in development mode (Nuxt dev server + Tauri window):

```bash
bun run dev
```

## Checks

```bash
bun run fmt:check   # oxfmt + cargo fmt --check
bun run lint        # oxlint + cargo clippy
bun run nuxt:typecheck
bun run test         # vitest
```

## Release

Version bumps, changelog generation, and Tauri version sync are handled by `scripts/release.ts`:

```bash
bun run release patch   # or minor / major
```

This bumps `package.json`, generates the changelog, syncs the version into `src-tauri/Cargo.toml`, `Cargo.lock`, and `tauri.conf.json`, then commits, tags, and pushes. The [release workflow](.github/workflows/release.yaml) picks up the pushed tag and builds/publishes the desktop app.
