#!/usr/bin/env bash
# Fetches the verification and benchmark data into data/ (gitignored), each
# file pinned by URL and SHA-256. Run from anywhere; safe to rerun: a dataset
# already present at its pinned version is kept, and a benchmark dataset whose
# download or extraction was interrupted resumes where it stopped.
#
#   scripts/fetch-data.sh                the default set below
#   scripts/fetch-data.sh --large        also the large Oxford datasets: 5.96 GB
#                                        of zips, 113 GB once unpacked
#   scripts/fetch-data.sh --lubm 10,100  also LUBM data from UBA for these
#                                        numbers of universities (needs tools/jre,
#                                        from scripts/fetch-tools.sh)
#
# Verification data:
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
#
# Benchmark datasets (crates/bench/README.md; docs/research/competitors-2026-11.md
# §8). Oxford's: the data of the RDFox papers (AAAI 2014 and 2015) with their
# programs, each as OWL (.owl, RDF/XML) and as Datalog (.dlog, for RDFox):
#   data/oxford/claros/   Claros.ttl (84 lines' IRIs repaired: repair_iris),
#                         Claros_{L,LE,U}.owl and .dlog
#   data/oxford/uobm/     UOBM-010.ttl, UOBM_{L,U}.owl and .dlog
#   data/oxford/lubm/     LUBM_{L,LE,U}.owl and .dlog (the data: --large or --lubm)
#   data/oxford/opencyc/  OpenCyc.ttl (its own OWL axioms) and OpenCyc-eq.dlog
#   data/owl2bench/       OWL2RL-1.owl, OWL2RL-2.owl, OWL2RL-5.owl: OWL2Bench's
#                         OWL 2 RL ontologies with data for 1, 2 and 5
#                         universities (Zenodo record 4764368)
#   data/uba/             UBA 1.7, the LUBM data generator (classes/, src/)
# With --large (each pinned by the CRC-32 its zip records for the data file,
# as no SHA-256 of these zips has been taken; unpacked: 23.9, 45.6, 15.9 and
# 27.4 GB, and each zip stays until its data file is out):
#   data/oxford/lubm-01k/ LUBM-01K.ttl       data/oxford/uobm-01k/ UOBM-01K.ttl
#   data/oxford/dbpedia/  DBPedia.ttl, DBpedia_{L,LE}.owl and .dlog
#   data/oxford/uniprot/  UniProt.ttl and UniProt-eq.dlog (no OWL ontology ships)
# With --lubm N:
#   data/uba/lubm-N.owl   UBA's data for N universities (-index 0 -seed 0, the
#                         options the LUBM page gives for the papers' data),
#                         every department's file merged into one
#
# Not fetched: the AIJ 2019 programs (ChEMBL, Reactome, UniProt, UOBM) and its
# DBpedia subset, whose links return 404.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
data="$root/data"
mkdir -p "$data"

large=false
lubm_sizes=()
while [ $# -gt 0 ]; do
	case "$1" in
	--large) large=true ;;
	--lubm)
		[ $# -ge 2 ] || { echo "fetch-data: --lubm needs a list of sizes, such as 10,100" >&2; exit 64; }
		IFS=, read -ra lubm_sizes <<<"$2"
		for n in ${lubm_sizes[@]+"${lubm_sizes[@]}"}; do
			[[ "$n" =~ ^[1-9][0-9]*$ ]] || { echo "fetch-data: --lubm: not a positive number: $n" >&2; exit 64; }
		done
		shift
		;;
	*) echo "usage: scripts/fetch-data.sh [--large] [--lubm N,...]" >&2; exit 64 ;;
	esac
	shift
done

