#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

I18N_DIR="$ROOT_DIR/assets/i18n"
LOCALES_FILE="$I18N_DIR/locales.json"
EN_FILE="$I18N_DIR/en.json"
STATE_FILE="$ROOT_DIR/.i18n/translate-index.json"
MODE="${1:-all}"
LOCALE="${LOCALE:-en}"
AUTH_MODE="${AUTH_MODE:-auto}"
TRANSLATOR_BIN="${TRANSLATOR_BIN:-greentic-i18n-translator}"
I18N_INCREMENTAL="${I18N_INCREMENTAL:-1}"

fail() {
  echo "error: $*" >&2
  exit 1
}

info() {
  echo "info: $*"
}

usage() {
  cat <<'USAGE'
Usage: tools/i18n.sh [all|translate|validate|status|check-auth]

Environment overrides:
  LOCALE=...          Locale for translator runtime messages (default: en)
  AUTH_MODE=...       Auth mode for translate (default: auto)
  TRANSLATOR_BIN=...  Translator command (default: greentic-i18n-translator)
  I18N_INCREMENTAL=1  Translate only missing/stale keys (default: 1; set 0 for full en.json)
USAGE
}

require_files() {
  [[ -f "$LOCALES_FILE" ]] || fail "missing $LOCALES_FILE"
  [[ -f "$EN_FILE" ]] || fail "missing $EN_FILE"
}

load_locales() {
  mapfile -t LOCALES < <(python3 - <<'PY' "$LOCALES_FILE"
import json, sys
for item in json.load(open(sys.argv[1], encoding="utf-8")):
    print(item)
PY
)
  LOCALES_CSV="$(IFS=,; echo "${LOCALES[*]}")"
}

ensure_locale_files() {
  for locale in "${LOCALES[@]}"; do
    file="$I18N_DIR/$locale.json"
    if [[ ! -f "$file" ]]; then
      mkdir -p "$(dirname "$file")"
      printf "{\n}\n" > "$file"
      info "created $file"
    fi
  done
}

json_syntax_check() {
  python3 -m json.tool "$EN_FILE" >/dev/null
  for locale in "${LOCALES[@]}"; do
    file="$I18N_DIR/$locale.json"
    [[ -f "$file" ]] || fail "missing $file"
    python3 -m json.tool "$file" >/dev/null
  done
}

ensure_codex_installed() {
  if command -v codex >/dev/null 2>&1; then
    return
  fi

  info "codex not found; attempting install via npm"
  if command -v npm >/dev/null 2>&1; then
    npm install -g @openai/codex
  else
    fail "codex is not installed and npm is unavailable; install codex manually"
  fi

  command -v codex >/dev/null 2>&1 || fail "codex install attempt failed"
}

ensure_codex_login() {
  local status_output
  status_output="$(codex login status 2>&1 || true)"
  if echo "$status_output" | grep -q "Logged in"; then
    return
  fi

  info "codex is not logged in; opening login flow"
  if [[ ! -t 0 ]]; then
    fail "non-interactive shell cannot complete codex login"
  fi

  codex login
  status_output="$(codex login status 2>&1 || true)"
  echo "$status_output" | grep -q "Logged in" || fail "codex login failed"
}

resolve_translator_bin() {
  if command -v "$TRANSLATOR_BIN" >/dev/null 2>&1; then
    return
  fi
  if command -v greentic-qa-translator >/dev/null 2>&1; then
    TRANSLATOR_BIN="greentic-qa-translator"
    return
  fi
  fail "translator binary not found: expected '$TRANSLATOR_BIN' (or greentic-qa-translator)"
}

run_translate() {
  local en_input="$1"
  "$TRANSLATOR_BIN" --locale "$LOCALE" \
    translate --langs "$LOCALES_CSV" --en "$en_input" --auth-mode "$AUTH_MODE"
}

run_validate() {
  "$TRANSLATOR_BIN" --locale "$LOCALE" \
    validate --langs "$LOCALES_CSV" --en "$EN_FILE"
}

run_status() {
  "$TRANSLATOR_BIN" --locale "$LOCALE" \
    status --langs "$LOCALES_CSV" --en "$EN_FILE"
}

