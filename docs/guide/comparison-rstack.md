# Vite+ vs Rstack

Both [Vite+](https://viteplus.dev/) and [Rstack](https://rstack.rs/) are **unified JavaScript toolchains** that combine multiple tools behind a consistent architecture. They optimize for different ecosystems and entry points. This page is a practical comparison to help you choose—or understand when each stack is a better fit.

::: tip Scope
This comparison focuses on product positioning and typical workflows. Tool versions and feature sets change quickly; always check the latest docs for each project.
:::

## At a glance

| | **Vite+** | **Rstack** |
| --- | --- | --- |
| **Steward** | [VoidZero](https://voidzero.dev) (Vite / Vitest / Rolldown / Oxc ecosystem) | [Web Infra](https://github.com/web-infra-dev) (ByteDance open source) |
| **Core idea** | One `vp` CLI + local `vite-plus` package for runtime, package manager, and frontend toolchain | Family of tools centered on the [Rspack](https://rspack.rs/) bundler |
| **Bundler lineage** | Vite + [Rolldown](https://rolldown.rs/) (Vite ecosystem, ESM-native dev) | [Rspack](https://rspack.rs/) (Rust bundler with webpack-compatible APIs) |
| **Primary app workflow** | `vp create` / `vp migrate` → `vp dev` / `vp check` / `vp test` / `vp build` | [Rsbuild](https://rsbuild.rs/) app projects on top of Rspack |
| **Library packaging** | `vp pack` (tsdown-oriented library / binary packaging) | [Rslib](https://rslib.rs/) |
| **Lint / format** | Oxlint + Oxfmt via `vp lint` / `vp fmt` / `vp check` | Typically ESLint/Prettier or community plugins (not a single built-in `vp check` equivalent) |
| **Testing** | Bundled [Vitest](https://vitest.dev/) via `vp test` | [Rstest](https://rstest.rs/) in the Rstack family |
| **Docs / static sites (ecosystem)** | [VitePress](https://vitepress.dev/) is common in the Vite ecosystem (powers this site); not a first-class `vp` command | [Rspress](https://rspress.rs/) (first-class Rstack product) |
| **Package manager / Node** | `vp install`, `vp env` manage PM workflows and Node runtimes | Use your own Node + npm/pnpm/yarn/bun (no unified PM CLI like `vp`) |
| **Monorepo tasks** | `vp run` with Vite Task caching | Rsbuild/Rspack workspaces + external task runners (Turbo, Nx, etc.) |
| **Ecosystem fit** | Deep Vite plugin ecosystem (React, Vue, Svelte, …) | Strong webpack/Rspack plugin ecosystem and Module Federation story |

## What is Vite+?

Vite+ is the unified entry point for **local web development** around the Vite stack. A single dependency and the global `vp` binary cover:

- Runtime and package-manager workflows (`vp env`, `vp install`, `vp add`, …)
- Dev server and production builds (`vp dev`, `vp build`) on Vite + Rolldown
- Format, lint, and type-check in one pass (`vp check`)
- Tests (`vp test` → Vitest)
- Library / binary packaging (`vp pack`)
- Scaffolding and migration (`vp create`, `vp migrate`)
- Monorepo task running with caching (`vp run`)

Configuration lives primarily in one `vite.config.ts` via `defineConfig` from `vite-plus`. See [Why Vite+](/guide/why) and the [Getting Started](/guide/) guide.

## What is Rstack?

[Rstack](https://rstack.rs/) is a **family of tools** centered on Rspack:

| Tool | Role |
| --- | --- |
| **Rspack** | High-performance bundler (Rust) with webpack-compatible APIs |
| **Rsbuild** | Batteries-included app build tool on Rspack |
| **Rslib** | Library / UI component packaging on Rsbuild |
| **Rspress** | Documentation / static site generator |
| **Rsdoctor** | Build analysis and diagnostics |
| **Rstest** | Testing in the Rstack stack |

You pick the Rstack packages you need rather than a single global CLI that also owns Node and package-manager install flows.

## When Vite+ is a better fit

- You already use **Vite** (or plan to) and want one toolchain for **dev, check, test, build, pack, and install**.
- You want **Oxc-based** lint/format and Vitest without assembling them yourself.
- You care about **migrating** an existing Vite app with `vp migrate` and a single config surface.
- You want **runtime / package-manager** management (`vp env`, `vp install`) next to frontend commands.
- Your stack is framework-on-Vite (React, Vue, Svelte, Solid, …) and you want to stay inside that ecosystem.

## When Rstack is a better fit

- You are invested in **webpack-compatible** tooling, loaders, or plugins and want a faster drop-in path via Rspack.
- You need **Module Federation** or large multi-app compositions where the Rspack/Rsbuild story is primary.
- You prefer composing **specialized packages** (Rsbuild + Rslib + Rspress + Rsdoctor) instead of a single `vp` entrypoint.
- Your organization already standardizes on Web Infra’s stack.

## Overlap (both are “unified toolchains”)

Both projects aim to reduce fragmentation:

- **Performance-first** Rust-powered cores (bundlers: Rolldown vs Rspack; Vite+ also ships Oxc for lint/format/transform)
- **Sensible defaults** so greenfield apps need less config
- **Coverage beyond “just a bundler”** (apps, libraries, quality tooling, and related workflow tools)

They are **not** drop-in replacements for each other. Migrating between Vite+ and Rstack is a project-level decision (bundler model, plugins, config shape, and test runner).

## Side-by-side workflow sketch

```bash
# Vite+
vp create
vp install
vp dev
vp check
vp test
vp build
```

```bash
# Typical Rstack app (illustrative; see Rsbuild docs for current CLI)
# create / scaffold via Rsbuild docs
pnpm create rsbuild   # or equivalent current scaffold
pnpm install
pnpm dev
pnpm build
# lint/test via project scripts or Rstest / ESLint as configured
```

## Related reading

- [Why Vite+](/guide/why)
- [Getting Started](/guide/)
- [Migrate to Vite+](/guide/migrate)
- [Rstack](https://rstack.rs/) · [GitHub org](https://github.com/web-infra-dev)
- [Rspack](https://rspack.rs/) · [Rsbuild](https://rsbuild.rs/) · [Rslib](https://rslib.rs/) · [Rspress](https://rspress.rs/) · [Rstest](https://rstest.rs/) · [Rsdoctor](https://rsdoctor.rs/)

