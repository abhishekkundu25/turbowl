#!/usr/bin/env bash
# Fetches the oracle binaries and the bench's competitors into tools/
# (gitignored), each pinned by version and SHA-256. Run from anywhere; safe to
# rerun (a tool already present at its pinned version is kept; one left half
# installed by a crash has no VERSION file yet, so it is fetched again).
#
#   tools/nemo/nmo              Nemo 0.10.1 (the Nemo oracle; a bench competitor)
#   tools/jre/bin/java          Temurin JRE 21.0.13+9 (runs HermiT, UBA and VLog)
#   tools/hermit/HermiT.jar     HermiT 1.3.8 (the DL reasoner oracle)
#   tools/souffle/bin/souffle   Souffle 2.5, Homebrew's bottle (a bench competitor;
#                               macOS arm64 only)
#   tools/vlog/rulewerk-client-0.9.0.jar
#                               VLog 1.3.7 inside Rulewerk 0.9.0's client, run on
#                               the JRE (a bench competitor)
#
# Souffle's compiled mode needs a C++ compiler: the bottle uses /usr/bin/c++
# and the macOS SDK of Xcode's command-line tools (xcode-select --install).
# GraphDB is not fetched: GraphDB Free needs a licence sent by e-mail.
#
# Platforms: macOS arm64 and Linux x86_64. Elsewhere the platform-specific
# tools are not fetched and their oracles report `skipped`.
#
# Not fetched: Konclude. Its newest release (v0.7.0-1138, 2021) has no arm64
# macOS build (the x86_64 one needs Rosetta), and it reads only OWL/XML and
# functional syntax. HermiT reads Turtle and RDF/XML through its bundled OWL API.
#
# The OWL-RL oracle is not here: it is the uv project in oracles/owlrl/
# (`uv sync --project oracles/owlrl --frozen`), pinned by its uv.lock.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
tools="$root/tools"
mkdir -p "$tools"

case "$(uname -s)-$(uname -m)" in
Darwin-arm64)
	nemo_asset="nemo_v0.10.1_aarch64-apple-darwin"
	nemo_sha="4b1e9a9f46a315ab8c590fb0124cb470c33b5293b9f3a61dd53225b31ed79f16"
	jre_asset="OpenJDK21U-jre_aarch64_mac_hotspot_21.0.13_9.tar.gz"
	jre_sha="225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a"
	jre_home="jdk-21.0.13+9-jre/Contents/Home"
	# Homebrew's bottle for macOS 27 (arm64_golden_gate), from Formula/s/souffle.rb.
	souffle_bottle="ec13234dafe4585d95b3f4fab7b1af209342581d19a7d94fe32b26566f924a1f"
	;;
Linux-x86_64)
	nemo_asset="nemo_v0.10.1_x86_64-unknown-linux-gnu"
	nemo_sha="2f659c3803fa7b9a4ce1c9449803038e04562d74d83e5cd325cca43b4fcdd6f9"
	jre_asset="OpenJDK21U-jre_x64_linux_hotspot_21.0.13_9.tar.gz"
	jre_sha="a535a1df9c6284d9618872984e026ae0352d1f7e9e0605425c0e6114f47351dd"
	jre_home="jdk-21.0.13+9-jre"
	souffle_bottle=""
	;;
*)
	nemo_asset=""
	jre_asset=""
	souffle_bottle=""
	echo "fetch-tools: no pinned Nemo or JRE for $(uname -s)-$(uname -m); those oracles will be skipped" >&2
	;;
esac

