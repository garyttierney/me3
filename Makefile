.PHONY: build

CARGO = cargo
CARGO_FLAGS = --features=sentry
CARGO_PROFILE=debug

ME3_WINDOWS_BINARIES=target/x86_64-pc-windows-msvc/$(CARGO_PROFILE)/me3.exe \
	target/x86_64-pc-windows-msvc/$(CARGO_PROFILE)/me3_mod_host.dll \
	target/x86_64-pc-windows-msvc/$(CARGO_PROFILE)/me3-launcher.exe
ME3_LINUX_BINARIES=target/x86_64-unknown-linux-musl/$(CARGO_PROFILE)/me3
build:
	@echo "Building $(CARGO_PROFILE) binaries..."
	$(CARGO) build $(CARGO_FLAGS) --target=x86_64-pc-windows-msvc
	$(CARGO) build $(CARGO_FLAGS) --target=x86_64-unknown-linux-musl -p me3-cli


$(ME3_BINARIES): build
me3-linux-amd64.tar.gz: $(ME3_WINDOWS_BINARIES) $(ME3_LINUX_BINARIES) CHANGELOG.md
	tar -czv -f $@ \
		--show-transformed-names --show-stored-names \
		--transform="s|target/x86_64-pc-windows-msvc/$(CARGO_PROFILE)/|bin/win64/|" \
		--transform="s|target/x86_64-unknown-linux-musl/$(CARGO_PROFILE)/|bin/|" \
		--transform="s|distribution/portable/cross-platform/|/|" \
		--transform="s|distribution/portable/linux/|/|" \
		distribution/portable/cross-platform \
		distribution/portable/linux \
		$^

staging_dir := $(shell mktemp -d)
cwd := $(shell pwd)

me3-windows-amd64.zip: $(ME3_WINDOWS_BINARIES)
	mkdir -p "${staging_dir}/bin" && \
	mkdir -p "${staging_dir}/eldenring-mods" "${staging_dir}/nightreign-mods" && \
	cp $(ME3_WINDOWS_BINARIES) "${staging_dir}/bin" && \
	cp -R distribution/portable/cross-platform/* "${staging_dir}" && \
	cp -R distribution/portable/windows/* "${staging_dir}" && \
	cp CHANGELOG.md LICENSE-APACHE LICENSE-MIT "${staging_dir}"

	(cd "${staging_dir}" && zip -r "${cwd}/$@" ./*)
