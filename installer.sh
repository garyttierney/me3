#!/bin/sh
# shellcheck shell=dash
# shellcheck disable=SC2039  # local is non-POSIX
set -u

INSTALLER_VERSION=v0.7.0

need_cmd() {
    if ! check_cmd "$1"; then
        err "need '$1' (command not found)"
    fi
}

check_cmd() {
    command -v "$1" >/dev/null 2>&1
    return $?
}

is_zsh() {
    [ -n "${ZSH_VERSION-}" ]
}

downloader() {
    # zsh does not split words by default, Required for curl retry arguments below.
    is_zsh && setopt local_options shwordsplit

    local _dld
    local _ciphersuites
    local _err
    local _status
    local _retry
    if check_cmd curl; then
        _dld=curl
    elif check_cmd wget; then
        _dld=wget
    else
        _dld='curl or wget' # to be used in error message of need_cmd
    fi

    if [ "$1" = --check ]; then
        need_cmd "$_dld"
    elif [ "$_dld" = curl ]; then
        check_curl_for_retry_support
        _retry="$RETVAL"
        get_ciphersuites_for_curl
        _ciphersuites="$RETVAL"
        if [ -n "$_ciphersuites" ]; then
            # shellcheck disable=SC2086
            _err=$(curl $_retry --proto '=https' --tlsv1.2 --ciphers "$_ciphersuites" --silent --show-error --fail --location "$1" --output "$2" 2>&1)
            _status=$?
        else
            warn "Not enforcing strong cipher suites for TLS, this is potentially less secure"
            if ! check_help_for "$3" curl --proto --tlsv1.2; then
                warn "Not enforcing TLS v1.2, this is potentially less secure"
                # shellcheck disable=SC2086
                _err=$(curl $_retry --silent --show-error --fail --location "$1" --output "$2" 2>&1)
                _status=$?
            else
                # shellcheck disable=SC2086
                _err=$(curl $_retry --proto '=https' --tlsv1.2 --silent --show-error --fail --location "$1" --output "$2" 2>&1)
                _status=$?
            fi
        fi
        if [ -n "$_err" ]; then
            warn "$_err"
            if echo "$_err" | grep -q 404$; then
                err "installer for platform '$3' not found, this may be unsupported"
                exit 1
            fi
        fi
        return $_status
    elif [ "$_dld" = wget ]; then
        if [ "$(wget -V 2>&1 | head -2 | tail -1 | cut -f1 -d" ")" = "BusyBox" ]; then
            warn "using the BusyBox version of wget.  Not enforcing strong cipher suites for TLS or TLS v1.2, this is potentially less secure"
            _err=$(wget "$1" -O "$2" 2>&1)
            _status=$?
        else
            get_ciphersuites_for_wget
            _ciphersuites="$RETVAL"
            if [ -n "$_ciphersuites" ]; then
                _err=$(wget --https-only --secure-protocol=TLSv1_2 --ciphers "$_ciphersuites" "$1" -O "$2" 2>&1)
                _status=$?
            else
                warn "Not enforcing strong cipher suites for TLS, this is potentially less secure"
                if ! check_help_for "$3" wget --https-only --secure-protocol; then
                    warn "Not enforcing TLS v1.2, this is potentially less secure"
                    _err=$(wget "$1" -O "$2" 2>&1)
                    _status=$?
                else
                    _err=$(wget --https-only --secure-protocol=TLSv1_2 "$1" -O "$2" 2>&1)
                    _status=$?
                fi
            fi
        fi
        if [ -n "$_err" ]; then
            warn "$_err"
            if echo "$_err" | grep -q ' 404 Not Found$'; then
                err "installer for platform '$3' not found, this may be unsupported"
                exit 1
            fi
        fi
        return $_status
    else
        err "Unknown downloader" # should not reach here
        exit 1
    fi
}

# Run a command that should never fail. If the command fails execution
# will immediately terminate with an error showing the failing
# command.
ensure() {
    if ! "$@"; then
        err "command failed: $*"
        exit 1
    fi
}

say() {
    if [ "$ME3_QUIET" = "no" ]; then
        __print 'info' "$1" >&2
    fi
}

warn() {
    __print 'warn' "$1" >&2
}

# NOTE: you are required to exit yourself
# we don't do it here because of multiline errors
err() {
    __print 'error' "$1" >&2
}

