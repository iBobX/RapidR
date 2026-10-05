# Shared by the release scripts (sourced): the version, where things go.
#
#   dist/<version>/prep/   made once on the Mac (prepare.sh): every OS' home
#                          (home.py), the web interpreter, the IDE's bytecode,
#                          the source the VMs and containers build from
#   dist/<version>/work/   scratch, removed when a script ends
#   dist/<version>/out/    the release's files: what `gh release create` uploads
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
DIST="${RAPIDR_DIST:-$ROOT/dist/$VERSION}"
PREP="$DIST/prep"
OUT="$DIST/out"
WORK="$DIST/work"

die() { echo "error: $*" >&2; exit 1; }
step() { echo "== $*"; }
need() { command -v "$1" >/dev/null || die "$1 is needed ($2)"; }
# (nothing a release script runs prints or touches the user's registry)
export RAPIDR_PRINT_TO="${RAPIDR_PRINT_TO:-$DIST/prints}" RAPIDR_REGISTRY="${RAPIDR_REGISTRY:-$DIST/registry.reg}"
mkdir -p "$RAPIDR_PRINT_TO" "$OUT"
