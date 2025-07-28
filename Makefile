.PHONY: all clean dist-linux dist-windows
.DELETE_ON_ERROR:
.ONESHELL:
SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c

CARGO_HOME ?= ${HOME}/.cargo
RUSTUP_HOME ?= ${HOME}/.rustup

CARGOFLAGS ?= --features=sentry --release
DESTDIR ?= out
SOURCE_DATE_EPOCH ?= $(shell git log -1 --format=%ct)
COMMIT ?= $(shell git rev-parse --verify HEAD)
RUSTFLAGS ?= 

linux_target_triple = x86_64-unknown-linux-gnu
linux_tarball_path := $(abspath $(DESTDIR)/me3-linux-amd64.tar.gz)
windows_target_triple = x86_64-pc-windows-msvc
windows_zip_path := $(abspath $(DESTDIR)/me3-windows-amd64.zip)

signing_cert = releng/certificate.pem
signing_key ?= e25a03beabebbaa4b79fe76121540ec059794a60

license_files=LICENSE-MIT LICENSE-APACHE
windows_binaries=$(addprefix $(DESTDIR)/,me3.exe me3_mod_host.dll me3-launcher.exe)

all: $(DESTDIR)/me3-windows-amd64.zip $(DESTDIR)/me3-linux-amd64.tar.gz $(DESTDIR)/me3_installer.exe
clean:
	rm -Rf $(DESTDIR)

cargo_build = SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) \
		BUILD_COMMIT_ID=$(COMMIT) \
		RUSTC_BOOTSTRAP=1 \
		RUSTC_WRAPPER=support/rustc.wrapper \
		cargo build --locked $(CARGOFLAGS) \
		-v \
        --target $(1) \
        -Z unstable-options --artifact-dir=$(DESTDIR)/ \
        -p $(2)


$(DESTDIR)/CHANGELOG.pdf: CHANGELOG.md
	pandoc -t html $< -o $@

$(DESTDIR)/me3 $(DESTDIR)/me3.debug:
	$(call cargo_build,$(linux_target_triple),me3-cli)
	objcopy --only-keep-debug out/me3 out/me3.debug
	objcopy --strip-debug out/me3
	objcopy --add-gnu-debuglink=out/me3.debug out/me3

$(DESTDIR)/me3.exe:
	$(call cargo_build,$(windows_target_triple),me3-cli)

$(DESTDIR)/me3-launcher.exe:
	$(call cargo_build,$(windows_target_triple),me3-launcher)

$(DESTDIR)/me3_mod_host.dll:
	$(call cargo_build,$(windows_target_triple),me3-mod-host)

$(DESTDIR)/me3_installer.exe: $(windows_binaries) $(DESTDIR)/CHANGELOG.pdf
	makensis -DTARGET_DIR=$(shell dirname $<)/ installer.nsi -X"OutFile $@"

$(DESTDIR)/me3-windows-amd64.zip: dist-windows
	cd "$(DESTDIR)/dist-windows"
	zip -r "$(windows_zip_path)" ./*

$(DESTDIR)/me3-linux-amd64.tar.gz: dist-linux
	cd "$(DESTDIR)/dist-linux"
	tar --mtime="@$(SOURCE_DATE_EPOCH)" --sort=name --owner=0 --group=0 --numeric-owner -czv -f "$(linux_tarball_path)" ./*

dist-windows: $(windows_binaries) $(DESTDIR)/CHANGELOG.pdf
	install -d "$(DESTDIR)/dist-windows/eldenring-mods" "$(DESTDIR)/dist-linux/nightreign-mods"
	install -D -t "$(DESTDIR)/dist-windows/bin" $(windows_binaries)
	install -Dm 0755 -t "$(DESTDIR)/dist-windows" \
		distribution/windows/launch-eldenring-mods.bat \
		distribution/windows/launch-nightreign-mods.bat \
		distribution/cross-platform/eldenring-default.me3 \
		distribution/cross-platform/nightreign-default.me3 \
		$(DESTDIR)/CHANGELOG.pdf \
		$(license_files)

dist-linux: $(windows_binaries) $(DESTDIR)/me3 $(DESTDIR)/CHANGELOG.pdf
	mkdir -p "$(DESTDIR)/dist-linux/eldenring-mods" "$(DESTDIR)/dist-linux/nightreign-mods"
	install -Dm 0755 -t "$(DESTDIR)/dist-linux/bin" $(DESTDIR)/me3
	install -Dm 0755 -t "$(DESTDIR)/dist-linux/bin/win64" $(windows_binaries)
	install -Dm 0755 -t "$(DESTDIR)/dist-linux" \
		distribution/linux/install-user.sh \
		distribution/linux/launch-eldenring-mods.sh \
		distribution/linux/launch-nightreign-mods.sh \
		distribution/cross-platform/eldenring-default.me3 \
		distribution/cross-platform/nightreign-default.me3 \
		$(DESTDIR)/CHANGELOG.pdf \
		$(license_files)

	install -Dm 0644 -t "$(DESTDIR)/dist-linux/dist" \
		distribution/linux/dist/me3-launch.desktop \
		distribution/linux/dist/me3.xml \
		distribution/cross-platform/dist/me3.png \
		distribution/cross-platform/dist/me3.ico

	install -d "$(DESTDIR)/dist-linux/eldenring-mods" "$(DESTDIR)/dist-linux/nightreign-mods"

.NOTPARALLEL: $(windows_binaries)