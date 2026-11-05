#!/usr/bin/env bash
# Fetches the verification data into data/ (gitignored), each pinned by
# version and SHA-256. Run from anywhere; safe to rerun (a dataset already
# present at its pinned version is kept).
#
#   data/reasonable/   reasonable at commit fecee2b (the commit the parity
#                      research read): example_models/ (Brick 1.1 as
#                      ontologies/Brick.n3, owl.n3, rdfs.ttl, small1.n3,
#                      soda_hall.n3), benches/python/buildings/*.ttl, and
#                      lib/src/tests.rs (its unit-test inputs)
#   data/brick/        Brick v1.5.0, Brick.ttl (the release asset)
#   data/w3c/owl2/     the OWL 2 conformance tests in the RL profile
#                      (profile-RL.rdf), the owl.semanticweb.org export as the
#                      Internet Archive kept it on 2016-07-03 (the site is gone)
#   data/w3c/rdf-tests/ the RDF 1.1 Semantics entailment tests
#                      (rdf/rdf11/rdf-mt/) from w3c/rdf-tests at 202a74e
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
data="$root/data"
mkdir -p "$data"

sha256() {
	if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# fetch URL SHA256 FILE: downloads URL to FILE and checks its SHA-256.
fetch() {
	curl -sSfL --retry 3 -o "$3" "$1"
	local got
	got="$(sha256 "$3")"
	if [ "$got" != "$2" ]; then
		echo "fetch-data: checksum mismatch for $1: expected $2, got $got" >&2
		rm -f "$3"
		exit 1
	fi
}

needs() {
	[ "$(cat "$data/$1/VERSION" 2>/dev/null)" != "$2" ]
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

reasonable_commit="fecee2b6d3713f97c86aad56e099674dcacea75b"
if needs reasonable "$reasonable_commit"; then
	echo "fetch-data: reasonable at ${reasonable_commit:0:7}"
	mkdir "$work/reasonable"
	fetch "https://codeload.github.com/gtfierro/reasonable/tar.gz/$reasonable_commit" \
		"4a83d00059264a7c6a8a779f56ee7d0b0bb644e4ae535224e707b359e7a9b61a" "$work/reasonable.tar.gz"
	tar -xzf "$work/reasonable.tar.gz" -C "$work/reasonable"
	src="$work/reasonable/reasonable-$reasonable_commit"
	rm -rf "$data/reasonable"
	mkdir -p "$data/reasonable/benches/python" "$data/reasonable/lib/src"
	mv "$src/example_models" "$data/reasonable/example_models"
	mv "$src/benches/python/buildings" "$data/reasonable/benches/python/buildings"
	mv "$src/lib/src/tests.rs" "$data/reasonable/lib/src/tests.rs"
	mv "$src/LICENSE" "$data/reasonable/LICENSE" 2>/dev/null || true
	echo "$reasonable_commit" >"$data/reasonable/VERSION"
fi

if needs brick v1.5.0; then
	echo "fetch-data: Brick v1.5.0"
	rm -rf "$data/brick"
	mkdir -p "$data/brick"
	fetch "https://github.com/BrickSchema/Brick/releases/download/v1.5.0/Brick.ttl" \
		"f665c93e5426a66257a8f34173ceb87a4674b9b15286e0390a5a61cfb905f8a9" "$data/brick/Brick.ttl"
	echo v1.5.0 >"$data/brick/VERSION"
fi

if needs w3c/owl2 20160703034201; then
	echo "fetch-data: OWL 2 RL conformance tests"
	rm -rf "$data/w3c/owl2"
	mkdir -p "$data/w3c/owl2"
	fetch "http://web.archive.org/web/20160703034201id_/http://owl.semanticweb.org/exports/profile-RL.rdf" \
		"941d02a52cb96029d5b3500143cf7f006e371326f8798874a8d0d20966555ceb" "$data/w3c/owl2/profile-RL.rdf"
	echo 20160703034201 >"$data/w3c/owl2/VERSION"
fi

rdf_tests_commit="202a74eb81f47bcad13bbafdd37eba69c361c1c2"
if needs w3c/rdf-tests "$rdf_tests_commit"; then
	echo "fetch-data: RDF 1.1 Semantics tests at ${rdf_tests_commit:0:7}"
	mkdir "$work/rdf-tests"
	fetch "https://codeload.github.com/w3c/rdf-tests/tar.gz/$rdf_tests_commit" \
		"0daf336f0df4cbbadba11cf675d6eb6b07b06ef71143c899a14075d8e56c578a" "$work/rdf-tests.tar.gz"
	tar -xzf "$work/rdf-tests.tar.gz" -C "$work/rdf-tests"
	rm -rf "$data/w3c/rdf-tests"
	mkdir -p "$data/w3c/rdf-tests"
	mv "$work/rdf-tests/rdf-tests-$rdf_tests_commit/rdf/rdf11/rdf-mt" "$data/w3c/rdf-tests/rdf-mt"
	echo "$rdf_tests_commit" >"$data/w3c/rdf-tests/VERSION"
fi

echo "fetch-data: done ($data)"
