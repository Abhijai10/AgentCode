.PHONY: bootstrap format format-check lint typecheck test build daemon-build validate clean app

bootstrap:
	cargo --version

format:
	cargo fmt --all

format-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

typecheck:
	cargo check --workspace --all-targets

test:
	cargo test --workspace

build:
	cargo build --workspace

daemon-build:
	cargo build -p ac-daemon

desktop-build:
	cargo build -p ac-desktop-placeholder

migrations-check:
	cargo test -p ac-migrations

validate: format-check lint typecheck test

clean:
	cargo clean

# The ONE command that puts the latest app on this Mac: release build +
# .app bundle (daemon sidecar inside) + DMG (hdiutil, no create-dmg) +
# install to /Applications.
app:
	bash ./scripts/bundle-mac.sh
