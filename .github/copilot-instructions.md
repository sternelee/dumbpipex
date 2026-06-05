# Copilot Instructions

## Build and verification commands

| Task | Command |
| --- | --- |
| Frontend dev server | `pnpm dev` |
| Frontend production build | `pnpm build` |
| SvelteKit/TypeScript check | `pnpm check` |
| Tauri desktop dev | `pnpm tauri dev` |
| Tauri desktop build | `pnpm tauri build` |
| Rust compile check | `cargo check --manifest-path src-tauri/Cargo.toml` |
| Rust test suite | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Single Rust test | `cargo test --manifest-path src-tauri/Cargo.toml <test_name>` |

## High-level architecture

- This repo is a Tauri desktop app with a SvelteKit frontend. The frontend is shipped as a static SPA: `src/routes/+layout.ts` disables SSR, `svelte.config.js` uses `@sveltejs/adapter-static` with `fallback: "index.html"`, and Tauri loads `../build` from `src-tauri/tauri.conf.json`.
- The frontend/backend boundary is the Tauri command bridge. Svelte components call Rust through `invoke()` from `@tauri-apps/api/core`, and Rust exposes those entrypoints with `#[tauri::command]` functions registered in `tauri::generate_handler![...]`.
- `src-tauri/src/main.rs` is only the binary entrypoint; application setup lives in `src-tauri/src/lib.rs`. New commands, plugins, and app wiring should be added in `lib.rs`, not pushed into `main.rs`.
- Tauri capabilities are explicit. `src-tauri/capabilities/default.json` defines the permissions for the main window, so plugin additions or features that need extra access should update capability config as well as Rust setup.
- Vite is configured specifically for Tauri development: port `1420` is fixed, HMR uses port `1421` when `TAURI_DEV_HOST` is set, and file watching ignores `src-tauri/**`.

## Key conventions

- Use Svelte 5 rune-style local state in components (`$state(...)`), matching `src/routes/+page.svelte`.
- Preserve the static-SPA assumption when changing SvelteKit code. Server-only features or SSR-dependent patterns will not work without reworking the current Tauri packaging setup.
- Keep Node package-manager commands aligned across files. Both `package.json` and `src-tauri/tauri.conf.json` are now aligned to pnpm. If you change one side, update the other.
- Static assets are served from `static/` and referenced with root-relative URLs in Svelte components.
