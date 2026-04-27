JXL_BIN := ./jxl_from_tree

.PHONY: build run setup clean-jxl

## Build the Rust project only (jxl_from_tree is optional).
build:
	cargo build --release

## Start the development server.
run: build
	cargo run --release

## Full setup: build jxl_from_tree from source. Safe to re-run — skips
## the libjxl build if the binary already exists. The Rust project is
## built by `make run` (or `make build`), so no second cargo build here.
setup: $(JXL_BIN)
	@echo ""
	@echo "Setup complete.  Start the server with:  make run"

## Build jxl_from_tree from the libjxl source tree (uses system highway/brotli).
$(JXL_BIN):
	./scripts/build_jxl_from_tree.sh $(JXL_BIN)

## Remove the jxl_from_tree binary and its bundled .so files
## (force a rebuild on next 'make setup').
clean-jxl:
	rm -f $(JXL_BIN)
	rm -rf ./lib
