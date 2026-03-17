.PHONY: all fmt test check build coverage

all: fmt test check build

fmt:
	cargo fmt

test:
	cargo test

check:
	cargo check

build:
	cargo build

coverage:
	./scripts/coverage.sh
