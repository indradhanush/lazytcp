.PHONY: all check build coverage

all: check build

check:
	cargo fmt
	cargo check
	cargo test

build:
	cargo build

coverage:
	./scripts/coverage.sh
