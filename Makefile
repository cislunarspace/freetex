.PHONY: build test fmt lint run clean ensure-frontend-deps e2e

TAURI_CLI = npx --prefix frontend tauri

ensure-frontend-deps:
	npm --prefix frontend install

build: ensure-frontend-deps
	$(TAURI_CLI) build

test:
	cargo test --manifest-path=src-tauri/Cargo.toml

# 引擎端到端测试，需要 .dev/models/（见 README）
# Engine E2E test; requires .dev/models/ (see README)
e2e:
	cargo test --manifest-path=src-tauri/Cargo.toml --lib e2e -- --ignored --nocapture

fmt:
	cargo fmt --manifest-path=src-tauri/Cargo.toml -- --check

lint:
	cargo clippy --manifest-path=src-tauri/Cargo.toml -- -D warnings

run: ensure-frontend-deps
	$(TAURI_CLI) dev

clean:
	cargo clean --manifest-path=src-tauri/Cargo.toml
	rm -rf frontend/dist
