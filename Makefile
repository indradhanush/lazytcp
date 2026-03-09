.PHONY: all fmt test check

all: fmt test check

fmt:
	cargo fmt

test:
	cargo test

check:
	cargo check
