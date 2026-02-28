#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

I18N_DIR="$ROOT_DIR/assets/i18n"
LOCALES_FILE="$I18N_DIR/locales.json"
EN_FILE="$I18N_DIR/en.json"
MODE="${1:-all}"
LOCALE="${LOCALE:-en}"
AUTH_MODE="${AUTH_MODE:-auto}"
TRANSLATOR_BIN="${TRANSLATOR_BIN:-greentic-i18n-translator}"

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
  "$TRANSLATOR_BIN" --locale "$LOCALE" \
    translate --langs "$LOCALES_CSV" --en "$EN_FILE" --auth-mode "$AUTH_MODE"
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
    run_translate
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
    run_translate
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