sha256() {
	if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# fetch URL SHA256 FILE [CURL_ARGS...]: downloads URL to FILE and checks its SHA-256.
fetch() {
	curl -sSfL --retry 3 "${@:4}" -o "$3" "$1"
	local got
	got="$(sha256 "$3")"
	if [ "$got" != "$2" ]; then
		echo "fetch-tools: checksum mismatch for $1: expected $2, got $got" >&2
		rm -f "$3"
		exit 1
	fi
}

# install NAME VERSION: true when tools/NAME is missing or at another version.
needs() {
	[ "$(cat "$tools/$1/VERSION" 2>/dev/null)" != "$2" ]
}

# Each archive is unpacked into its own new, empty directory, then moved.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [ -n "$nemo_asset" ] && needs nemo 0.10.1; then
	echo "fetch-tools: Nemo 0.10.1"
	mkdir "$work/nemo"
	fetch "https://github.com/knowsys/nemo/releases/download/v0.10.1/$nemo_asset.tar.gz" "$nemo_sha" "$work/nemo.tar.gz"
	tar -xzf "$work/nemo.tar.gz" -C "$work/nemo"
	rm -rf "$tools/nemo"
	mv "$work/nemo/$nemo_asset" "$tools/nemo"
	echo 0.10.1 >"$tools/nemo/VERSION"
fi

if [ -n "$jre_asset" ] && needs jre 21.0.13+9; then
	echo "fetch-tools: Temurin JRE 21.0.13+9"
	mkdir "$work/jre"
	fetch "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.13%2B9/$jre_asset" "$jre_sha" "$work/jre.tar.gz"
	tar -xzf "$work/jre.tar.gz" -C "$work/jre"
	rm -rf "$tools/jre"
	mv "$work/jre/$jre_home" "$tools/jre"
	echo 21.0.13+9 >"$tools/jre/VERSION"
fi

if needs hermit 1.3.8; then
	echo "fetch-tools: HermiT 1.3.8"
	mkdir "$work/hermit"
	fetch "https://www.cs.ox.ac.uk/isg/tools/HermiT/download/1.3.8/HermiT.zip" \
		"5ce30a35a1f02e933c262dfc976dd9e4e7bfe6c3e587c7bdaee6f57b97c814d0" "$work/hermit.zip"
	unzip -q "$work/hermit.zip" HermiT.jar readme.txt -d "$work/hermit"
	rm -rf "$tools/hermit"
	mv "$work/hermit" "$tools/hermit"
	echo 1.3.8 >"$tools/hermit/VERSION"
fi

if [ -n "$souffle_bottle" ] && needs souffle 2.5; then
	echo "fetch-tools: Souffle 2.5 (Homebrew bottle)"
	mkdir "$work/souffle"
	# ghcr.io serves Homebrew's bottles to anyone with this anonymous token.
	fetch "https://ghcr.io/v2/homebrew/core/souffle/blobs/sha256:$souffle_bottle" "$souffle_bottle" \
		"$work/souffle.tar.gz" -H "Authorization: Bearer QQ=="
	tar -xzf "$work/souffle.tar.gz" -C "$work/souffle"
	# Homebrew rewrites the cellar path when it installs a bottle; the compile
	# script is the one file that names it. The script also has to be told
	# where the headers are: the bottle holds them twice, and only the copy
	# under include/souffle/souffle is consistent on its own.
	dest="$(cd "$tools" && pwd -P)/souffle"
	sed -i.orig \
		-e "s#@@HOMEBREW_CELLAR@@/souffle/2.5#$dest#" \
		-e "s#^  \"includes\": \"#  \"includes\": \"-I$dest/include/souffle #" \
		"$work/souffle/souffle/2.5/bin/souffle-compile.py"
	rm "$work/souffle/souffle/2.5/bin/souffle-compile.py.orig"
	rm -rf "$tools/souffle"
	mv "$work/souffle/souffle/2.5" "$tools/souffle"
	echo 2.5 >"$tools/souffle/VERSION"
	xcode-select -p >/dev/null 2>&1 ||
		echo "fetch-tools: no Xcode command-line tools: Souffle cannot compile programs (xcode-select --install)" >&2
fi

if [ -n "$jre_asset" ] && needs vlog 1.3.7; then
	echo "fetch-tools: VLog 1.3.7 (Rulewerk 0.9.0's client)"
	mkdir "$work/vlog"
	fetch "https://github.com/knowsys/rulewerk/releases/download/v0.9.0/rulewerk-client-0.9.0.jar" \
		"785307e4f0e42c30bd84cf9a8e7ff73319d23e95c92f314b4d3f7b63c82751f3" "$work/vlog/rulewerk-client-0.9.0.jar"
	rm -rf "$tools/vlog"
	mv "$work/vlog" "$tools/vlog"
	echo 1.3.7 >"$tools/vlog/VERSION"
fi

echo "fetch-tools: done ($tools)"
