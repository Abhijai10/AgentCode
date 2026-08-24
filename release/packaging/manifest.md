# Release Packaging Manifest

Target: macOS ARM64 desktop product.

Bundle components:

- desktop application shell
- daemon
- SQLite migrations
- approved managed-tool metadata
- release diagnostics policy

Application state directories must be outside user repositories:

- config
- SQLite data
- indexes
- cache
- logs
- managed binaries
- browser profiles
- artifacts

Windows packaging remains future scope.