check_key_coverage() {
  python3 - <<'PY' "$I18N_DIR" "$EN_FILE" "$LOCALES_FILE"
import json
import sys
from pathlib import Path

i18n_dir = Path(sys.argv[1])
en_path = Path(sys.argv[2])
locales_path = Path(sys.argv[3])

en = json.loads(en_path.read_text(encoding="utf-8"))
if not isinstance(en, dict):
    print(f"error: {en_path} must be a JSON object", file=sys.stderr)
    sys.exit(1)

locales = json.loads(locales_path.read_text(encoding="utf-8"))
if not isinstance(locales, list):
    print(f"error: {locales_path} must be a JSON array", file=sys.stderr)
    sys.exit(1)

failed = False
for locale in locales:
    locale_path = i18n_dir / f"{locale}.json"
    if not locale_path.exists():
        print(f"error: missing locale file {locale_path}", file=sys.stderr)
        failed = True
        continue
    raw = json.loads(locale_path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        print(f"error: {locale_path} must be a JSON object", file=sys.stderr)
        failed = True
        continue

    missing = [k for k in en.keys() if k not in raw]
    empty = [k for k in en.keys() if k in raw and str(raw[k]).strip() == ""]
    if missing or empty:
        failed = True
        print(f"error: {locale}.json is not fully translated", file=sys.stderr)
        if missing:
            print("  missing keys: " + ", ".join(missing), file=sys.stderr)
        if empty:
            print("  empty values: " + ", ".join(empty), file=sys.stderr)

if failed:
    sys.exit(1)

print(f"ok: all locale files fully cover en.json ({len(locales)} locales)")
PY
}

prepare_incremental_subset() {
  local subset_file="$1"
  local next_state_file="$2"
  python3 - <<'PY' "$I18N_DIR" "$EN_FILE" "$LOCALES_FILE" "$STATE_FILE" "$subset_file" "$next_state_file"
import hashlib
import json
import sys
from pathlib import Path

i18n_dir = Path(sys.argv[1])
en_path = Path(sys.argv[2])
locales_path = Path(sys.argv[3])
state_path = Path(sys.argv[4])
subset_path = Path(sys.argv[5])
next_state_path = Path(sys.argv[6])

en = json.loads(en_path.read_text(encoding="utf-8"))
if not isinstance(en, dict):
    print("error: en.json must be a JSON object", file=sys.stderr)
    sys.exit(1)

locales = json.loads(locales_path.read_text(encoding="utf-8"))
if not isinstance(locales, list):
    print("error: locales.json must be a JSON array", file=sys.stderr)
    sys.exit(1)

state_hashes = {}
if state_path.exists():
    try:
        state_raw = json.loads(state_path.read_text(encoding="utf-8"))
        if isinstance(state_raw, dict):
            hashes = state_raw.get("en_hashes")
            if isinstance(hashes, dict):
                state_hashes = {str(k): str(v) for k, v in hashes.items()}
    except Exception:
        state_hashes = {}

def key_hash(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()

current_hashes = {
    key: key_hash(value if isinstance(value, str) else str(value))
    for key, value in en.items()
}

dirty = set()

# English keys changed since last successful translation run.
for key, digest in current_hashes.items():
    if state_hashes.get(key) != digest:
        dirty.add(key)

# Missing/empty translations are always dirty.
for locale in locales:
    locale_path = i18n_dir / f"{locale}.json"
    if not locale_path.exists():
        dirty.update(en.keys())
        continue
    try:
        raw = json.loads(locale_path.read_text(encoding="utf-8"))
    except Exception:
        raw = {}
    if not isinstance(raw, dict):
        dirty.update(en.keys())
        continue
    for key in en.keys():
        if key not in raw:
            dirty.add(key)
            continue
        if str(raw.get(key, "")).strip() == "":
            dirty.add(key)

dirty_keys = sorted(dirty)
subset = {key: en[key] for key in dirty_keys}
subset_path.write_text(
    json.dumps(subset, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
)
next_state_path.write_text(
    json.dumps({"en_hashes": current_hashes}, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
print(len(dirty_keys))
PY
}

run_translate_incremental() {
  mkdir -p "$(dirname "$STATE_FILE")"
  local subset_file
  subset_file="$(mktemp "$I18N_DIR/.en.incremental.XXXXXX.json")"
  local next_state_file
  next_state_file="$(mktemp)"
  local dirty_count
  dirty_count="$(prepare_incremental_subset "$subset_file" "$next_state_file")"

  if [[ "$dirty_count" == "0" ]]; then
    info "no missing/stale keys detected; skipping translate"
    rm -f "$subset_file" "$next_state_file"
    return
  fi

  info "translating incremental subset: $dirty_count key(s)"
  run_translate "$subset_file"
  mv "$next_state_file" "$STATE_FILE"
  rm -f "$subset_file"
}

if [[ "$MODE" == "-h" || "$MODE" == "--help" ]]; then
  usage
  exit 0
fi

require_files
load_locales

case "$MODE" in
  all)
    ensure_locale_files
    json_syntax_check
    ensure_codex_installed
    ensure_codex_login
    resolve_translator_bin
    if [[ "$I18N_INCREMENTAL" == "1" ]]; then
      run_translate_incremental
    else
      run_translate "$EN_FILE"
    fi
    run_validate
    run_status
    check_key_coverage
    ;;
  translate)
    ensure_locale_files
    json_syntax_check
    ensure_codex_installed
    ensure_codex_login
    resolve_translator_bin
    if [[ "$I18N_INCREMENTAL" == "1" ]]; then
      run_translate_incremental
    else
      run_translate "$EN_FILE"
    fi
    ;;
  validate)
    ensure_locale_files
    json_syntax_check
    resolve_translator_bin
    run_validate
    check_key_coverage
    ;;
  status)
    ensure_locale_files
    json_syntax_check
    resolve_translator_bin
    run_status
    check_key_coverage
    ;;
  check-auth)
    ensure_codex_installed
    ensure_codex_login
    echo "ok: codex installed and logged in"
    ;;
  *)
    usage
    exit 2
    ;;
esac
