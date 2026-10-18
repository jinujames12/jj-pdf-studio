# Contributing to JJ PDF Studio

Thanks for helping! Please read our [Code of Conduct](CODE_OF_CONDUCT.md) first.

## Ways to help

- **Bug reports:** use the bug template. Include your Windows version, the app version and, if you
  can share it safely, a sample PDF that triggers the problem.
- **Feature ideas:** use the feature request template.
- **Code:** pick an open issue, or discuss your idea in an issue before a large change.
- **Translations:** planned for later.

## Development setup

Prerequisites (Windows 10/11 x64):

- Rust stable with the MSVC toolchain, plus Visual Studio Build Tools 2022 with the
  "Desktop development with C++" workload
- Node.js 20 LTS or newer
- The WebView2 runtime (already present on Windows 11)

```powershell
npm install
npm run tauri dev      # run with hot reload
npm run tauri build    # release build
```

The portable exe is written to `src-tauri\target\release\jj-pdf-studio.exe`.

## Project layout

```
crates/pdf-core/   pure Rust PDF logic, no Tauri dependency (unit and integration tests)
src-tauri/         the Tauri app: thin commands that call pdf-core
src/               Svelte + TypeScript frontend (components, stores, design tokens)
assets/brand/      logo and brand assets (all rights reserved, see its LICENSE.md)
scripts/           small maintenance scripts (for example set-version.mjs)
tests/fixtures/    small sample PDFs and images (your own or public domain)
```

All PDF writing happens in Rust; the frontend only renders previews and describes what the user
wants. See `SPEC.md` for the full design.

## Coding standards

- Rust: `cargo fmt --all` and `cargo clippy --workspace -- -D warnings` must pass.
- Frontend: `npm run lint` (ESLint and Prettier) and `npm run check` (svelte-check) must pass.
  `npm run format` fixes formatting.
- PDF logic belongs in `crates/pdf-core`, with tests.
- No network code. The app must work fully offline.
- No new dependency without a permissive license (MIT, Apache-2.0, BSD, ISC, Zlib; OFL for
  fonts). Never add AGPL or GPL code. Mention new dependencies in your PR.

## Tests

```powershell
cargo test --workspace
```

For every bug fix, add a fixture to `tests/fixtures/` (it must be your own work or public domain)
and an integration test that fails without the fix.

## Commits

We use [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`,
`refactor:`, `test:`, `chore:`.

## Pull request checklist

- [ ] Code is formatted and all checks above pass
- [ ] Tests added or updated
- [ ] No new network access; no new dependency without a permissive license
- [ ] `CHANGELOG.md` updated for user-visible changes
- [ ] Screenshots for UI changes

## Releasing (maintainers)

1. Update `CHANGELOG.md`.
2. Run `npm run version <x.y.z>` to set the version in `package.json`, `Cargo.toml` and
   `src-tauri/tauri.conf.json`, then `cargo check` to refresh `Cargo.lock`.
3. Commit, then `git tag v<x.y.z>` and `git push --tags`.
4. CI builds a draft release with the portable exe, installer and checksums. Review and publish it.

## License of contributions

By submitting a pull request you agree that your contribution is licensed under the
[Apache License 2.0](LICENSE) (inbound = outbound, no CLA).
