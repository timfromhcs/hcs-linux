SHELL := /bin/bash
VERSION ?= 0.1.0-alpha.1
ARCH ?= amd64
DIST_DIR ?= dist
ISO_NAME ?= HCS-Linux-$(VERSION)-$(ARCH).iso
ISO_PATH ?= $(DIST_DIR)/$(ISO_NAME)

.PHONY: all help fetch native test iso verify qa clean lint sbom

all: native test

help:
	@echo "HCS Linux Build System"
	@echo "======================"
	@echo "make fetch    - Fetch external dependencies and verify lockfiles"
	@echo "make native   - Compile core Rust daemons (hcsd, hcs-modeld, etc.)"
	@echo "make test     - Run unit and integration test suites"
	@echo "make iso      - Build the Debian 13 (Trixie) hybrid Live ISO"
	@echo "make verify   - Audit ISO structure, checksums, and manifest"
	@echo "make qa       - Run stress, visual QA, and security validation"
	@echo "make sbom     - Generate SPDX SBOM and third-party notices"
	@echo "make clean    - Remove build artifacts and temporary caches"

fetch:
	python3 scripts/fetch_models.py --verify-only || python scripts/fetch_models.py --verify-only
	python3 scripts/verify_sources.py || python scripts/verify_sources.py

native:
	cargo build --release --workspace

test:
	cargo test --workspace
	python3 -m unittest discover -s tests/unit -p "test_*.py" || python -m unittest discover -s tests/unit -p "test_*.py"

iso:
	@mkdir -p $(DIST_DIR)
	./scripts/build_iso.sh $(VERSION) $(ARCH)

verify:
	@if [ -f "$(ISO_PATH)" ]; then \
		sha256sum "$(ISO_PATH)" > $(DIST_DIR)/SHA256SUMS; \
		python3 scripts/verify_iso.py "$(ISO_PATH)"; \
	else \
		echo "ISO not found at $(ISO_PATH). Run 'make iso' first."; \
		exit 1; \
	fi

qa:
	python3 scripts/run_stress_test.py
	python3 scripts/run_security_audit.py

sbom:
	python3 scripts/generate_sbom.py

clean:
	cargo clean || true
	rm -rf $(DIST_DIR)/*.iso $(DIST_DIR)/*.squashfs $(DIST_DIR)/*.tmp
	rm -rf target/
