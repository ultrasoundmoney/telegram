# Releasing

This crate is released for internal consumption through git tags rather than
crates.io.

## Versioning

- Keep `Cargo.toml` as the source of truth for the crate version.
- Use SemVer-style versions.
- Tag releases as `vMAJOR.MINOR.PATCH`, matching the package version exactly.
- Ask downstream services to depend on tags instead of floating branches:

```toml
telegram = { git = "https://github.com/ultrasoundmoney/telegram", tag = "v0.2.0" }
```

Services can log `telegram::VERSION` at startup to make the selected release
visible at runtime.

## Checklist

1. Update `version` in `Cargo.toml`.
2. Update `Cargo.lock` if the package version changed.
3. Update `CHANGELOG.md` with the release date and notable changes.
4. Run `cargo fmt --all -- --check`.
5. Run `cargo test --all-features --all-targets`.
6. Run `cargo clippy --all-features --all-targets -- -D warnings`.
7. Commit the release changes.
8. Create an annotated tag: `git tag -a v0.2.0 -m "Release v0.2.0"`.
9. Push the commit and tag: `git push origin main v0.2.0`.
