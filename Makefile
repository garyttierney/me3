.PHONY: all dist dist-common dist-windows dist-linux

CARGO ?= cargo
CARGOFLAGS ?= --features=sentry --release
DESTDIR ?= out

WINDOWS_TRIPLE ?= x86_64-pc-windows-msvc
LINUX_TRIPLE ?= x86_64-unknown-linux-gnu
SOURCE_DATE_EPOCH ?= $(shell git log -1 --format=%ct)
COMMIT_ID ?= $(shell git rev-parse --verify HEAD)

SIGNING_CERTIFICATE ?= releng/certificate.pem
SIGNING_KEY ?= e25a03beabebbaa4b79fe76121540ec059794a60
LINUX_BINARIES=$(DESTDIR)/me3

_WINDOWS_BINARIES=me3.exe me3_mod_host.dll me3-launcher.exe
_WINDOWS_INSTALLER=me3_installer.exe
_WINDOWS_BINARY_DESTDIR = $(if $(SIGNED),$(DESTDIR)/signed,$(DESTDIR))

LICENSES=LICENSE-MIT LICENSE-APACHE
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

$(DESTDIR)/me3.spec: me3.spec.rpkg
	rpkg spec -p > $(DESTDIR)/me3.spec

$(DESTDIR)/me3-fedora-42-x86_64.rpm: $(DESTDIR)/me3.spec
	rpkg srpm --outdir $(DESTDIR)
	mock --enable-network -r fedora-42-x86_64 $(DESTDIR)/me3-$(shell rpmspec -q --qf '%{version}' $(DESTDIR)/me3.spec)-$(shell rpmspec -q --qf '%{release}' $(DESTDIR)/me3.spec).src.rpm

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
	@(cd "$(DESTDIR)/dist-linux" && tar --mtime="@$(SOURCE_DATE_EPOCH)" --sort=name --owner=0 --group=0 --numeric-owner -czv -f "$(linux_tarball_path)" ./*)

dist-windows: dist-common $(WINDOWS_BINARIES)
	@rm -rf $(DESTDIR)/dist-windows
	@mkdir -p $(DESTDIR)/dist-windows/bin
	@mkdir -p "$(DESTDIR)/dist-windows/eldenring-mods" "$(DESTDIR)/dist-windows/nightreign-mods"
	@cp -v $(WINDOWS_BINARIES) $(DESTDIR)/dist-windows/bin
	@cp -v -R distribution/cross-platform/* distribution/windows/* $(DESTDIR)/dist-common/* $(DESTDIR)/dist-windows/

dist-linux: $(WINDOWS_BINARIES) $(LINUX_BINARIES) $(DESTDIR)/CHANGELOG.pdf
	install -Dm 0755 -t "$(DESTDIR)/dist-linux/bin" $(LINUX_BINARIES)
	install -Dm 0755 -t "$(DESTDIR)/dist-linux/bin/win64" $(WINDOWS_BINARIES)
	install -Dm 0755 -t "$(DESTDIR)/dist-linux" \
		distribution/linux/install-user.sh \
		distribution/linux/launch-eldenring-mods.sh \
		distribution/linux/launch-nightreign-mods.sh \
		distribution/cross-platform/eldenring-default.me3 \
		distribution/cross-platform/nightreign-default.me3 \
		$(DESTDIR)/CHANGELOG.pdf \
		$(LICENSES)

	install -Dm 0644 -t "$(DESTDIR)/dist-linux/dist" \
		distribution/linux/dist/me3-launch.desktop \
		distribution/linux/dist/me3.xml \
		distribution/cross-platform/dist/me3.png \
		distribution/cross-platform/dist/me3.ico

	install -d "$(DESTDIR)/dist-linux/eldenring-mods" "$(DESTDIR)/dist-linux/nightreign-mods"

dist-common: $(DESTDIR)/CHANGELOG.pdf
	@rm -rf $(DESTDIR)/dist-common
	@mkdir -p $(DESTDIR)/dist-common
	@cp -v $(DESTDIR)/CHANGELOG.pdf $(DESTDIR)/dist-common
	@cp -v LICENSE-APACHE LICENSE-MIT $(DESTDIR)/dist-common
