# Local builds. The version comes from git (see build.rs):
#   tag v1.10.1 checked out      -> 1.10.1
#   8 commits past v1.10.1       -> 1.10.1+8.g0587d41
#   uncommitted changes          -> ...+dirty
# Release builds set TMUX_LEAP_VERSION explicitly, so they keep the plain release version.

PREFIX ?= /usr/local
BIN := target/release/tmux-leap

.PHONY: all build install uninstall version clean

all: build

build:
	cargo build --release --locked

install: build
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/tmux-leap
	@echo "installed tmux-leap $$($(DESTDIR)$(PREFIX)/bin/tmux-leap --version | cut -d' ' -f2) to $(DESTDIR)$(PREFIX)/bin/tmux-leap"

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/tmux-leap

version: build
	@$(BIN) --version

clean:
	cargo clean
