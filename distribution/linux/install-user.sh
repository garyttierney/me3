#!/bin/sh

# Install me3 for the current user

bindir=$HOME/.local/bin
datadir=${XDG_DATA_HOME:-$HOME/.local/share}
confdir=${XDG_CONFIG_HOME:-$HOME/.config}

install -Dpm 0755 -t "${bindir}" bin/me3
install -Dpm 0644 -t "${datadir}/me3/windows-bin" bin/win64/me3-launcher.exe \
                                                  bin/win64/me3_mod_host.dll

install -Dpm 0644 -t "${datadir}/applications" dist/me3-launch.desktop
install -Dpm 0644 -t "${datadir}/mime/packages" dist/me3.xml
install -Dpm 0644 -t "${datadir}/icons/hicolor/128x128/apps" dist/me3.png

if [ ! -f "${confdir}/me3/me3.toml" ]; then
    mkdir -p "${confdir}/me3"
    cat >"${confdir}/me3/me3.toml" <<EOF
crash_reporting = false
windows_binaries_dir = "${datadir}/me3/windows-bin"
EOF
fi

# install example profiles
if [ ! -d "${confdir}/me3/profiles" ]; then
    install -Dpm 0644 -t "${confdir}/me3/profiles" ./*.me3
    mkdir "${confdir}/me3/profiles/eldenring-mods"
    mkdir "${confdir}/me3/profiles/nightreign-mods"
fi