sha256() {
	if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# fetch URL SHA256 FILE: downloads URL to FILE and checks its SHA-256. FILE
# appears only once checked, so one that exists is kept; an interrupted
# download resumes from FILE.part.
fetch() {
	[ -f "$3" ] && return
	curl -sSfL --retry 3 -C - -o "$3.part" "$1"
	local got
	got="$(sha256 "$3.part")"
	if [ "$got" != "$2" ]; then
		echo "fetch-data: checksum mismatch for $1: expected $2, got $got" >&2
		rm -f "$3.part"
		exit 1
	fi
	mv "$3.part" "$3"
}

needs() {
	[ "$(cat "$data/$1/VERSION" 2>/dev/null)" != "$2" ]
}

# begin DIR VERSION: true when data/DIR is not complete at VERSION. A DIR at
# another version is removed; one without a VERSION file is a run that
# stopped partway, and is filled from where it stopped. `finish` writes the
# VERSION file last.
begin() {
	local have
	have="$(cat "$data/$1/VERSION" 2>/dev/null || true)"
	[ "$have" != "$2" ] || return 1
	[ -z "$have" ] || rm -rf "${data:?}/$1"
	mkdir -p "$data/$1"
	echo "fetch-data: $1 ($2)"
}

finish() {
	echo "$2" >"$data/$1/VERSION"
}

# file DIR URL SHA256: the file at URL, into data/DIR under its own name.
file() {
	fetch "$2" "$3" "$data/$1/${2##*/}"
}

# unzip_one DIR URL PIN MEMBER [FILTER]: MEMBER of the zip at URL, into
# data/DIR, passed through the command FILTER if one is given. PIN is the
# zip's SHA-256, or crc32:HEX for a zip never downloaded to take one: the
# CRC-32 its central directory records for MEMBER (read with a range
# request), which the data is then tested against. The zip is kept only
# until MEMBER is in place.
unzip_one() {
	local dir="$data/$1" zip="$data/$1/${2##*/}" crc
	[ -f "$dir/$4" ] && return
	case "$3" in
	crc32:*)
		if [ ! -f "$zip" ]; then
			curl -sSfL --retry 3 -C - -o "$zip.part" "$2"
			mv "$zip.part" "$zip"
		fi
		crc="$(unzip -v "$zip" "$4" | awk -v m="$4" '$8 == m { print $7 }')"
		if [ "crc32:$crc" != "$3" ] || ! unzip -tq "$zip" "$4" >/dev/null; then
			echo "fetch-data: $2: $4 does not match CRC-32 ${3#crc32:} (recorded: ${crc:-none})" >&2
			rm -f "$zip"
			exit 1
		fi
		;;
	*) fetch "$2" "$3" "$zip" ;;
	esac
	rm -rf "$dir/.unzip"
	if [ -n "${5:-}" ]; then
		mkdir "$dir/.unzip"
		unzip -p "$zip" "$4" | "$5" >"$dir/.unzip/$4"
	else
		unzip -q "$zip" "$4" -d "$dir/.unzip"
	fi
	mv "$dir/.unzip/$4" "$dir/$4"
	rm -rf "$dir/.unzip" "$zip"
}