__print() {
    if $_ansi_escapes_are_valid; then
        printf '\33[1m%s:\33[0m %s\n' "$1" "$2" >&2
    else
        printf '%s: %s\n' "$1" "$2" >&2
    fi
}

check_help_for() {
    local _arch
    local _cmd
    local _arg
    _arch="$1"
    shift
    _cmd="$1"
    shift

    local _category
    if "$_cmd" --help | grep -q '"--help all"'; then
        _category="all"
    else
        _category=""
    fi

    case "$_arch" in

    *darwin*)
        if check_cmd sw_vers; then
            local _os_version
            local _os_major
            _os_version=$(sw_vers -productVersion)
            _os_major=$(echo "$_os_version" | cut -d. -f1)
            case $_os_major in
            10)
                # If we're running on macOS, older than 10.13, then we always
                # fail to find these options to force fallback
                if [ "$(echo "$_os_version" | cut -d. -f2)" -lt 13 ]; then
                    # Older than 10.13
                    warn "Detected macOS platform older than 10.13"
                    return 1
                fi
                ;;
            *)
                if ! { [ "$_os_major" -eq "$_os_major" ] 2>/dev/null && [ "$_os_major" -ge 11 ]; }; then
                    # Unknown product version, warn and continue
                    warn "Detected unknown macOS major version: $_os_version"
                    warn "TLS capabilities detection may fail"
                fi
                ;; # We assume that macOS v11+ will always be okay.
            esac
        fi
        ;;

    esac

    for _arg in "$@"; do
        if ! "$_cmd" --help "$_category" | grep -q -- "$_arg"; then
            return 1
        fi
    done

    true # not strictly needed
}

# Check if curl supports the --retry flag, then pass it to the curl invocation.
check_curl_for_retry_support() {
    local _retry_supported=""
    # "unspecified" is for arch, allows for possibility old OS using macports, homebrew, etc.
    if check_help_for "notspecified" "curl" "--retry"; then
        _retry_supported="--retry 3"
        if check_help_for "notspecified" "curl" "--continue-at"; then
            # "-C -" tells curl to automatically find where to resume the download when retrying.
            _retry_supported="--retry 3 -C -"
        fi
    fi

    RETVAL="$_retry_supported"
}

# Return cipher suite string specified by user, otherwise return strong TLS 1.2-1.3 cipher suites
# if support by local tools is detected. Detection currently supports these curl backends:
# GnuTLS and OpenSSL (possibly also LibreSSL and BoringSSL). Return value can be empty.
get_ciphersuites_for_curl() {
    if [ -n "${ME3_TLS_CIPHERSUITES-}" ]; then
        # user specified custom cipher suites, assume they know what they're doing
        RETVAL="$ME3_TLS_CIPHERSUITES"
        return
    fi

    local _openssl_syntax="no"
    local _gnutls_syntax="no"
    local _backend_supported="yes"
    if curl -V | grep -q ' OpenSSL/'; then
        _openssl_syntax="yes"
    elif curl -V | grep -iq ' LibreSSL/'; then
        _openssl_syntax="yes"
    elif curl -V | grep -iq ' BoringSSL/'; then
        _openssl_syntax="yes"
    elif curl -V | grep -iq ' GnuTLS/'; then
        _gnutls_syntax="yes"
    else
        _backend_supported="no"
    fi

    local _args_supported="no"
    if [ "$_backend_supported" = "yes" ]; then
        # "unspecified" is for arch, allows for possibility old OS using macports, homebrew, etc.
        if check_help_for "notspecified" "curl" "--tlsv1.2" "--ciphers" "--proto"; then
            _args_supported="yes"
        fi
    fi

    local _cs=""
    if [ "$_args_supported" = "yes" ]; then
        if [ "$_openssl_syntax" = "yes" ]; then
            _cs=$(get_strong_ciphersuites_for "openssl")
        elif [ "$_gnutls_syntax" = "yes" ]; then
            _cs=$(get_strong_ciphersuites_for "gnutls")
        fi
    fi

    RETVAL="$_cs"
}

