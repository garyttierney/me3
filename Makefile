.PHONY: all dist dist-common dist-windows dist-linux

CARGO ?= cargo
CARGOFLAGS ?= --features=sentry --release
DESTDIR ?= out

WINDOWS_TRIPLE ?= x86_64-pc-windows-msvc
LINUX_TRIPLE ?= x86_64-unknown-linux-gnu
SOURCE_DATE_EPOCH=$(shell git log -1 --format=%ct)

ME3_SIGNING_CERTIFICATE ?= releng/certificate.pem
ME3_SIGNING_KEY ?= e25a03beabebbaa4b79fe76121540ec059794a60
ME3_LINUX_BINARIES=$(DESTDIR)/me3

_WINDOWS_BINARIES=me3.exe me3_mod_host.dll me3-launcher.exe
_WINDOWS_INSTALLER=me3_installer.exe
_WINDOWS_BINARY_DESTDIR = $(if $(ME3_SIGNED),$(DESTDIR)/signed,$(DESTDIR))

ME3_WINDOWS_BINARIES=$(addprefix $(_WINDOWS_BINARY_DESTDIR)/,$(_WINDOWS_BINARIES))
ME3_INSTALLER_BINARY=$(addprefix $(_WINDOWS_BINARY_DESTDIR)/,$(_WINDOWS_INSTALLER))
ME3_DIST_FILES = $(ME3_INSTALLER_BINARY) $(ME3_WINDOWS_BINARIES)

ifeq ($(ME3_SIGNED),1)
	ME3_DIST_FILES += $(addsuffix .sig,$(ME3_DIST_FILES))
endif

all: dist
clean:
	rm -Rf $(DESTDIR)/

%.sig: %
	rm -f $@
	gpg -o $@ -b $<

$(DESTDIR)/%.pdf: %.md
	pandoc -t html $< -o $@

$(DESTDIR)/me3:
	SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) $(CARGO) build $(CARGOFLAGS) \
		--target $(LINUX_TRIPLE) \
		-Z unstable-options --artifact-dir=$(DESTDIR)/ \
		-p me3-cli

$(DESTDIR)/me3_installer.exe: $(ME3_WINDOWS_BINARIES) $(DESTDIR)/CHANGELOG.pdf
	makensis -DTARGET_DIR=$(shell dirname $<)/ installer.nsi -X"OutFile $@"

$(DESTDIR)/me3.exe $(DESTDIR)/me3-launcher.exe $(DESTDIR)/me3_mod_host.dll:
	SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) $(CARGO) build $(CARGOFLAGS) \
		--target $(WINDOWS_TRIPLE) \
		-Z unstable-options --artifact-dir=$(DESTDIR)/ \
		-p me3-launcher -p me3-mod-host -p me3-cli

$(DESTDIR)/signed/%: $(DESTDIR)/%
	$(if $(value ME3_SIGNING_PIN),,$(error "signing pin not set"))
	@mkdir -p $(DESTDIR)/signed
	@rm -f $@
	@osslsigncode sign \
		-verbose \
		-pkcs11module /opt/proCertumCardManager/sc30pkcs11-*.so \
		-certs $(ME3_SIGNING_CERTIFICATE) \
		-key $(ME3_SIGNING_KEY) \
		-pass $(ME3_SIGNING_PIN) \
		-h sha256 \
		-t http://time.certum.pl/ \
		-in $< \
		-out $@

upload-release-binaries: dist
	$(if $(value ME3_RELEASE_TAG),,$(error "release tag not set"))
	gh release upload --clobber -R garyttierney/me3 $(ME3_RELEASE_TAG) \
          'dist/me3_installer.exe#me3_installer.exe (Installer for Windows)' \
          'dist/me3-windows-amd64.zip#me3-windows-amd64.zip (Portable distribution for Windows)' \
          'dist/me3-linux-amd64.tar.gz#me3-linux-amd64.tar.gz (Portable distribution for Linux)' \
		  dist/*.sig


dist: $(ME3_DIST_FILES)
	@mkdir -p dist
	@cp -v $^ dist/

cwd := $(shell pwd)

$(DESTDIR)/me3-windows-amd64.zip: dist-windows
	@(cd "$(DESTDIR)/dist-windows" && zip -r "${cwd}/$@" ./*)

$(DESTDIR)/me3-linux-amd64.tar.gz: dist-linux
	@(cd "$(DESTDIR)/dist-linux" && tar --mtime="@0" --sort=name --owner=0 --group=0 --numeric-owner -czv -f "${cwd}/$@" ./*)

dist-windows: dist-common $(ME3_WINDOWS_BINARIES)
	@rm -rf $(DESTDIR)/dist-windows
	@mkdir -p $(DESTDIR)/dist-windows/bin
	@mkdir -p "$(DESTDIR)/dist-windows/eldenring-mods" "$(DESTDIR)/dist-windows/nightreign-mods"
	@cp -v $(ME3_WINDOWS_BINARIES) $(DESTDIR)/dist-windows/bin
	@cp -v -R distribution/portable/cross-platform/* distribution/portable/windows/* $(DESTDIR)/dist-common/* $(DESTDIR)/dist-windows/

dist-linux: dist-common $(ME3_WINDOWS_BINARIES) $(ME3_LINUX_BINARIES)
	@rm -rf $(DESTDIR)/dist-linux
	@mkdir -p $(DESTDIR)/dist-linux/bin/win64
	@mkdir -p "$(DESTDIR)/dist-linux/eldenring-mods" "$(DESTDIR)/dist-linux/nightreign-mods"
	@cp -v $(ME3_LINUX_BINARIES) $(DESTDIR)/dist-linux/bin
	@cp -v $(ME3_WINDOWS_BINARIES) $(DESTDIR)/dist-linux/bin/win64
	@cp -v -R distribution/portable/cross-platform/* distribution/portable/linux/* $(DESTDIR)/dist-common/* $(DESTDIR)/dist-linux/

dist-common: $(DESTDIR)/CHANGELOG.pdf
	@rm -rf $(DESTDIR)/dist-common
	@mkdir -p $(DESTDIR)/dist-common
	@cp -v $(DESTDIR)/CHANGELOG.pdf $(DESTDIR)/dist-common
	@cp -v LICENSE-APACHE LICENSE-MIT $(DESTDIR)/dist-common
