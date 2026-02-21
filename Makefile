all: check vdp-gl cargo

COMPILER := $(filter g++ clang,$(shell $(CXX) --version))
UNAME_S := $(shell uname)

# All binaries produced by this build
BINS = agon agon-cli agon-cpu agon-vdp agon-vdp-cli
BINDIR = bin

check:
	@if [ ! -f ./src/vdp/userspace-vdp-gl/README.md ]; then echo "Error: no source tree in ./src/vdp/userspace-vdp."; echo "Maybe you forgot to run: git submodule update --init"; echo; exit 1; fi

# Build only the vdp-gl static archive (needed for static-vdp, the default)
vdp-gl:
ifeq ($(UNAME_S),Darwin)
	EXTRA_FLAGS="-Wno-c++11-narrowing -arch arm64" SUFFIX=.arm64 $(MAKE) -C src/vdp/userspace-vdp-gl/src
else
	$(MAKE) -C src/vdp/userspace-vdp-gl/src
endif

# Build all VDP shared libraries (needed for dynamic-vdp and agon-vdp runtime)
vdp:
ifeq ($(UNAME_S),Darwin)
	EXTRA_FLAGS="-Wno-c++11-narrowing -arch arm64" SUFFIX=.arm64 $(MAKE) -C src/vdp
	$(MAKE) -C src/vdp lipo
	find src/vdp -type f \( -name "*.so" -a ! -name "*.x86_64.so" -a ! -name "*.arm64.so" \) -exec cp {} firmware/ \;
	rm -f firmware/vdp_platform.so && cp firmware/vdp_console8.so firmware/vdp_platform.so
else
	$(MAKE) -C src/vdp
	cp src/vdp/*.so firmware/
	rm -f firmware/vdp_platform.so && cp firmware/vdp_console8.so firmware/vdp_platform.so
endif

# Default: static-vdp build of all binaries
cargo:
	@mkdir -p $(BINDIR)
ifeq ($(OS),Windows_NT)
	set FORCE=1 && cargo build -r
	cargo build -r -p agon-cli-emulator -p agon-ez80 -p agon-vdp-sdl -p agon-vdp-cli
	$(foreach b,$(BINS),cp ./target/release/$(b) $(BINDIR)/ 2>/dev/null || true;)
else ifeq ($(UNAME_S),Darwin)
	cargo build -r --target=aarch64-apple-darwin
	cargo build -r --target=aarch64-apple-darwin -p agon-cli-emulator -p agon-ez80 -p agon-vdp-sdl -p agon-vdp-cli
	$(foreach b,$(BINS),lipo -create -output $(BINDIR)/$(b) ./target/aarch64-apple-darwin/release/$(b) 2>/dev/null || true;)
else
	cargo build -r
	cargo build -r -p agon-cli-emulator -p agon-ez80 -p agon-vdp-sdl -p agon-vdp-cli
	$(foreach b,$(BINS),cp ./target/release/$(b) $(BINDIR)/ 2>/dev/null || true;)
endif
	@echo "Built: $(foreach b,$(BINS),$(BINDIR)/$(b) )"

# Dynamic VDP build (loads .so files at runtime)
dynamic: check vdp cargo-dynamic

cargo-dynamic:
	@mkdir -p $(BINDIR)
ifeq ($(OS),Windows_NT)
	set FORCE=1 && cargo build -r --no-default-features --features dynamic-vdp
	cp ./target/release/agon $(BINDIR)/
else ifeq ($(UNAME_S),Darwin)
	FORCE=1 cargo build -r --target=aarch64-apple-darwin --no-default-features --features dynamic-vdp
	lipo -create -output $(BINDIR)/agon ./target/aarch64-apple-darwin/release/agon
else
	FORCE=1 cargo build -r --no-default-features --features dynamic-vdp
	cp ./target/release/agon $(BINDIR)/
endif

vdp-clean:
	rm -f firmware/*.so
ifeq ($(UNAME_S),Darwin)
	EXTRA_FLAGS="-Wno-c++11-narrowing -arch arm64" SUFFIX=.arm64 $(MAKE) -C src/vdp clean
else
	$(MAKE) -C src/vdp clean
endif

cargo-clean:
	rm -rf $(BINDIR)
	cargo clean

clean: vdp-clean cargo-clean

depends:
	$(MAKE) -C src/vdp depends

install:
	install -d $(HOME)/.local/bin
	$(foreach b,$(BINS),install $(BINDIR)/$(b) $(HOME)/.local/bin/$(b);)
	install -d $(HOME)/.local/bin/firmware
	cp -f firmware/vdp_*.so $(HOME)/.local/bin/firmware/ 2>/dev/null || true
	cp -f firmware/mos_*.bin $(HOME)/.local/bin/firmware/ 2>/dev/null || true
	@echo "Installed to ~/.local/bin/: $(BINS) + firmware/"

PREFIX_INSTALL_DIR = $(shell $(BINDIR)/agon --prefix 2>/dev/null)
install-prefix:
ifneq ($(PREFIX_INSTALL_DIR),)
	install -D -t $(PREFIX_INSTALL_DIR)/share/fab-agon-emulator/ firmware/vdp_*.so 2>/dev/null || true
	install -D -t $(PREFIX_INSTALL_DIR)/share/fab-agon-emulator/ firmware/mos_*.bin
	install -D -t $(PREFIX_INSTALL_DIR)/share/fab-agon-emulator/ firmware/mos_*.map 2>/dev/null || true
	$(foreach b,$(BINS),install -D -t $(PREFIX_INSTALL_DIR)/bin/ $(BINDIR)/$(b);)
else
	@echo "make install-prefix requires an install PREFIX (eg PREFIX=/usr/local make)"
endif
