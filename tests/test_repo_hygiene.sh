#!/usr/bin/env bash
set -u
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
fail=0
legacy='(^|/)(aooscope|web|webui\.py|start\.sh|Dockerfile\.python|.*\.pyc)$|(^|/)tests/(test_aooscope_.*\.py|parity/|designer_model_test\.mjs|test_repo_hygiene\.py)$'
while IFS= read -r path; do
  [[ -e "$path" ]] || continue
  if printf '%s\n' "$path" | rg -q "$legacy"; then
    echo "FAIL: legacy Python/runtime file remains: $path"
    fail=1
  fi
done < <(git ls-files --cached --others --exclude-standard)
[[ $fail -eq 0 ]] && echo 'PASS: legacy Python/runtime files absent'
if rg -n -g '!Sources/**' -g '!Cargo.lock' -g '!tests/fixtures/compat-v0.2/**' -g '!tests/test_repo_hygiene.sh' '(python3?|waitress|Dockerfile\.python|webui\.py|start\.sh)' README.md README.fr.md CONTRIBUTING.md SECURITY.md compose.yaml Dockerfile xtask crates docs .github 2>/dev/null; then
  echo 'FAIL: forbidden runtime references remain'
  fail=1
else
  echo 'PASS: forbidden runtime references absent'
fi
needles=("C""loud 9" "clo""ud9" "192.""168.1." "CT""130" "Arez""ki" "/srv/lxc/""administration")
while IFS= read -r path; do
  [[ -f "$path" ]] || continue
  [[ "$path" == Sources/* ]] && continue
  for needle in "${needles[@]}"; do
    if [[ "${path,,}" == *"${needle,,}"* ]] || grep -IqiF -- "$needle" "$path"; then
      printf 'FAIL: personal installation identifier in %s: %s\n' "$path" "$needle"
      fail=1
    fi
  done
done < <(git ls-files --cached --others --exclude-standard)
[[ $fail -eq 0 ]] && echo 'PASS: no personal installation identifiers'
exit "$fail"