# Return cipher suite string specified by user, otherwise return strong TLS 1.2-1.3 cipher suites
# if support by local tools is detected. Detection currently supports these wget backends:
# GnuTLS and OpenSSL (possibly also LibreSSL and BoringSSL). Return value can be empty.
get_ciphersuites_for_wget() {
    if [ -n "${ME3_TLS_CIPHERSUITES-}" ]; then
        # user specified custom cipher suites, assume they know what they're doing
        RETVAL="$ME3_TLS_CIPHERSUITES"
        return
    fi

    local _cs=""
    if wget -V | grep -q '\-DHAVE_LIBSSL'; then
        # "unspecified" is for arch, allows for possibility old OS using macports, homebrew, etc.
        if check_help_for "notspecified" "wget" "TLSv1_2" "--ciphers" "--https-only" "--secure-protocol"; then
            _cs=$(get_strong_ciphersuites_for "openssl")
        fi
    elif wget -V | grep -q '\-DHAVE_LIBGNUTLS'; then
        # "unspecified" is for arch, allows for possibility old OS using macports, homebrew, etc.
        if check_help_for "notspecified" "wget" "TLSv1_2" "--ciphers" "--https-only" "--secure-protocol"; then
            _cs=$(get_strong_ciphersuites_for "gnutls")
        fi
    fi

    RETVAL="$_cs"
}

# Return strong TLS 1.2-1.3 cipher suites in OpenSSL or GnuTLS syntax. TLS 1.2
# excludes non-ECDHE and non-AEAD cipher suites. DHE is excluded due to bad
# DH params often found on servers (see RFC 7919). Sequence matches or is
# similar to Firefox 68 ESR with weak cipher suites disabled via about:config.
# $1 must be openssl or gnutls.
get_strong_ciphersuites_for() {
    if [ "$1" = "openssl" ]; then
        # OpenSSL is forgiving of unknown values, no problems with TLS 1.3 values on versions that don't support it yet.
        echo "TLS_AES_128_GCM_SHA256:TLS_CHACHA20_POLY1305_SHA256:TLS_AES_256_GCM_SHA384:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384"
    elif [ "$1" = "gnutls" ]; then
        # GnuTLS isn't forgiving of unknown values, so this may require a GnuTLS version that supports TLS 1.3 even if wget doesn't.
        # Begin with SECURE128 (and higher) then remove/add to build cipher suites. Produces same 9 cipher suites as OpenSSL but in slightly different order.
        echo "SECURE128:-VERS-SSL3.0:-VERS-TLS1.0:-VERS-TLS1.1:-VERS-DTLS-ALL:-CIPHER-ALL:-MAC-ALL:-KX-ALL:+AEAD:+ECDHE-ECDSA:+ECDHE-RSA:+AES-128-GCM:+CHACHA20-POLY1305:+AES-256-GCM"
    fi
}

main() {
    if [ "${ME3_QUIET+set}" != set ]; then
        ME3_QUIET=no
    fi

    downloader --check
    need_cmd mktemp
    need_cmd chmod
    need_cmd mkdir
    need_cmd rm
    need_cmd rmdir
    need_cmd tar

    me3_version=${VERSION:-"$INSTALLER_VERSION"}

    local _ansi_escapes_are_valid=false
    if [ -t 2 ]; then
        if [ "${TERM+set}" = 'set' ]; then
            case "$TERM" in
            xterm* | rxvt* | urxvt* | linux* | vt*)
                _ansi_escapes_are_valid=true
                ;;
            esac
        fi
    fi

    local dir
    if ! dir="$(ensure mktemp -d)"; then
        # Because the previous command ran in a subshell, we must manually
        # propagate exit status.
        exit 1
    fi

    local localdistfile="$1"
    local distfile="me3-linux-amd64.tar.gz"
    local distfilepath

    if [ -f "$localdistfile" ]; then
        distfilepath="$localdistfile"
        say "using local tarball $localdistfile"
    else
        distfilepath="$dir/$distfile"
        say "downloading $distfile"
        downloader "https://github.com/garyttierney/me3/releases/download/$me3_version/me3-linux-amd64.tar.gz" "$distfilepath"
    fi

    local distdir="$dir/dist"
    ensure mkdir -p "$distdir"
    ensure tar vxf "$distfilepath" -C "$distdir"
    {
        ensure cd "$distdir"
        ensure "$distdir/install-user.sh"
    }

    if ! check_cmd me3; then
        say "me3 is not available on PATH, make sure to update your shell profile\nPATH=\"\$PATH:$HOME/.local/bin\""
    fi

    datadir=${XDG_DATA_HOME:-$HOME/.local/share}

    check_cmd update-mime-database && update-mime-database "$datadir/mime"
    check_cmd update-desktop-database && update-desktop-database "$datadir/applications"

    ensure rm -rf "$dir"
}

set +u
main "$@" || exit 1
