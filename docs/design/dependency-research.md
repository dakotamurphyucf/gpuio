<!-- Imported from Linear 19a70399-3350-4355-90a3-37024022dec5 on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

Version rationale, original dependency comparison and pinned source provenance. Publication-time installed packages are supplementary and are not asserted to be the exact earlier test environment. Full native-v017 source/patch manifests are in the validated-experiment document and evidence bundle.

Published to GPUIO on 2026-09-10. [Download the source/evidence bundle](<https://uploads.linear.app/698151a6-07bd-4043-9a7b-15f84b8c23da/0a5f6f70-b343-4265-a61f-2aa405aeb710/0c24fc23-9d1e-4772-bf2b-9c2d5d9f2a05?signature=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJwYXRoIjoiLzY5ODE1MWE2LTA3YmQtNDA0My05YTdiLTE1Zjg0YjhjMjNkYS8wYTVmNmY3MC1iMzQzLTQyNjUtYTYxZi0yYWE0MDVhZWI3MTAvMGMyNGZjMjMtOWQxZS00NzcyLWJmMmItOWMyZDVkOWYyYTA1IiwiaWF0IjoxNzg5MTU1MTE2LCJleHAiOjE3ODkxNTU0MTZ9.mmZ2kuxAPBWsrqBY-72efCBNFvcgO48mY0S8fwUdtnQ>) for complete experiment sources, patches, original logs and file checksums. Historical local paths identify archive files; they are not setup instructions for a new machine.

## notes/bonsai-version-comparison.md

# Bonsai version comparison for GPUIO — 2026-09-10

Recommendation: start with Bonsai v0.17.0, stock OCaml 5.3.0, the v0.17 Jane Street dependency family, and Dune 3.24.2 using the validated native-library selection. Pin the exact experimental fork revision. The native-v017 experiment already validates the core integration; it does not validate all of Bonsai.

Compared source commits: v0.17 e929674585a67818734b06e12ea328872d185970 and preview f31661450eb133fe89564219d97669c2735c6622. This is a focused interface comparison, not an exhaustive release audit. CHANGES.md still starts at v0.17 and changelog.md at 2022, so neither provides a complete preview feature list.

v0.17 already has the graph-based Bonsai.Cont API, state machines, actors, keyed assoc, lifecycle hooks, clocks, memoized shared computations, dynamic scope and a manual native driver. The preview exposes the graph API directly at Bonsai. Use Cont for the initial adapter to reduce conceptual migration work. Do not describe Memo or recursive components as new preview features: they already exist in v0.17.

Verified preview additions relevant to future work:

* Debug.watch_computation and debug_node, plus richer driver instrumentation: potentially useful for diagnosing unexpected recomputation, but not required to render native widgets.
* Autopack and helpers such as assoc_n, wrap_n and with_model_resetter_n: reduce packing/unpacking boilerplate for multiple reactive outputs. Ordinary tuples/combinators are available in v0.17.
* state': functional state updates as a convenience API. The experiment implements this small helper through v0.17 state_machine0.
* with_inverted_lifecycle_ordering: explicitly orders dependent computation lifecycles. Ordinary activation/deactivation hooks exist and pass our v0.17 tests; no current host requirement needs this additional control.
* Memo.responses and memo subscriber inspection: extra visibility into shared computations, not the introduction of memoization itself.
* Library reorganization separating the core from browser libraries: cleaner native packaging upstream, whereas the v0.17 experiment uses explicit source directory selection.

The preview also contains substantial internal changes. We have not benchmarked equivalent workloads or audited every fix, so no claim that the versions have identical performance or correctness is justified. No preview-only feature identified in this review is required by the current GPUIO design. Revisit the version when a measured bottleneck, a required feature, or a compatible stable release supplies a concrete reason.

Keep Bonsai driver/version details in a small adapter and keep the Rust protocol/retained tree independent of Bonsai. Avoid recreating preview features speculatively. Current browser/terminal components are renderer-specific and are not ready-made GPUI widgets.

Primary evidence: sources/bonsai-v0.17/src/cont.mli; sources/bonsai-current/src/cont.mli (Autopack around line 30, state' around 153, lifecycle ordering around 522, Memo around 783, Debug around 1091); both src/bonsai.ml files; src/driver interfaces; native-v017/README.md and evidence logs. Upstream stable package: [https://opam.ocaml.org/packages/bonsai/](<https://opam.ocaml.org/packages/bonsai/>).

---

## notes/dependency-baseline.txt

```text
Installed packages:
# Packages matching: installed
# Name                      # Version
afl-persistent              1.4
angstrom                    0.16.1
arp                         4.1.0
asn1-combinators            0.3.2
astring                     0.8.5
atomic                      base
awa                         0.5.2
awa-mirage                  0.5.2
b0                          0.0.6
base                        v0.17.3
base-bigarray               base
base-bytes                  base
base-domains                base
base-effects                base
base-nnp                    base
base-threads                base
base-unix                   base
base64                      3.5.2
base_bigstring              v0.17.0
base_quickcheck             v0.17.0
bechamel                    0.5.0
bheap                       2.0.0
bigarray-compat             1.1.0
bigarray-overlap            0.2.1
bigstringaf                 0.10.0
bimage                      0.6.0
bimage-unix                 0.6.0
bin_prot                    v0.17.0-1
bos                         0.2.1
brr                         0.0.8
bstr                        0.0.4
ca-certs                    1.0.1
ca-certs-nss                3.118
camlp-streams               5.0.1
camlzip                     1.14
capitalization              v0.17.0
carton                      0.7.2
carton-git                  0.7.2
carton-lwt                  0.7.2
cf                          0.5.0
cf-lwt                      0.5.0
checkseum                   0.5.2
chrome-trace                3.21.1
cmdliner                    2.1.1
cohttp                      6.1.1
cohttp-eio                  6.1.1
cohttp-lwt                  6.1.1
cohttp-lwt-unix             6.1.1
conduit                     8.0.0
conduit-lwt                 8.0.0
conduit-lwt-unix            8.0.0
conf-g++                    1.0
conf-glpk                   1
conf-gmp                    5
conf-gmp-powm-sec           4
conf-libffi                 2.0.0
conf-libpcre                2
conf-libssl                 4
conf-oniguruma              1
conf-openblas               0.2.3
conf-openblas-macOS-env     1
conf-pkg-config             4
conf-zlib                   1
containers                  3.18
containers-data             3.18
core                        v0.17.1
core_kernel                 v0.17.0
core_unix                   v0.17.0
cppo                        1.8.0
crunch                      4.0.0
csexp                       1.5.2
cstruct                     6.2.0
cstruct-lwt                 6.2.0
cstruct-unix                6.2.0
ctypes                      0.24.0
ctypes-foreign              0.24.0
decompress                  1.5.3
diet                        0.4
digestif                    1.3.0
dns                         10.2.3
dns-client                  10.2.3
dns-client-mirage           10.2.3
domain-local-await          1.0.1
domain-name                 0.5.0
dot-merlin-reader           5.6-503
dowsing-lib                 dev
duff                        0.5
dune                        3.21.1
dune-build-info             3.21.1
dune-compiledb              0.6.0
dune-configurator           3.21.1
dune-rpc                    3.21.1
duration                    0.2.1
dyn                         3.21.1
eio                         1.3
eio-ssl                     0.3.0
eio_main                    1.3
eio_posix                   1.3
either                      1.0.0
emile                       1.1
encore                      0.8.1
eqaf                        0.10
ethernet                    3.2.0
expect_test_helpers_core    v0.17.0
ezgzip                      0.2.3
ezjsonm                     1.3.0
faraday                     0.8.2
fiber                       3.7.0
fieldslib                   v0.17.0
fix                         20250919
fmt                         0.11.0
fpath                       0.7.3
fs-io                       3.21.1
fsevents                    0.3.0
gel                         v0.17.0
gen                         1.1
git                         3.18.0
git-mirage                  3.18.0
git-paf                     3.18.0
git-unix                    3.18.0
gluten                      0.5.2
gluten-eio                  0.5.2
gmap                        0.3.0
h1                          1.0.0
h2                          0.13.0
h2-eio                      0.13.0
happy-eyeballs              2.0.1
happy-eyeballs-lwt          2.0.1
happy-eyeballs-mirage       2.0.1
hex                         1.5.0
hmap                        0.8.1
hpack                       0.13.0
http                        6.1.1
httpun                      0.2.0
httpun-eio                  0.2.0
httpun-types                0.2.0
httpun-ws                   0.2.0
hxd                         0.3.6
int_repr                    v0.17.0
integers                    0.7.0
iomux                       0.4
ipaddr                      5.6.1
ipaddr-cstruct              5.6.1
ipaddr-sexp                 5.6.1
iter                        1.9
jane-street-headers         v0.17.0
jane_rope                   v0.17.0
js_of_ocaml                 6.4.1
js_of_ocaml-compiler        6.4.1
js_of_ocaml-toplevel        6.4.1
jsonaf                      v0.17.0
jsonm                       1.0.2
jsonrpc                     1.23.1
jst-config                  v0.17.0
kdf                         1.0.0
ke                          0.6
lambda-term                 3.3.3
lambdasoup                  1.1.1
logs                        0.10.0
lp                          0.4.0
lp-glpk                     0.4.0
lru                         0.3.1
lsp                         1.23.1
lwd                         0.4
lwt                         5.9.2
lwt-dllist                  1.1.0
lwt_eio                     0.5.1
lwt_react                   1.2.0
macaddr                     5.6.1
macaddr-cstruct             5.6.1
magic-mime                  1.3.1
markup                      1.0.3
md2mld                      0.7.0
menhir                      20260209
menhirCST                   20260209
menhirGLR                   20260209
menhirLib                   20260209
menhirSdk                   20260209
merlin                      5.6-503
merlin-lib                  5.6-503
metrics                     0.5.0
mew                         0.1.0
mew_vi                      0.5.0
mimic                       0.0.9
mimic-happy-eyeballs        0.0.9
mirage-crypto               1.2.0
mirage-crypto-ec            1.2.0
mirage-crypto-pk            1.2.0
mirage-crypto-rng           1.2.0
mirage-crypto-rng-eio       1.2.0
mirage-flow                 5.0.0
mirage-kv                   6.1.1
mirage-mtime                5.2.0
mirage-net                  4.0.0
mirage-ptime                5.2.0
mirage-sleep                4.1.0
mtime                       2.1.0
nottui                      0.4
nottui-pretty               0.4
notty                       0.2.3
npy                         0.0.9
num                         1.6
ocaml                       5.3.0
ocaml-base-compiler         5.3.0
ocaml-compiler              5.3.0
ocaml-compiler-libs         v0.17.0
ocaml-config                3
ocaml-index                 5.6-503
ocaml-lsp-server            1.23.1
ocaml-options-vanilla       1
ocaml-platform-sdk          dev
ocaml-syntax-shims          1.0.0
ocaml-version               4.0.3
ocaml_intrinsics_kernel     v0.17.1
ocamlbuild                  0.16.1
ocamlc-loc                  3.21.1
ocamlfind                   1.9.8
ocamlformat                 0.28.1
ocamlformat-lib             0.28.1
ocamlformat-rpc-lib         0.28.1
ocamlgraph                  2.2.0
ocp-indent                  1.9.0
ocplib-endian               1.2
odig                        0.1.0
odoc                        3.1.0
odoc-parser                 3.1.0
ohex                        0.2.0
omd                         2.0.0~alpha4
oniguruma                   0.1.2
optint                      0.3.0
ordering                    3.21.1
ounit2                      2.2.7
owl                         1.2
owl-base                    1.2
paf                         0.8.0
parsexp                     v0.17.0
path_glob                   0.3
pcre                        8.0.5
pecu                        0.7
piaf                        0.2.0
pp                          2.0.0
ppx_assert                  v0.17.0
ppx_base                    v0.17.0
ppx_bench                   v0.17.0
ppx_bin_prot                v0.17.0
ppx_blob                    0.9.0
ppx_cold                    v0.17.0
ppx_compare                 v0.17.0
ppx_custom_printf           v0.17.0
ppx_derivers                1.2.1
ppx_deriving                6.0.3
ppx_deriving_yojson         3.9.1
ppx_diff                    v0.17.0
ppx_disable_unused_warnings v0.17.0
ppx_enumerate               v0.17.0
ppx_expect                  v0.17.2
ppx_fields_conv             v0.17.0
ppx_fixed_literal           v0.17.0
ppx_globalize               v0.17.0
ppx_hash                    v0.17.0
ppx_here                    v0.17.0
ppx_ignore_instrumentation  v0.17.0
ppx_inline_test             v0.17.0
ppx_jane                    v0.17.0
ppx_jsonaf_conv             v0.17.0
ppx_let                     v0.17.0
ppx_log                     v0.17.0
ppx_module_timer            v0.17.0
ppx_optcomp                 v0.17.0
ppx_optional                v0.17.0
ppx_pipebang                v0.17.0
ppx_repr                    0.7.0
ppx_sexp_conv               v0.17.0
ppx_sexp_message            v0.17.0
ppx_sexp_value              v0.17.0
ppx_stable                  v0.17.0
ppx_stable_witness          v0.17.0
ppx_string                  v0.17.0
ppx_string_conv             v0.17.0
ppx_tydi                    v0.17.0
ppx_typerep_conv            v0.17.0
ppx_variants_conv           v0.17.0
ppx_yojson_conv_lib         v0.17.0
ppxlib                      0.35.0
ppxlib_jane                 v0.17.2
prettym                     0.0.4
psq                         0.2.1
ptime                       1.2.0
randomconv                  0.2.0
re                          1.14.0
re2                         v0.17.0
react                       1.2.2
regex_parser_intf           v0.17.0
repr                        0.7.0
result                      1.5
rresult                     0.7.0
sedlex                      3.7
seq                         base
sexp_pretty                 v0.17.0
sexplib                     v0.17.0
sexplib0                    v0.17.0
sherlodoc                   3.1.0
spawn                       v0.17.0
splittable_random           v0.17.0
ssl                         0.7.0
stdio                       v0.17.0
stdlib-shims                0.3.0
stdune                      3.21.1
stringext                   1.6.0
tcpip                       9.0.1
textmate-language           0.4.0
thread-table                1.0.0
time_now                    v0.17.0
timezone                    v0.17.0
tls                         2.0.3
tls-eio                     2.0.3
tls-mirage                  2.0.3
top-closure                 3.21.1
topkg                       1.1.1
trace                       0.11
trace-tef                   0.11
trie                        1.0.0
typerep                     v0.17.1
tyxml                       4.6.0
uchar                       0.0.2
unstrctrd                   0.4
uopt                        v0.17.0
uri                         4.4.0
uri-sexp                    4.4.0
utop                        2.16.0
uucp                        17.0.0
uunf                        17.0.0
uuseg                       17.0.0
uutf                        1.0.4
variantslib                 v0.17.0
x509                        1.0.6
xdg                         3.21.1
yojson                      3.0.0
zarith                      1.14
zed                         3.2.3

Workspace lockfile:
opam-version: "2.0"
name: "ochat"
version: "dev"
synopsis: "A short synopsis"
description: "A longer description"
tags: ["topics" "to describe" "your" "project"]
depends: [
  "angstrom" {= "0.16.1"}
  "arp" {= "4.1.0"}
  "asn1-combinators" {= "0.3.2"}
  "astring" {= "0.8.5"}
  "awa" {= "0.6.1"}
  "awa-mirage" {= "0.6.1"}
  "base" {= "v0.17.3"}
  "base-bigarray" {= "base"}
  "base-bytes" {= "base"}
  "base-domains" {= "base"}
  "base-effects" {= "base"}
  "base-nnp" {= "base"}
  "base-threads" {= "base"}
  "base-unix" {= "base"}
  "base64" {= "3.5.2"}
  "base_bigstring" {= "v0.17.0"}
  "base_quickcheck" {= "v0.17.1"}
  "bheap" {= "2.0.0"}
  "bigstringaf" {= "0.10.0"}
  "bimage" {= "0.6.0"}
  "bimage-unix" {= "0.6.0"}
  "bin_prot" {= "v0.17.0-1"}
  "bos" {= "0.3.0"}
  "bstr" {= "0.1.1"}
  "ca-certs" {= "1.0.3"}
  "ca-certs-nss" {= "3.126"}
  "camlp-streams" {= "5.0.1"}
  "camlzip" {= "1.14"}
  "capitalization" {= "v0.17.0"}
  "carton" {= "0.7.2"}
  "carton-git" {= "0.7.2"}
  "carton-lwt" {= "0.7.2"}
  "cf" {= "0.5.0"}
  "cf-lwt" {= "0.5.0"}
  "checkseum" {= "0.5.3"}
  "cmdliner" {= "1.3.0"}
  "cohttp" {= "6.3.0"}
  "cohttp-eio" {= "6.3.0"}
  "cohttp-lwt" {= "6.3.0"}
  "cohttp-lwt-unix" {= "6.3.0"}
  "conduit" {= "8.0.0"}
  "conduit-lwt" {= "8.0.0"}
  "conduit-lwt-unix" {= "8.0.0"}
  "conf-g++" {= "1.0"}
  "conf-gmp" {= "5"}
  "conf-gmp-powm-sec" {= "4"}
  "conf-libffi" {= "2.0.0"}
  "conf-libpcre" {= "2"}
  "conf-libssl" {= "4"}
  "conf-oniguruma" {= "1"}
  "conf-openblas" {= "0.2.3"}
  "conf-pkg-config" {= "5"}
  "conf-zlib" {= "1"}
  "core" {= "v0.17.2"}
  "core_kernel" {= "v0.17.0"}
  "core_unix" {= "v0.17.1"}
  "cppo" {= "1.8.0"}
  "csexp" {= "1.5.2"}
  "cstruct" {= "6.3.0"}
  "cstruct-lwt" {= "6.3.0"}
  "ctypes" {= "0.24.0"}
  "ctypes-foreign" {= "0.24.0"}
  "decompress" {= "1.6.0"}
  "digestif" {= "1.3.1"}
  "dns" {= "10.2.5"}
  "dns-client" {= "10.2.5"}
  "dns-client-mirage" {= "10.2.5"}
  "domain-local-await" {= "1.0.1"}
  "domain-name" {= "0.5.0"}
  "duff" {= "0.5"}
  "dune" {= "3.24.2"}
  "dune-build-info" {= "3.24.2"}
  "dune-compiledb" {= "0.6.0"}
  "dune-configurator" {= "3.24.2"}
  "duration" {= "0.3.1"}
  "eio" {= "1.3"}
  "eio-ssl" {= "0.3.0"}
  "eio_linux" {= "1.3"}
  "eio_main" {= "1.3"}
  "eio_posix" {= "1.3"}
  "either" {= "1.0.0"}
  "emile" {= "1.1"}
  "encore" {= "0.8.1"}
  "eqaf" {= "0.10"}
  "ethernet" {= "3.2.0"}
  "expect_test_helpers_core" {= "v0.17.0"}
  "ezgzip" {= "0.2.3"}
  "ezjsonm" {= "1.3.0"}
  "faraday" {= "0.8.2"}
  "fieldslib" {= "v0.17.0"}
  "fmt" {= "0.11.0"}
  "fpath" {= "0.7.3"}
  "fsevents" {= "0.3.0"}
  "fsevents-lwt" {= "0.3.0"}
  "gel" {= "v0.17.0"}
  "git" {= "3.18.0"}
  "git-mirage" {= "3.18.0"}
  "git-paf" {= "3.18.0"}
  "git-unix" {= "3.18.0"}
  "gluten" {= "0.5.2"}
  "gluten-eio" {= "0.5.2"}
  "gmap" {= "0.3.0"}
  "h1" {= "1.0.0"}
  "h2" {= "0.13.0"}
  "h2-eio" {= "0.13.0"}
  "happy-eyeballs" {= "2.0.1"}
  "happy-eyeballs-lwt" {= "2.0.1"}
  "happy-eyeballs-mirage" {= "2.0.1"}
  "hex" {= "1.5.0"}
  "hmap" {= "0.8.1"}
  "hpack" {= "0.13.0"}
  "http" {= "6.3.0"}
  "httpun" {= "0.2.0"}
  "httpun-eio" {= "0.2.0"}
  "httpun-types" {= "0.2.0"}
  "httpun-ws" {= "0.2.0"}
  "hxd" {= "0.5.0"}
  "inotify" {= "2.6"}
  "int_repr" {= "v0.17.0"}
  "integers" {= "0.8.0"}
  "iomux" {= "0.4"}
  "ipaddr" {= "5.6.2"}
  "ipaddr-cstruct" {= "5.6.2"}
  "ipaddr-sexp" {= "5.6.2"}
  "irmin" {= "3.11.0"}
  "irmin-git" {= "3.11.0"}
  "irmin-watcher" {= "0.5.0"}
  "jane-street-headers" {= "v0.17.0"}
  "jane_rope" {= "v0.17.0"}
  "jsonaf" {= "v0.17.0"}
  "jsonm" {= "1.0.2"}
  "jst-config" {= "v0.17.0"}
  "kdf" {= "1.1.1"}
  "ke" {= "0.6"}
  "lambdasoup" {= "1.1.1"}
  "logs" {= "0.10.0"}
  "lru" {= "0.3.1"}
  "lwd" {= "0.4"}
  "lwt" {= "6.1.2"}
  "lwt-dllist" {= "1.1.0"}
  "lwt_eio" {= "0.6"}
  "macaddr" {= "5.6.2"}
  "macaddr-cstruct" {= "5.6.2"}
  "magic-mime" {= "1.3.1"}
  "markup" {= "1.0.3"}
  "menhir" {= "20260209"}
  "menhirCST" {= "20260209"}
  "menhirGLR" {= "20260209"}
  "menhirLib" {= "20260209"}
  "menhirSdk" {= "20260209"}
  "metrics" {= "0.5.0"}
  "mimic" {= "0.0.10"}
  "mimic-happy-eyeballs" {= "0.0.10"}
  "mirage-crypto" {= "1.2.0"}
  "mirage-crypto-ec" {= "1.2.0"}
  "mirage-crypto-pk" {= "1.2.0"}
  "mirage-crypto-rng" {= "1.2.0"}
  "mirage-crypto-rng-eio" {= "1.2.0"}
  "mirage-flow" {= "5.0.0"}
  "mirage-kv" {= "6.1.1"}
  "mirage-mtime" {= "5.2.0"}
  "mirage-net" {= "4.0.0"}
  "mirage-ptime" {= "5.2.0"}
  "mirage-sleep" {= "4.1.0"}
  "mtime" {= "2.2.0"}
  "nottui" {= "0.4"}
  "notty" {= "0.2.3"}
  "npy" {= "0.0.9"}
  "num" {= "1.6"}
  "ocaml" {= "5.3.0"}
  "ocaml-base-compiler" {= "5.3.0"}
  "ocaml-compiler" {= "5.3.0"}
  "ocaml-compiler-libs" {= "v0.17.0"}
  "ocaml-config" {= "3"}
  "ocaml-options-vanilla" {= "1"}
  "ocaml-syntax-shims" {= "1.0.0"}
  "ocaml_intrinsics_kernel" {= "v0.17.2"}
  "ocamlbuild" {= "0.16.1"}
  "ocamlfind" {= "1.9.8"}
  "ocamlgraph" {= "2.2.0"}
  "ocplib-endian" {= "1.2"}
  "ohex" {= "0.2.0"}
  "omd" {= "2.0.0~alpha4"}
  "oniguruma" {= "0.1.2"}
  "optint" {= "0.3.0"}
  "owl" {= "1.2"}
  "owl-base" {= "1.2"}
  "paf" {= "0.8.0"}
  "parsexp" {= "v0.17.0"}
  "path_glob" {= "0.3"}
  "pcre" {= "8.0.5"}
  "pecu" {= "0.7"}
  "piaf" {= "0.2.0"}
  "ppx_assert" {= "v0.17.0"}
  "ppx_base" {= "v0.17.0"}
  "ppx_bench" {= "v0.17.1"}
  "ppx_bin_prot" {= "v0.17.1"}
  "ppx_blob" {= "0.9.0"}
  "ppx_cold" {= "v0.17.0"}
  "ppx_compare" {= "v0.17.0"}
  "ppx_custom_printf" {= "v0.17.0"}
  "ppx_derivers" {= "1.2.1"}
  "ppx_deriving" {= "6.1.3"}
  "ppx_diff" {= "v0.17.1"}
  "ppx_disable_unused_warnings" {= "v0.17.0"}
  "ppx_enumerate" {= "v0.17.0"}
  "ppx_expect" {= "v0.17.3"}
  "ppx_fields_conv" {= "v0.17.0"}
  "ppx_fixed_literal" {= "v0.17.0"}
  "ppx_globalize" {= "v0.17.2"}
  "ppx_hash" {= "v0.17.0"}
  "ppx_here" {= "v0.17.0"}
  "ppx_ignore_instrumentation" {= "v0.17.0"}
  "ppx_inline_test" {= "v0.17.1"}
  "ppx_irmin" {= "3.11.0"}
  "ppx_jane" {= "v0.17.0"}
  "ppx_jsonaf_conv" {= "v0.17.1"}
  "ppx_let" {= "v0.17.1"}
  "ppx_log" {= "v0.17.0"}
  "ppx_module_timer" {= "v0.17.0"}
  "ppx_optcomp" {= "v0.17.1"}
  "ppx_optional" {= "v0.17.0"}
  "ppx_pipebang" {= "v0.17.0"}
  "ppx_repr" {= "0.8.0"}
  "ppx_sexp_conv" {= "v0.17.1"}
  "ppx_sexp_message" {= "v0.17.0"}
  "ppx_sexp_value" {= "v0.17.0"}
  "ppx_stable" {= "v0.17.1"}
  "ppx_stable_witness" {= "v0.17.0"}
  "ppx_string" {= "v0.17.0"}
  "ppx_string_conv" {= "v0.17.0"}
  "ppx_tydi" {= "v0.17.1"}
  "ppx_typerep_conv" {= "v0.17.1"}
  "ppx_variants_conv" {= "v0.17.1"}
  "ppxlib" {= "0.38.0"}
  "ppxlib_jane" {= "v0.17.4"}
  "prettym" {= "0.0.5"}
  "psq" {= "0.2.1"}
  "ptime" {= "1.2.0"}
  "randomconv" {= "0.2.0"}
  "re" {= "1.14.0"}
  "re2" {= "v0.17.0"}
  "regex_parser_intf" {= "v0.17.0"}
  "repr" {= "0.8.0"}
  "rresult" {= "0.7.0"}
  "seq" {= "base"}
  "sexp_pretty" {= "v0.17.0"}
  "sexplib" {= "v0.17.0"}
  "sexplib0" {= "v0.17.0"}
  "spawn" {= "v0.17.0"}
  "splittable_random" {= "v0.17.0"}
  "ssl" {= "0.7.0"}
  "stdio" {= "v0.17.0"}
  "stdlib-shims" {= "0.3.0"}
  "stringext" {= "1.6.0"}
  "tcpip" {= "9.0.1"}
  "textmate-language" {= "0.4.0"}
  "thread-table" {= "1.0.0"}
  "time_now" {= "v0.17.0"}
  "timezone" {= "v0.17.0"}
  "tls" {= "2.1.2"}
  "tls-eio" {= "2.1.2"}
  "tls-mirage" {= "2.1.2"}
  "topkg" {= "1.1.1"}
  "typerep" {= "v0.17.1"}
  "uchar" {= "0.0.2"}
  "unstrctrd" {= "0.4"}
  "uopt" {= "v0.17.0"}
  "uri" {= "4.4.0"}
  "uri-sexp" {= "4.4.0"}
  "uring" {= "2.7.0"}
  "uucp" {= "17.0.0"}
  "uunf" {= "17.0.0"}
  "uuseg" {= "17.0.0"}
  "uutf" {= "1.0.4"}
  "variantslib" {= "v0.17.0"}
  "x509" {= "1.1.1"}
  "zarith" {= "1.14"}
]
build: [
  ["dune" "subst"] {dev}
  [
    "dune"
    "build"
    "-p"
    name
    "-j"
    jobs
    "@install"
    "@runtest" {with-test}
    "@doc" {with-doc}
  ]
]
pin-depends: [
  [
    "piaf.0.2.0"
    "git+https://github.com/anmonteiro/piaf.git#c7428ec14dc681e0ef13375cdf657b2727143678"
  ]
  [
    "textmate-language.0.4.0"
    "git+https://github.com/dakotamurphyucf/ocaml-textmate-language.git#e9d1ea854b0734db5e616cacb2ce7e2e5fd744bf"
  ]
]
x-maintenance-intent: ["(latest)"]
```

---

## notes/source-manifest.json

```json
[
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/binprot-rs",
    "repository": "https://github.com/LaurentMazare/binprot-rs.git",
    "commit": "165b4d64d8f2a580af453f1e60deba41c28cd6df",
    "commit_date": "2023-12-16"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/bonsai-current",
    "repository": "https://github.com/janestreet/bonsai.git",
    "commit": "f31661450eb133fe89564219d97669c2735c6622",
    "commit_date": "2026-07-10"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/bonsai-v0.17",
    "repository": "https://github.com/janestreet/bonsai.git",
    "commit": "e929674585a67818734b06e12ea328872d185970",
    "commit_date": "2024-05-07"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/bonsai_concrete-current",
    "repository": "https://github.com/janestreet/bonsai_concrete.git",
    "commit": "10601f857306e691462fa049cb8b58c162d86cca",
    "commit_date": "2026-07-10"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/bonsai_term-current",
    "repository": "https://github.com/janestreet/bonsai_term.git",
    "commit": "2457232d3aa144fb887a053748a920544db60f72",
    "commit_date": "2026-07-10"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/gpuix",
    "repository": "https://github.com/remorses/gpuix.git",
    "commit": "18e695ed0ee8121a7793413ca795e08eda2a13df",
    "commit_date": "2026-09-10"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/incr_dom-v0.17",
    "repository": "https://github.com/janestreet/incr_dom.git",
    "commit": "36459c50998a21c52affdebcc5e83f19c5312360",
    "commit_date": "2024-05-07"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/janestreet-opam-current",
    "repository": "https://github.com/janestreet/opam-repository.git",
    "commit": "6789b91abef324f0f9dc2a07332afc4843c7dbe5",
    "commit_date": "2026-07-10"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/ocaml-interop",
    "repository": "https://github.com/tizoc/ocaml-interop.git",
    "commit": "9f941c63b8a9dcd652c6535da196f48d7a4dcc75",
    "commit_date": "2025-05-27"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/ocaml-rs",
    "repository": "https://github.com/zshipko/ocaml-rs.git",
    "commit": "2f6649dcd72471d380112a28027b7b98d3300a1b",
    "commit_date": "2026-08-12"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/virtual_dom-v0.17",
    "repository": "https://github.com/janestreet/virtual_dom.git",
    "commit": "e2c80cea41db6484825b5d4b487a042ef058d591",
    "commit_date": "2024-05-07"
  },
  {
    "directory": "/Users/dakotamurphy/gpuio-research-2026-09-10/sources/zed",
    "repository": "https://github.com/zed-industries/zed.git",
    "commit": "a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b",
    "commit_date": "2026-09-10"
  }
]
```

---

## notes/bonsai-opam-local-metadata.txt

```text
opam-version: "2.0"
name: "bonsai"
version: "v0.17.0"
synopsis: "A library for building dynamic webapps, using Js_of_ocaml"
description: """\
Bonsai is a library for building reusable UI components inside an
     Incremental-style UI framework such as Incr_dom or React."""
maintainer: "Jane Street developers"
authors: "Jane Street Group, LLC"
license: "MIT"
homepage: "https://github.com/janestreet/bonsai"
doc: "https://ocaml.janestreet.com/ocaml-core/latest/doc/bonsai/index.html"
bug-reports: "https://github.com/janestreet/bonsai/issues"
depends: [
  "ocaml" {>= "5.1.0"}
  "async" {>= "v0.17" & < "v0.18~"}
  "async_durable" {>= "v0.17" & < "v0.18~"}
  "async_extra" {>= "v0.17" & < "v0.18~"}
  "async_js" {>= "v0.17" & < "v0.18~"}
  "async_kernel" {>= "v0.17" & < "v0.18~"}
  "async_rpc_kernel" {>= "v0.17" & < "v0.18~"}
  "async_rpc_websocket" {>= "v0.17" & < "v0.18~"}
  "babel" {>= "v0.17" & < "v0.18~"}
  "base" {>= "v0.17" & < "v0.18~"}
  "bin_prot" {>= "v0.17" & < "v0.18~"}
  "core" {>= "v0.17" & < "v0.18~"}
  "core_bench" {>= "v0.17" & < "v0.18~"}
  "core_kernel" {>= "v0.17" & < "v0.18~"}
  "core_unix" {>= "v0.17" & < "v0.18~"}
  "expect_test_helpers_core" {>= "v0.17" & < "v0.18~"}
  "fuzzy_match" {>= "v0.17" & < "v0.18~"}
  "incr_dom" {>= "v0.17" & < "v0.18~"}
  "incr_map" {>= "v0.17" & < "v0.18~"}
  "legacy_diffable" {>= "v0.17" & < "v0.18~"}
  "ordinal_abbreviation" {>= "v0.17" & < "v0.18~"}
  "patdiff" {>= "v0.17" & < "v0.18~"}
  "polling_state_rpc" {>= "v0.17" & < "v0.18~"}
  "ppx_css" {>= "v0.17" & < "v0.18~"}
  "ppx_diff" {>= "v0.17" & < "v0.18~"}
  "ppx_here" {>= "v0.17" & < "v0.18~"}
  "ppx_jane" {>= "v0.17" & < "v0.18~"}
  "ppx_let" {>= "v0.17" & < "v0.18~"}
  "ppx_pattern_bind" {>= "v0.17" & < "v0.18~"}
  "ppx_quick_test" {>= "v0.17" & < "v0.18~"}
  "ppx_typed_fields" {>= "v0.17" & < "v0.18~"}
  "profunctor" {>= "v0.17" & < "v0.18~"}
  "record_builder" {>= "v0.17" & < "v0.18~"}
  "sexp_grammar" {>= "v0.17" & < "v0.18~"}
  "sexplib0" {>= "v0.17" & < "v0.18~"}
  "streamable" {>= "v0.17" & < "v0.18~"}
  "textutils" {>= "v0.17" & < "v0.18~"}
  "versioned_polling_state_rpc" {>= "v0.17" & < "v0.18~"}
  "virtual_dom" {>= "v0.17" & < "v0.18~"}
  "base64" {>= "3.4.0"}
  "cohttp-async" {>= "2.5.7" & < "3.0.0" | >= "5.1.1" & < "6.0.0"}
  "dune" {>= "3.11.0" & < "3.24.0"}
  "gen_js_api" {>= "1.0.8"}
  "js_of_ocaml" {>= "5.1.1" & < "5.7.0"}
  "js_of_ocaml-ppx" {>= "5.1.1" & < "5.7.0"}
  "ocaml-embed-file" {>= "v0.17" & < "v0.18~"}
  "ppxlib" {>= "0.28.0"}
  "re" {>= "1.8.0"}
  "uri" {>= "3.0.0"}
]
available: arch != "arm32" & arch != "x86_32"
build: ["dune" "build" "-p" name "-j" jobs]
dev-repo: "git+https://github.com/janestreet/bonsai.git"
url {
  src:
    "https://github.com/janestreet/bonsai/archive/refs/tags/v0.17.0.tar.gz"
  checksum:
    "sha256=c78c4476ee6b856846e2d0941e5965009d5e1b853e564b2b1bee61202f0b1ebb"
}
x-maintenance-intent: ["(latest)"]
```

---

## native-v017/evidence/installed-packages-publication.txt

```text
afl-persistent              1.4
angstrom                    0.16.1
arp                         4.1.0
asn1-combinators            0.3.2
astring                     0.8.5
atomic                      base
awa                         0.5.2
awa-mirage                  0.5.2
b0                          0.0.6
base                        v0.17.3
base-bigarray               base
base-bytes                  base
base-domains                base
base-effects                base
base-nnp                    base
base-threads                base
base-unix                   base
base64                      3.5.2
base_bigstring              v0.17.0
base_quickcheck             v0.17.0
bechamel                    0.5.0
bheap                       2.0.0
bigarray-compat             1.1.0
bigarray-overlap            0.2.1
bigstringaf                 0.10.0
bimage                      0.6.0
bimage-unix                 0.6.0
bin_prot                    v0.17.0-1
bos                         0.2.1
brr                         0.0.8
bstr                        0.0.4
ca-certs                    1.0.1
ca-certs-nss                3.118
camlp-streams               5.0.1
camlzip                     1.14
capitalization              v0.17.0
carton                      0.7.2
carton-git                  0.7.2
carton-lwt                  0.7.2
cf                          0.5.0
cf-lwt                      0.5.0
checkseum                   0.5.2
chrome-trace                3.21.1
cmdliner                    2.1.1
cohttp                      6.1.1
cohttp-eio                  6.1.1
cohttp-lwt                  6.1.1
cohttp-lwt-unix             6.1.1
conduit                     8.0.0
conduit-lwt                 8.0.0
conduit-lwt-unix            8.0.0
conf-g++                    1.0
conf-glpk                   1
conf-gmp                    5
conf-gmp-powm-sec           4
conf-libffi                 2.0.0
conf-libpcre                2
conf-libssl                 4
conf-oniguruma              1
conf-openblas               0.2.3
conf-openblas-macOS-env     1
conf-pkg-config             4
conf-zlib                   1
containers                  3.18
containers-data             3.18
core                        v0.17.1
core_kernel                 v0.17.0
core_unix                   v0.17.0
cppo                        1.8.0
crunch                      4.0.0
csexp                       1.5.2
cstruct                     6.2.0
cstruct-lwt                 6.2.0
cstruct-unix                6.2.0
ctypes                      0.24.0
ctypes-foreign              0.24.0
decompress                  1.5.3
diet                        0.4
digestif                    1.3.0
dns                         10.2.3
dns-client                  10.2.3
dns-client-mirage           10.2.3
domain-local-await          1.0.1
domain-name                 0.5.0
dot-merlin-reader           5.6-503
dowsing-lib                 dev
duff                        0.5
dune                        3.21.1
dune-build-info             3.21.1
dune-compiledb              0.6.0
dune-configurator           3.21.1
dune-rpc                    3.21.1
duration                    0.2.1
dyn                         3.21.1
eio                         1.3
eio-ssl                     0.3.0
eio_main                    1.3
eio_posix                   1.3
either                      1.0.0
emile                       1.1
encore                      0.8.1
eqaf                        0.10
ethernet                    3.2.0
expect_test_helpers_core    v0.17.0
ezgzip                      0.2.3
ezjsonm                     1.3.0
faraday                     0.8.2
fiber                       3.7.0
fieldslib                   v0.17.0
fix                         20250919
fmt                         0.11.0
fpath                       0.7.3
fs-io                       3.21.1
fsevents                    0.3.0
gel                         v0.17.0
gen                         1.1
git                         3.18.0
git-mirage                  3.18.0
git-paf                     3.18.0
git-unix                    3.18.0
gluten                      0.5.2
gluten-eio                  0.5.2
gmap                        0.3.0
h1                          1.0.0
h2                          0.13.0
h2-eio                      0.13.0
happy-eyeballs              2.0.1
happy-eyeballs-lwt          2.0.1
happy-eyeballs-mirage       2.0.1
hex                         1.5.0
hmap                        0.8.1
hpack                       0.13.0
http                        6.1.1
httpun                      0.2.0
httpun-eio                  0.2.0
httpun-types                0.2.0
httpun-ws                   0.2.0
hxd                         0.3.6
int_repr                    v0.17.0
integers                    0.7.0
iomux                       0.4
ipaddr                      5.6.1
ipaddr-cstruct              5.6.1
ipaddr-sexp                 5.6.1
iter                        1.9
jane-street-headers         v0.17.0
jane_rope                   v0.17.0
js_of_ocaml                 6.4.1
js_of_ocaml-compiler        6.4.1
js_of_ocaml-toplevel        6.4.1
jsonaf                      v0.17.0
jsonm                       1.0.2
jsonrpc                     1.23.1
jst-config                  v0.17.0
kdf                         1.0.0
ke                          0.6
lambda-term                 3.3.3
lambdasoup                  1.1.1
logs                        0.10.0
lp                          0.4.0
lp-glpk                     0.4.0
lru                         0.3.1
lsp                         1.23.1
lwd                         0.4
lwt                         5.9.2
lwt-dllist                  1.1.0
lwt_eio                     0.5.1
lwt_react                   1.2.0
macaddr                     5.6.1
macaddr-cstruct             5.6.1
magic-mime                  1.3.1
markup                      1.0.3
md2mld                      0.7.0
menhir                      20260209
menhirCST                   20260209
menhirGLR                   20260209
menhirLib                   20260209
menhirSdk                   20260209
merlin                      5.6-503
merlin-lib                  5.6-503
metrics                     0.5.0
mew                         0.1.0
mew_vi                      0.5.0
mimic                       0.0.9
mimic-happy-eyeballs        0.0.9
mirage-crypto               1.2.0
mirage-crypto-ec            1.2.0
mirage-crypto-pk            1.2.0
mirage-crypto-rng           1.2.0
mirage-crypto-rng-eio       1.2.0
mirage-flow                 5.0.0
mirage-kv                   6.1.1
mirage-mtime                5.2.0
mirage-net                  4.0.0
mirage-ptime                5.2.0
mirage-sleep                4.1.0
mtime                       2.1.0
nottui                      0.4
nottui-pretty               0.4
notty                       0.2.3
npy                         0.0.9
num                         1.6
ocaml                       5.3.0
ocaml-base-compiler         5.3.0
ocaml-compiler              5.3.0
ocaml-compiler-libs         v0.17.0
ocaml-config                3
ocaml-index                 5.6-503
ocaml-lsp-server            1.23.1
ocaml-options-vanilla       1
ocaml-platform-sdk          dev
ocaml-syntax-shims          1.0.0
ocaml-version               4.0.3
ocaml_intrinsics_kernel     v0.17.1
ocamlbuild                  0.16.1
ocamlc-loc                  3.21.1
ocamlfind                   1.9.8
ocamlformat                 0.28.1
ocamlformat-lib             0.28.1
ocamlformat-rpc-lib         0.28.1
ocamlgraph                  2.2.0
ocp-indent                  1.9.0
ocplib-endian               1.2
odig                        0.1.0
odoc                        3.1.0
odoc-parser                 3.1.0
ohex                        0.2.0
omd                         2.0.0~alpha4
oniguruma                   0.1.2
optint                      0.3.0
ordering                    3.21.1
ounit2                      2.2.7
owl                         1.2
owl-base                    1.2
paf                         0.8.0
parsexp                     v0.17.0
path_glob                   0.3
pcre                        8.0.5
pecu                        0.7
piaf                        0.2.0
pp                          2.0.0
ppx_assert                  v0.17.0
ppx_base                    v0.17.0
ppx_bench                   v0.17.0
ppx_bin_prot                v0.17.0
ppx_blob                    0.9.0
ppx_cold                    v0.17.0
ppx_compare                 v0.17.0
ppx_custom_printf           v0.17.0
ppx_derivers                1.2.1
ppx_deriving                6.0.3
ppx_deriving_yojson         3.9.1
ppx_diff                    v0.17.0
ppx_disable_unused_warnings v0.17.0
ppx_enumerate               v0.17.0
ppx_expect                  v0.17.2
ppx_fields_conv             v0.17.0
ppx_fixed_literal           v0.17.0
ppx_globalize               v0.17.0
ppx_hash                    v0.17.0
ppx_here                    v0.17.0
ppx_ignore_instrumentation  v0.17.0
ppx_inline_test             v0.17.0
ppx_jane                    v0.17.0
ppx_jsonaf_conv             v0.17.0
ppx_let                     v0.17.0
ppx_log                     v0.17.0
ppx_module_timer            v0.17.0
ppx_optcomp                 v0.17.0
ppx_optional                v0.17.0
ppx_pipebang                v0.17.0
ppx_repr                    0.7.0
ppx_sexp_conv               v0.17.0
ppx_sexp_message            v0.17.0
ppx_sexp_value              v0.17.0
ppx_stable                  v0.17.0
ppx_stable_witness          v0.17.0
ppx_string                  v0.17.0
ppx_string_conv             v0.17.0
ppx_tydi                    v0.17.0
ppx_typerep_conv            v0.17.0
ppx_variants_conv           v0.17.0
ppx_yojson_conv_lib         v0.17.0
ppxlib                      0.35.0
ppxlib_jane                 v0.17.2
prettym                     0.0.4
psq                         0.2.1
ptime                       1.2.0
randomconv                  0.2.0
re                          1.14.0
re2                         v0.17.0
react                       1.2.2
regex_parser_intf           v0.17.0
repr                        0.7.0
result                      1.5
rresult                     0.7.0
sedlex                      3.7
seq                         base
sexp_pretty                 v0.17.0
sexplib                     v0.17.0
sexplib0                    v0.17.0
sherlodoc                   3.1.0
spawn                       v0.17.0
splittable_random           v0.17.0
ssl                         0.7.0
stdio                       v0.17.0
stdlib-shims                0.3.0
stdune                      3.21.1
stringext                   1.6.0
tcpip                       9.0.1
textmate-language           0.4.0
thread-table                1.0.0
time_now                    v0.17.0
timezone                    v0.17.0
tls                         2.0.3
tls-eio                     2.0.3
tls-mirage                  2.0.3
top-closure                 3.21.1
topkg                       1.1.1
trace                       0.11
trace-tef                   0.11
trie                        1.0.0
typerep                     v0.17.1
tyxml                       4.6.0
uchar                       0.0.2
unstrctrd                   0.4
uopt                        v0.17.0
uri                         4.4.0
uri-sexp                    4.4.0
utop                        2.16.0
uucp                        17.0.0
uunf                        17.0.0
uuseg                       17.0.0
uutf                        1.0.4
variantslib                 v0.17.0
x509                        1.0.6
xdg                         3.21.1
yojson                      3.0.0
zarith                      1.14
zed                         3.2.3
```

