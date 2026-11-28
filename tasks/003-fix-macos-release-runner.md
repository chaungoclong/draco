# Task 003 — Fix macOS GitHub Actions Release Runner

- **Status**: `ready_for_review`
- **Branch**: `fix/macos-release-runner`
- **Date**: 2026-11-28

## 1. Scope
- **Issue**: GitHub Actions rejected `macos-14` (`The configuration 'macos-14-arm64-us-default' is not supported`) during `v0.1.0` release build.
- **Fix**: Switch both `aarch64-apple-darwin` and `x86_64-apple-darwin` matrix entries in `.github/workflows/release.yml` to `macos-latest` (Apple Silicon runner with universal Xcode SDK), and update `docs/specs/008-release-pipeline-and-distribution.md`.

## 2. Test Report
- **PASS**:
  - `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`: `YAML OK`.
- **FAIL**: None.
- **NOT RUN**: Remote macOS runner execution (verified on GitHub when re-tagging or dispatching `Release`).
