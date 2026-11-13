#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The .deb and .rpm of one Linux build, for packages.qso-graph.net's apt and dnf repositories
# (qso-graph/qso-graph-packages), which sign them:
#
#   scripts/linux-packages.sh <qg-interop> <version> <linux-x86_64|linux-arm64> <out-dir>
#
# Writes qso-graph-interop-<version>-linux-{amd64|arm64}.deb and -linux-{x86_64|aarch64}.rpm to <out-dir>
# and prints their paths. They hold the static program, its README and license.
# A beta's version is X.Y.Z~beta.N inside
# the package, which both apt and dnf sort before X.Y.Z. Needs dpkg-deb and rpmbuild.
set -euo pipefail

exe=${1:?usage: linux-packages.sh <qg-interop> <version> <linux-x86_64|linux-arm64> <out-dir>}
version=${2:?version}
platform=${3:?platform}
out=${4:?out-dir}
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-beta\.[1-9][0-9]*)?$ ]] || { echo "Not a version: $version" >&2; exit 2; }
case "$platform" in
  linux-x86_64) deb_arch=amd64 rpm_arch=x86_64 ;;
  linux-arm64) deb_arch=arm64 rpm_arch=aarch64 ;;
  *) echo "Not a Linux platform: $platform" >&2; exit 2 ;;
esac
here=$(cd "$(dirname "$0")/.." && pwd)
pkg_version=${version/-beta./\~beta.}
name=qso-graph-interop
summary="Listen, decode, poll, and simulate amateur radio application protocols"
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(git -C "$here" log -1 --format=%ct)}
mkdir -p "$out"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# One tree for both: the static program and its documents.
root="$work/root"
install -D -m 0755 "$exe" "$root/usr/bin/qg-interop"
install -D -m 0644 "$here/crates/qso-graph-interop-cli/README.md" "$root/usr/share/doc/$name/README.md"
for f in LICENSE; do install -D -m 0644 "$here/$f" "$root/usr/share/doc/$name/$f"; done

# .deb
deb="$work/deb"
cp -a "$root" "$deb"
mkdir -m 0755 "$deb/DEBIAN"
cat > "$deb/DEBIAN/control" <<CONTROL
Package: $name
Version: $pkg_version
Architecture: $deb_arch
Maintainer: QSO Graph contributors <maintainers@qso-graph.io>
Homepage: https://github.com/qso-graph/qso-graph-interop
Section: hamradio
Priority: optional
Description: $summary
 qg-interop supports WSJT-X, N1MM Logger+, and fldigi for application
 integration and synthetic protocol traffic.
CONTROL
find "$deb" -exec touch -h -d "@$SOURCE_DATE_EPOCH" {} +
deb_file="$out/$name-$version-linux-$deb_arch.deb"
dpkg-deb --root-owner-group -Zxz --build "$deb" "$deb_file" >/dev/null

# .rpm
top="$work/rpmbuild"
mkdir -p "$top/SPECS"
cat > "$top/SPECS/$name.spec" <<SPEC
%global debug_package %{nil}
%global __strip /bin/true
%global _build_id_links none
Name: $name
Version: $pkg_version
Release: 1
Summary: $summary
AutoReqProv: no
License: GPL-3.0-or-later
URL: https://github.com/qso-graph/qso-graph-interop
%description
qg-interop supports WSJT-X, N1MM Logger+, and fldigi for application
integration and synthetic protocol traffic.
%install
cp -a $root/. %{buildroot}/
%files
/usr/bin/qg-interop
%doc /usr/share/doc/$name
SPEC
rpmbuild --quiet --define "_topdir $top" --define "_tmppath $work" --define "use_source_date_epoch_as_buildtime 1" \
  --define "clamp_mtime_to_source_date_epoch 1" --target "$rpm_arch" -bb "$top/SPECS/$name.spec" >/dev/null 2>&1 \
  || { rpmbuild --define "_topdir $top" --define "_tmppath $work" --target "$rpm_arch" -bb "$top/SPECS/$name.spec" >&2; exit 1; }
rpm_file="$out/$name-$version-linux-$rpm_arch.rpm"
cp "$top"/RPMS/"$rpm_arch"/*.rpm "$rpm_file"

# Each is the program as built, under the name and version given.
[ "$(dpkg-deb -f "$deb_file" Package) $(dpkg-deb -f "$deb_file" Version) $(dpkg-deb -f "$deb_file" Architecture)" = "$name $pkg_version $deb_arch" ] \
  || { echo "The .deb is not $name $pkg_version $deb_arch" >&2; exit 1; }
[ "$(rpm -qp --qf '%{NAME} %{VERSION} %{ARCH}' "$rpm_file" 2>/dev/null)" = "$name $pkg_version $rpm_arch" ] \
  || { echo "The .rpm is not $name $pkg_version $rpm_arch" >&2; exit 1; }
cmp <(dpkg-deb --fsys-tarfile "$deb_file" | tar -xO ./usr/bin/qg-interop) "$exe" \
  || { echo "The .deb's qg-interop is not the program built" >&2; exit 1; }
cmp <(rpm2cpio "$rpm_file" | cpio -i --quiet --to-stdout ./usr/bin/qg-interop) "$exe" \
  || { echo "The .rpm's qg-interop is not the program built" >&2; exit 1; }
echo "$deb_file"
echo "$rpm_file"