# repair_iris: N-Triples on stdin to stdout, with what an IRI may not hold
# percent-encoded in the IRI terms only: a backslash (%5C), a backtick (%60)
# and a % that starts no percent-encoding (%25). Literals are left alone.
# Claros.ttl has 84 such lines (23 IRIs, none of whose encoded forms it
# already holds, so no two IRIs merge); a strict parser rejects them, so
# without this no OWL 2 RL reasoner that checks IRIs could load it.
repair_iris() {
	LC_ALL=C awk '
		function iri(t,   out, i, c) {
			out = ""
			for (i = 1; i <= length(t); i++) {
				c = substr(t, i, 1)
				if (c == "\\") c = "%5C"
				else if (c == "`") c = "%60"
				else if (c == "%" && substr(t, i + 1, 2) !~ /^[0-9A-Fa-f][0-9A-Fa-f]$/) c = "%25"
				out = out c
			}
			return out
		}
		!/[\\`%]/ { print; next }
		{
			line = $0
			out = ""
			for (k = 0; k < 3; k++) {
				if (substr(line, 1, 1) == "<") {
					e = index(line, ">")
					out = out iri(substr(line, 1, e))
				} else if (substr(line, 1, 2) == "_:") {
					e = index(line, " ") - 1
					out = out substr(line, 1, e)
				} else break
				line = substr(line, e + 1)
				match(line, /^[ \t]*/)
				out = out substr(line, 1, RLENGTH)
				line = substr(line, RLENGTH + 1)
			}
			print out line
		}'
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

# Oxford's data and programs. The zips hold the data; the programs are files
# of the evaluation pages (cs.ox.ac.uk/isg/tools/RDFox/2014/AAAI/ and
# 2015/AAAI/Equality/).
zips14="https://krr-nas.cs.ox.ac.uk/2014/AAAI/RDFox"
zips15="https://krr-nas.cs.ox.ac.uk/2015/AAAI/RDFox/Equality"
in14="https://www.cs.ox.ac.uk/isg/tools/RDFox/2014/AAAI/input"
in15="https://www.cs.ox.ac.uk/isg/tools/RDFox/2015/AAAI/Equality/TestCases"

if begin oxford/claros aaai-2014+iris; then
	unzip_one oxford/claros "$zips14/Claros.zip" 635de5c1a5635264c0e3b6b8742be133badc5ece0b9330e56b9c8732ec6f9750 Claros.ttl repair_iris
	file oxford/claros "$in14/Claros/owl/Claros_L.owl" bb9ce5db918e5e37905f9b9bc5301baafe9f99361a90f5d9f3fe1c2cce9ce6ba
	file oxford/claros "$in14/Claros/owl/Claros_LE.owl" 9119c0aed73fe235bf3139f1f06ea7723c882261f2c5ba9d33f0e206a897a054
	file oxford/claros "$in14/Claros/owl/Claros_U.owl" bb8c703e0c453066889ae974a72b0eb7d325cef607f7fdb0a33e9f89dbf9b4ad
	file oxford/claros "$in14/Claros/dlog/Claros_L.dlog" 929ee74c6b6730a351ebd9d3cc6e04c5a9cad72be04ab9dbdd789ac0fd081653
	file oxford/claros "$in14/Claros/dlog/Claros_LE.dlog" d8292471ced8919a9a2d6923cd8c3d71c49f72d1dafb43629b8cb472a588b27a
	file oxford/claros "$in14/Claros/dlog/Claros_U.dlog" 7a0834804c18ad92d167a84208af5edd333e3382072e3f00d06eca52d792a6cb
	finish oxford/claros aaai-2014+iris
fi

if begin oxford/uobm aaai-2014; then
	unzip_one oxford/uobm "$zips14/UOBM-010.zip" a6c78646e60e0c96dafca117ac0fa5218317a43dd29ace0d281769251d8c378a UOBM-010.ttl
	file oxford/uobm "$in14/UOBM/owl/UOBM_L.owl" ea362d44ee1004311a49073d77df917b2885fd4fa26ef0c39070aeab8f415621
	file oxford/uobm "$in14/UOBM/owl/UOBM_U.owl" 41ed5c6e0cc85a9468baf8c19f998cfd8804245d8f470ed74ef361077c914954
	file oxford/uobm "$in14/UOBM/dlog/UOBM_L.dlog" 67d62537ae09c53361e2898f35bc058cd4039ec11c972d83d628613b4007b17b
	file oxford/uobm "$in14/UOBM/dlog/UOBM_U.dlog" 7f27790ce541815c368b3778d476daf88692ea72d1621a43842a407f2f0e4202
	finish oxford/uobm aaai-2014
fi

if begin oxford/lubm aaai-2014; then
	file oxford/lubm "$in14/LUBM/owl/LUBM_L.owl" d8b4aa6658f36948c796c1db61be8fdafa4146aed03bff357d4bbf8439dfa841
	file oxford/lubm "$in14/LUBM/owl/LUBM_LE.owl" 538915c54b466f0d4d2e972efbd9c1803dddc1f7002f1e22c2de656b3aacd73a
	file oxford/lubm "$in14/LUBM/owl/LUBM_U.owl" cc41cc6019fdece26ffbf3aaf598d75d1c8bb2d820b7e7c409a0a8d57180ad4b
	file oxford/lubm "$in14/LUBM/dlog/LUBM_L.dlog" c45ad58036b867c7bdb2fd905859f8073002a886dc52565460ef6a170905aa56
	file oxford/lubm "$in14/LUBM/dlog/LUBM_LE.dlog" 2dfd1991550f57ec17977077057e75e96a825abd8d238d8bf40dd7f82938dee5
	file oxford/lubm "$in14/LUBM/dlog/LUBM_U.dlog" 06cb01074b8934b8609b481f16ca202906fb09944948864c7ce44b90df6e74b7
	finish oxford/lubm aaai-2014
fi

if begin oxford/opencyc aaai-2015; then
	unzip_one oxford/opencyc "$zips15/OpenCyc.zip" 72d8172855ec33aeee85018fa8565e53e7ee8603fc0fecaa5c02be9ab3ab73e3 OpenCyc.ttl
	file oxford/opencyc "$in15/OpenCyc/dlog/OpenCyc-eq.dlog" 4c1d6ba611fa0ad67614160a5ba09ba44b27537bde5ed7be593e6e0f2ba7e8e6
	finish oxford/opencyc aaai-2015
fi

if begin owl2bench zenodo-4764368; then
	zenodo="https://zenodo.org/api/records/4764368/files"
	# Zenodo serves each file at .../NAME/content.
	fetch "$zenodo/OWL2RL-1.owl/content" d62b95235d8584b50a56bc78a0da8d49f27bba42f2c93f634497d4c266147bf4 "$data/owl2bench/OWL2RL-1.owl"
	fetch "$zenodo/OWL2RL-2.owl/content" 343e8e3680832fe7fb62b951c27e1019b1d4e8c3a12189049ea847ce3c9dfdef "$data/owl2bench/OWL2RL-2.owl"
	fetch "$zenodo/OWL2RL-5.owl/content" efa6b316d5e387a929a89badfb992975ec213f69db1b9794e8a3634d323adb95 "$data/owl2bench/OWL2RL-5.owl"
	finish owl2bench zenodo-4764368
fi

if begin uba 1.7; then
	fetch "https://swat.cse.lehigh.edu/projects/lubm/uba1.7.zip" \
		3d44f468e36b7f3cd532f0f5693a019d020877846397b1fa657367cbd53f380a "$data/uba/uba1.7.zip"
	rm -rf "$data/uba/classes" "$data/uba/src"
	unzip -q -o "$data/uba/uba1.7.zip" -d "$data/uba"
	rm "$data/uba/uba1.7.zip"
	finish uba 1.7
fi

if $large; then
	if begin oxford/lubm-01k aaai-2014; then
		unzip_one oxford/lubm-01k "$zips14/LUBM-01K.zip" crc32:b41dc771 LUBM-01K.ttl
		finish oxford/lubm-01k aaai-2014
	fi
	if begin oxford/uobm-01k aaai-2014; then
		unzip_one oxford/uobm-01k "$zips14/UOBM-01K.zip" crc32:b4b422a4 UOBM-01K.ttl
		finish oxford/uobm-01k aaai-2014
	fi
	if begin oxford/dbpedia aaai-2014; then
		unzip_one oxford/dbpedia "$zips14/DBpedia.zip" crc32:aeed9657 DBPedia.ttl
		file oxford/dbpedia "$in14/DBpedia/owl/DBpedia_L.owl" 4ab2cb202e9f59d1a4fa6775a6bc582832233876a9708ecf8ee43a298128c0c0
		file oxford/dbpedia "$in14/DBpedia/owl/DBpedia_LE.owl" c73090e6c5b9d68db126108e90b9359c7068e7daf3824e4c7556a457a61cdea6
		file oxford/dbpedia "$in14/DBpedia/dlog/DBpedia_L.dlog" fb2a0438ff13fbb2d1d403cdb947dd1d69129503e18050b7f08744858fc35703
		file oxford/dbpedia "$in14/DBpedia/dlog/DBpedia_LE.dlog" 0b39294885f9b7752b84c04955d3ab0f1e03638bccbc8da7d19911e4dee6f33a
		finish oxford/dbpedia aaai-2014
	fi
	if begin oxford/uniprot aaai-2015; then
		unzip_one oxford/uniprot "$zips15/UniProt.zip" crc32:cce9c148 UniProt.ttl
		file oxford/uniprot "$in15/UniProt/dlog/UniProt-eq.dlog" 72bd7add95364b0df594ed9552eb29936f93b32273a8308f00d6476c9f192cc0
		finish oxford/uniprot aaai-2015
	fi
fi

# UBA writes each department to "<user.dir>\University<u>_<d>.owl" (a Windows
# separator), so with user.dir "." each lands in the working directory as
# ".\University<u>_<d>.owl". The merged file keeps one header, whose
# ontology is named by the univ-bench IRI where UBA writes "" (the file's
# own URL), as Oxford's LUBM data names it.
for n in ${lubm_sizes[@]+"${lubm_sizes[@]}"}; do
	out="$data/uba/lubm-$n.owl"
	[ ! -f "$out" ] || continue
	java="$root/tools/jre/bin/java"
	[ -x "$java" ] || { echo "fetch-data: --lubm needs tools/jre/bin/java: run scripts/fetch-tools.sh" >&2; exit 1; }
	echo "fetch-data: LUBM, $n universities (UBA 1.7, -index 0 -seed 0)"
	run="$data/uba/.lubm-$n"
	rm -rf "$run"
	mkdir "$run"
	onto="http://swat.cse.lehigh.edu/onto/univ-bench.owl"
	(cd "$run" && "$java" -Duser.dir=. -cp "$data/uba/classes" edu.lehigh.swat.bench.uba.Generator \
		-univ "$n" -index 0 -seed 0 -onto "$onto" >/dev/null)
	for ((u = 0; u < n; u++)); do
		files=()
		for ((d = 0; ; d++)); do
			f="$run/.\\University${u}_${d}.owl"
			[ -f "$f" ] || break
			files+=("$f")
		done
		[ ${#files[@]} -gt 0 ] || { echo "fetch-data: UBA wrote no file for university $u" >&2; exit 1; }
		awk -v first=$((u == 0)) -v onto="$onto" '
			FNR == 1 { body = 0 }
			body { if ($0 != "</rdf:RDF>") print; next }
			first && NR == FNR { sub(/rdf:about=""/, "rdf:about=\"" onto "\""); print }
			/<\/owl:Ontology>/ { body = 1 }' "${files[@]}"
	done >"$run/lubm.owl"
	echo "</rdf:RDF>" >>"$run/lubm.owl"
	mv "$run/lubm.owl" "$out"
	rm -rf "$run"
done

echo "fetch-data: done ($data)"
