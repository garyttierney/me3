%global debug_package %{nil}

Name:       me3
Version:    0.6.1
Release:    1%{?dist}
Summary:    Modding framework for FROMSOFTWARE games

License:    Apache-2.0 OR MIT
URL:        https://me3.help

%if %{undefined branch} && %{undefined commit}
Source0:        https://github.com/garyttierney/me3/archive/v%{version}/%{name}-%{version}.tar.gz
%elif %{defined branch}
Source0:        https://github.com/garyttierney/me3/archive/refs/heads/%{branch}.tar.gz
%elif %{defined commit}
Source0:        https://github.com/garyttierney/me3/archive/%{commit}/%{name}-%{commit}.tar.gz
%endif

Source1: vendor.tar.gz

BuildRequires: rust-packaging
BuildRequires: rust-std-static-x86_64-pc-windows-gnu
BuildRequires: make
BuildRequires: gcc
BuildRequires: binutils
BuildRequires: mingw64-binutils
BuildRequires: mingw64-crt
BuildRequires: mingw64-gcc
BuildRequires: mingw64-libstdc++
BuildRequires: mingw64-cpp
BuildRequires: mingw64-headers
BuildRequires: mingw64-filesystem
BuildRequires: git
BuildRequires: cargo-rpm-macros >= 24
BuildRequires: rust >= 1.88.0

%description
me3 is a mod loader for FROMSOFTWARE games

%define cargo_patch_git_deps %{?nil:\\\
cat >> .cargo/config.toml <<EOF

[source."git+https://github.com/Hpmason/retour-rs"]
git = "https://github.com/Hpmason/retour-rs"
replace-with = "vendored-sources"

EOF}

%prep
%autosetup -a1 -p1

%build
%cargo_prep -v vendor
%cargo_patch_git_deps
%cargo_build -a -- --package me3-cli --target=x86_64-unknown-linux-gnu

# unsupported by mingw64 ld
%undefine _package_note_flags
%mingw64_env
%cargo_prep -v vendor
%cargo_patch_git_deps
%cargo_build -a -- --package me3-launcher --package me3-mod-host --target=x86_64-pc-windows-gnu

%cargo_license_summary
%{cargo_license} > LICENSE.dependencies
%cargo_vendor_manifest
sed -i -e '/https:\/\//d' cargo-vendor.txt

%install	
install -Dpm 0755 -t "%{buildroot}%{_bindir}" target/x86_64-unknown-linux-gnu/rpm/me3
install -Dpm 0755 -t "%{buildroot}%{_libdir}/me3/x86_64-windows" target/x86_64-pc-windows-gnu/rpm/me3-launcher.exe target/x86_64-pc-windows-gnu/rpm/me3_mod_host.dll
install -Dpm 0644 -t "%{buildroot}%{_datadir}/applications" distribution/linux/me3-launch.desktop
install -Dpm 0644 -t "%{buildroot}%{_datadir}/mime/packages" distribution/linux/me3.xml
install -Dpm 0644 -t "%{buildroot}%{_datadir}/icons/hicolor/128x128/apps" distribution/assets/me3.png

%post
update-desktop-database %{_datadir}/applications &> /dev/null || :
update-mime-database %{_datadir}/mime

%postun
update-desktop-database %{_datadir}/applications &> /dev/null || :
update-mime-database %{_datadir}/mime

%files
%license LICENSE.dependencies
%license cargo-vendor.txt
%license LICENSE-APACHE LICENSE-MIT
%doc README.md
%{_bindir}/me3
%{_libdir}/me3/
%{_datadir}/applications/me3-launch.desktop
%{_datadir}/mime/packages/me3.xml
%{_datadir}/icons/hicolor/128x128/apps/me3.png

%changelog
%autochangelog