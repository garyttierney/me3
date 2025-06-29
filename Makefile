.PHONY: all dist dist-common dist-windows dist-linux

CARGO ?= cargo
CARGOFLAGS ?= --features=sentry --release
DESTDIR ?= out

WINDOWS_TRIPLE ?= x86_64-pc-windows-msvc
LINUX_TRIPLE ?= x86_64-unknown-linux-gnu
SOURCE_DATE_EPOCH=$(shell git log -1 --format=%ct)
COMMIT_ID=$(shell git rev-parse --verify HEAD)

SIGNING_CERTIFICATE ?= releng/certificate.pem
SIGNING_KEY ?= e25a03beabebbaa4b79fe76121540ec059794a60
LINUX_BINARIES=$(DESTDIR)/me3

_WINDOWS_BINARIES=me3.exe me3_mod_host.dll me3-launcher.exe
_WINDOWS_INSTALLER=me3_installer.exe
_WINDOWS_BINARY_DESTDIR = $(if $(SIGNED),$(DESTDIR)/signed,$(DESTDIR))

WINDOWS_BINARIES=$(addprefix $(_WINDOWS_BINARY_DESTDIR)/,$(_WINDOWS_BINARIES))
INSTALLER_BINARY=$(addprefix $(_WINDOWS_BINARY_DESTDIR)/,$(_WINDOWS_INSTALLER))
DIST_FILES=$(INSTALLER_BINARY) $(DESTDIR)/me3-windows-amd64.zip $(DESTDIR)/me3-linux-amd64.tar.gz

ifeq ($(SIGNED),1)
	DIST_FILES+=$(addsuffix .sig,$(INSTALLER_BINARY) $(DESTDIR)/me3-windows-amd64.zip $(DESTDIR)/me3-linux-amd64.tar.gz)
endif

all: $(DIST_FILES)
clean:
	rm -Rf $(DESTDIR)/

%.sig: %
	rm -f $@
	gpg -o $@ -b $<

$(DESTDIR)/%.pdf: %.md
	pandoc -t html $< -o $@

$(DESTDIR)/me3:
	SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) BUILD_COMMIT_ID=$(COMMIT_ID) $(CARGO) build $(CARGOFLAGS) \
		--target $(LINUX_TRIPLE) \
		-Z unstable-options --artifact-dir=$(DESTDIR)/ \
		-p me3-cli

$(DESTDIR)/me3_installer.exe: $(WINDOWS_BINARIES) $(DESTDIR)/CHANGELOG.pdf
	makensis -DTARGET_DIR=$(shell dirname $<)/ installer.nsi -X"OutFile $@"

$(DESTDIR)/me3.exe $(DESTDIR)/me3-launcher.exe $(DESTDIR)/me3_mod_host.dll:
	SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) BUILD_COMMIT_ID=$(COMMIT_ID) $(CARGO) build $(CARGOFLAGS) \
		--target $(WINDOWS_TRIPLE) \
		-Z unstable-options --artifact-dir=$(DESTDIR)/ \
		-p me3-launcher -p me3-mod-host -p me3-cli

$(DESTDIR)/signed/%: $(DESTDIR)/%
	$(if $(value SIGNING_PIN),,$(error "signing pin not set"))
	@mkdir -p $(DESTDIR)/signed
	@rm -f $@
	@osslsigncode sign \
		-verbose \
		-pkcs11module /opt/proCertumCardManager/sc30pkcs11-*.so \
		-certs $(SIGNING_CERTIFICATE) \
		-key $(SIGNING_KEY) \
		-pass $(SIGNING_PIN) \
		-h sha256 \
		-t http://time.certum.pl/ \
		-in $< \
		-out $@

windows_zip_path := $(abspath $(DESTDIR)/me3-windows-amd64.zip)
$(DESTDIR)/me3-windows-amd64.zip: dist-windows
	@(cd "$(DESTDIR)/dist-windows" && zip -r "$(windows_zip_path)" ./*)

linux_tarball_path := $(abspath $(DESTDIR)/me3-linux-amd64.tar.gz)
$(DESTDIR)/me3-linux-amd64.tar.gz: dist-linux
	@(cd "$(DESTDIR)/dist-linux" && tar --mtime="@0" --sort=name --owner=0 --group=0 --numeric-owner -czv -f "$(linux_tarball_path)" ./*)

dist-windows: dist-common $(WINDOWS_BINARIES)
	@rm -rf $(DESTDIR)/dist-windows
	@mkdir -p $(DESTDIR)/dist-windows/bin
	@mkdir -p "$(DESTDIR)/dist-windows/eldenring-mods" "$(DESTDIR)/dist-windows/nightreign-mods"
	@cp -v $(WINDOWS_BINARIES) $(DESTDIR)/dist-windows/bin
	@cp -v -R distribution/portable/cross-platform/* distribution/portable/windows/* $(DESTDIR)/dist-common/* $(DESTDIR)/dist-windows/

dist-linux: dist-common $(WINDOWS_BINARIES) $(LINUX_BINARIES)
	@rm -rf $(DESTDIR)/dist-linux
	@mkdir -p $(DESTDIR)/dist-linux/bin/win64
	@mkdir -p "$(DESTDIR)/dist-linux/eldenring-mods" "$(DESTDIR)/dist-linux/nightreign-mods"
	@cp -v $(LINUX_BINARIES) $(DESTDIR)/dist-linux/bin
	@cp -v $(WINDOWS_BINARIES) $(DESTDIR)/dist-linux/bin/win64
	@cp -v -R distribution/portable/cross-platform/* distribution/portable/linux/* $(DESTDIR)/dist-common/* $(DESTDIR)/dist-linux/

dist-common: $(DESTDIR)/CHANGELOG.pdf
	@rm -rf $(DESTDIR)/dist-common
	@mkdir -p $(DESTDIR)/dist-common
	@cp -v $(DESTDIR)/CHANGELOG.pdf $(DESTDIR)/dist-common
	@cp -v LICENSE-APACHE LICENSE-MIT $(DESTDIR)/dist-common
