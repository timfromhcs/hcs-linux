SHELL := /bin/bash
VERSION ?= 2.0.0
ARCH ?= amd64
DIST_DIR ?= dist
ISO_NAME ?= HCS-Linux-$(VERSION)-$(ARCH).iso
ISO_PATH ?= $(DIST_DIR)/$(ISO_NAME)

# The v2 gate names. `make gate-all` runs every one of them; see
# docs/V2_STABLE_RELEASE_MASTER_PLAN.md §7.1. The lettered targets exist so a
# single gate can be run in isolation while debugging.
.PHONY: all help fetch native test iso verify qa gui a11y payload models sbom \
        gate-static gate-security gate-supply gate-stress gate-all clean

all: native test

help:
	@echo "HCS Linux Build System"
	@echo "======================"
	@echo "make fetch       - Fetch external dependencies and verify lockfiles"
	@echo "make native      - Compile core Rust daemons and GUI binaries"
	@echo "make test        - Run unit and integration test suites"
	@echo "make iso         - Build the Debian 13 (Trixie) hybrid Live ISO"
	@echo "make verify      - Audit ISO structure, checksums, and manifest"
	@echo "make qa          - Stress, security and visual validation"
	@echo "make gui         - GUI gates: headless render, regression, RAM budget"
	@echo "make a11y        - Accessibility gate: focus order, WCAG contrast, motion"
	@echo "make payload     - ISO payload contract (every required file present)"
	@echo "make models      - Report which starter models may be baked into the ISO"
	@echo "make sbom        - Generate SPDX SBOM and third-party notices"
	@echo "make gate-all    - Every release gate, in order"
	@echo "make clean       - Remove build artifacts and temporary caches"
	@echo ""
	@echo "Linux build deps for the GUI crates (Slint software renderer with"
	@echo "system fonts): pkg-config libfontconfig-dev libfreetype-dev"

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

gui:
	@for theme in obsidian titanium stealth high_contrast; do \
		python3 scripts/verify_gui.py --render-only --theme $$theme || exit 1; \
		python3 scripts/verify_gui.py --regress --theme $$theme || exit 1; \
	done
	python3 scripts/gui_ram_audit.py

a11y:
	@cargo run --quiet --bin hcs-a11y-check -- --report qa/reports/a11y.json

payload:
	@if [ -d target/rootfs ]; then \
		./scripts/verify_payload.sh target/rootfs $(VERSION); \
	elif [ -f "$(ISO_PATH)" ]; then \
		./scripts/verify_payload.sh --squashfs target/iso_staging/live/filesystem.squashfs $(VERSION); \
	else \
		echo "No staged rootfs and no ISO. Run 'make iso' first."; \
		exit 1; \
	fi

models:
	python3 scripts/stage_starter_models.py --check

sbom:
	python3 scripts/generate_sbom.py

# --- Gates -------------------------------------------------------------------
# Ordered cheapest-first so a formatting mistake does not cost a full ISO build.

gate-static:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

gate-security:
	python3 scripts/run_security_audit.py

gate-supply:
	python3 scripts/verify_sources.py
	python3 scripts/stage_starter_models.py --check

gate-stress:
	python3 scripts/run_stress_test.py

# Every gate. `make iso` is included because the payload contract can only be
# checked against a built image, and v1 shipped a payload that had silently lost
# its launchers, icons and manuals.
gate-all: gate-static gate-security gate-supply gui a11y iso verify payload gate-stress
	@echo ""
	@echo "=================================================="
	@echo "  ALL GATES GREEN — see qa/reports/ for evidence"
	@echo "=================================================="

clean:
	cargo clean || true
	rm -rf $(DIST_DIR)/*.iso $(DIST_DIR)/*.squashfs $(DIST_DIR)/*.tmp
	rm -rf target/
