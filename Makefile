.PHONY: all fmt test check coverage

all: fmt test check

fmt:
	cargo fmt

test:
	cargo test

check:
	cargo check

coverage:
	./scripts/coverage.sh
