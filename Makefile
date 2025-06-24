.PHONY: all dist dist-common dist-windows dist-linux

CARGO ?= cargo
CARGOFLAGS ?= --features=sentry --release

WINDOWS_TRIPLE ?= x86_64-pc-windows-msvc
LINUX_TRIPLE ?= x86_64-unknown-linux-musl
SOURCE_DATE_EPOCH=$(shell git log -1 --format=%ct)

ME3_SIGNING_CERTIFICATE ?= releng/certificate.pem
ME3_SIGNING_KEY ?= e25a03beabebbaa4b79fe76121540ec059794a60
ME3_LINUX_BINARIES=out/me3
ifeq ($(ME3_SIGNED),1)
	ME3_WINDOWS_BINARIES=out/signed/me3.exe out/signed/me3-launcher.exe out/signed/me3_mod_host.dll
	ME3_INSTALLER_BINARY=out/signed/me3_installer.exe
	ME3_DIST_FILES=$(ME3_INSTALLER_BINARY) $(ME3_INSTALLER_BINARY).sig out/me3-linux-amd64.tar.gz out/me3-linux-amd64.tar.gz.sig out/me3-windows-amd64.zip out/me3-windows-amd64.zip.sig
else
	ME3_WINDOWS_BINARIES=out/me3.exe out/me3-launcher.exe out/me3_mod_host.dll
	ME3_INSTALLER_BINARY=out/me3_installer.exe
	ME3_DIST_FILES=$(ME3_INSTALLER_BINARY) out/me3-linux-amd64.tar.gz out/me3-windows-amd64.zip
endif


all: dist
clean:
	rm -Rf out/

%.sig: %
	rm -f $@
	gpg -o $@ -b $<

out/%.pdf: %.md
	pandoc -t html $< -o $@

out/me3:
	SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) $(CARGO) build $(CARGOFLAGS) \
		--target $(LINUX_TRIPLE) \
		-Z unstable-options --artifact-dir=out/ \
		-p me3-cli

out/me3_installer.exe: $(ME3_WINDOWS_BINARIES) out/CHANGELOG.pdf
	makensis -DTARGET_DIR=$(shell dirname $<)/ installer.nsi -X"OutFile $@"

out/me3.exe out/me3-launcher.exe out/me3_mod_host.dll:
	SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) $(CARGO) build $(CARGOFLAGS) \
		--target $(WINDOWS_TRIPLE) \
		-Z unstable-options --artifact-dir=out/ \
		-p me3-launcher -p me3-mod-host -p me3-cli

out/signed/%: out/%
	$(if $(value ME3_SIGNING_PIN),,$(error "signing pin not set"))
	@mkdir -p out/signed
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

dist: $(ME3_DIST_FILES)
	@mkdir -p dist
	@cp -v $^ dist/

cwd := $(shell pwd)

out/me3-windows-amd64.zip: dist-windows
	@(cd "out/dist-windows" && zip -r "${cwd}/$@" ./*)

out/me3-linux-amd64.tar.gz: dist-linux
	@(cd "out/dist-linux" && tar --mtime="@0" --sort=name --owner=0 --group=0 --numeric-owner -czv -f "${cwd}/$@" ./*)

dist-windows: dist-common $(ME3_WINDOWS_BINARIES)
	@rm -rf out/dist-windows
	@mkdir -p out/dist-windows/bin
	@mkdir -p "out/dist-windows/eldenring-mods" "out/dist-windows/nightreign-mods"
	@cp -v $(ME3_WINDOWS_BINARIES) out/dist-windows/bin
	@cp -v -R distribution/portable/cross-platform/* distribution/portable/windows/* out/dist-common/* out/dist-windows/

dist-linux: dist-common $(ME3_WINDOWS_BINARIES) $(ME3_LINUX_BINARIES)
	@rm -rf out/dist-linux
	@mkdir -p out/dist-linux/bin/win64
	@mkdir -p "out/dist-linux/eldenring-mods" "out/dist-linux/nightreign-mods"
	@cp -v $(ME3_LINUX_BINARIES) out/dist-linux/bin
	@cp -v $(ME3_WINDOWS_BINARIES) out/dist-linux/bin/win64
	@cp -v -R distribution/portable/cross-platform/* distribution/portable/linux/* out/dist-common/* out/dist-linux/

dist-common: out/CHANGELOG.pdf
	@rm -rf out/dist-common
	@mkdir -p out/dist-common
	@cp -v out/CHANGELOG.pdf out/dist-common
	@cp -v LICENSE-APACHE LICENSE-MIT out/dist-common
