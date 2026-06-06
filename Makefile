JXL_BIN := ./jxl_from_tree
DJXL_BIN := ./djxl

.PHONY: build run setup clean-jxl

## Build the Rust project only (jxl_from_tree + djxl are optional).
build:
	cargo build --release

## Start the development server.
run: build
	cargo run --release

## Full setup: build jxl_from_tree (encoder) + djxl (decoder) from source.
## Safe to re-run — skips the libjxl build only when BOTH binaries already
## exist (one libjxl build produces both). The Rust project is built by
## `make run` (or `make build`), so no second cargo build here.
setup:
	@if [ -x "$(JXL_BIN)" ] && [ -x "$(DJXL_BIN)" ]; then \
		echo "jxl_from_tree + djxl already present — skipping libjxl build."; \
	else \
		./scripts/build_jxl_from_tree.sh $(JXL_BIN); \
	fi
	@git config core.hooksPath .githooks && echo "Installed pre-commit hook (core.hooksPath = .githooks)."
	@echo ""
	@echo "Setup complete.  Start the server with:  make run"

## Remove the built binaries and bundled .so files
## (force a rebuild on next 'make setup').
clean-jxl:
	rm -f $(JXL_BIN) $(DJXL_BIN)
	rm -rf ./lib
