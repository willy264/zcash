PYTHON ?= python3

.PHONY: demo fixture report test check
demo:
	$(PYTHON) scripts/demo.py demo

fixture:
	$(PYTHON) scripts/demo.py fixture

report:
	$(PYTHON) scripts/demo.py report

test:
	cargo test --workspace --locked

check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
