# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Semantic Versioning 2.0.0 for the crate version, `rabun-curator --version`, and `vMAJOR.MINOR.PATCH` git tags
- Create release workflow that tags SemVer 2.0.0 GitHub Releases with Linux binaries

### Changed

- Drop the automatic release-plz PR on push to master. This repository does not allow GitHub Actions to create pull requests; cut versions with Actions → Create release.
