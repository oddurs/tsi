#!/usr/bin/env bash
# A guided tour of tsi: every command, the new output system, and the library.
# Press Enter between stops. TOUR_NO_PAUSE=1 runs straight through.
set -u
cd "$(dirname "$0")/.."
printf 'Building release binary...\n'
cargo build --release -q || exit 1
tsi() { ./target/release/tsi "$@"; }
has_jq() { command -v jq >/dev/null; }
field() { tsi "$@" -o json | grep -m1 "\"$FIELD\"" | tr -dc '0-9.' | cut -d. -f1; }

n=0
stop() {
  n=$((n + 1))
  printf '\n\033[1;36m━━ %s. %s\033[0m\n' "$n" "$1"
  shift
  for line in "$@"; do printf '\033[2m   %s\033[0m\n' "$line"; done
  [ "${TOUR_NO_PAUSE:-}" = 1 ] || read -rp $'\n[Enter] ' _
}
act() { printf '\n\n\033[1;35m══════ %s ══════\033[0m\n' "$1"; }
show() { printf '\n\033[1m$ %s\033[0m\n' "$*"; }
run() { show "tsi $*"; tsi "$@"; }
# Output that is cut short: keep colour on through the pipe.
run_head() { local lines=$1; shift; show "tsi $* | head -$lines"; tsi --color always "$@" 2>&1 | head -"$lines"; }

R=(--payload 5000 --target-dv 9400 --engine raptor-2 --quiet)

# ─────────────────────────────────────────────────────────────────────────
act "I. The engine database"

stop "Every built-in engine" \
  "Real engines, embedded in the binary. Vacuum thrust and Isp by default."
run engines

stop "Filters, and sea-level performance" \
  "--propellant and --name narrow the list; --verbose adds sea-level thrust and" \
  "Isp, which is what a booster actually gets at liftoff."
run engines --propellant methane --verbose

# ─────────────────────────────────────────────────────────────────────────
act "II. One stage: the rocket equation"

stop "calculate" \
  "One Raptor-2 with 100 t of propellant: Δv, burn time, thrust-to-weight."
run calculate --engine raptor-2 --propellant-mass 100000

stop "Three output formats" \
  "pretty for people, compact for a status line, json for programs."
run calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000 -o compact
run calculate --isp 311 --mass-ratio 3.5 -o compact
run calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000 -o json

# ─────────────────────────────────────────────────────────────────────────
act "III. Staging a rocket"

stop "5 t to low Earth orbit on Raptors" \
  "The stage table, then bars for where the Δv and the mass go. The booster" \
  "takes less Δv than the upper stage: from sea level its Raptor averages" \
  "~345 s instead of 350 s, so Δv is cheaper up top."
run optimize "${R[@]}"

stop "Draw it, and count the losses" \
  "--diagram draws the stack to scale (height ~ sqrt of mass); --show-losses" \
  "estimates gravity, drag and steering losses against orbital velocity."
run optimize "${R[@]}" --diagram --show-losses

stop "Two optimizers that share no search logic" \
  "Analytical (Lagrange + numeric refinement) versus an exhaustive grid."
FIELD=total_mass_kg
printf '  analytical:  %s kg\n' "$(field optimize "${R[@]}")"
printf '  brute force: %s kg\n' "$(field optimize "${R[@]}" --optimizer brute-force)"

stop "Monte Carlo: build the design 5,000 times" \
  "Real engines and tanks vary. With zero margin, about half the builds fall" \
  "short, and tsi tells you the margin that would fix it."
run optimize "${R[@]}" --monte-carlo 5000 --seed 1

stop "Margin buys confidence" "The same problem, designed with 3% headroom."
run optimize "${R[@]}" --margin 3 --monte-carlo 5000 --seed 1

stop "Mixing engines, up to three stages" \
  "Offer kerosene and hydrogen: hydrogen goes on top, and the RL-10C, which" \
  "can't run at sea level, never flies the booster. Kerosene is so much worse" \
  "that the booster does only the 2,000 m/s minimum and hydrogen does the rest."
run optimize --payload 5000 --target-dv 9400 --engine merlin-1d,rl-10c --max-stages 3 --quiet --diagram

stop "Pinning an engine to a stage" "Merlins on the booster, Raptor above."
run_head 12 optimize "${R[@]}" --stage1-engine merlin-1d

stop "Launch from Mars" \
  "At 38% of Earth's gravity each engine lifts 2.6x as much, and with no air" \
  "the booster gets full vacuum Isp."
M=(optimize --payload 50000 --target-dv 4500 --engine raptor-2 --quiet)
for g in earth mars; do
  FIELD=total_mass_kg; kg=$(field "${M[@]}" --gravity $g)
  FIELD=engine_count; engines=$(field "${M[@]}" --gravity $g)
  printf '  %-6s %s kg, %s booster engine(s)\n' "$g:" "$kg" "$engines"
done

stop "Bring your own engine, launch from the Moon" \
  "--custom-engine name:thrust_kn:isp_s:mass_kg:propellant"
run optimize --payload 2000 --target-dv 6000 --custom-engine "Kestrel-X:200:340:300:loxch4" \
  --engine Kestrel-X --gravity moon --quiet --diagram

stop "Super Heavy class" \
  "100 t to orbit needs more than nine Raptors. tsi says which limit binds" \
  "and which flag to change; then we change it."
run optimize --payload 100000 --target-dv 9400 --engine raptor-2 --quiet
run_head 12 optimize --payload 100000 --target-dv 9400 --engine raptor-2 --max-engines 40 --quiet

# ─────────────────────────────────────────────────────────────────────────
act "IV. Output for people and for programs"

stop "--ascii" "For terminals and fonts without box-drawing characters. Symbols too."
run optimize "${R[@]}" --diagram --ascii

stop "Colour only where it helps" \
  "Colour is on in a terminal, off in a pipe, off with NO_COLOR or --color" \
  "never. Counting escape codes in each case:"
count() { grep -c $'\033\\[' || true; }
printf '  into a pipe:         %s lines with colour\n' "$(tsi engines | count)"
printf '  --color always:      %s lines with colour\n' "$(tsi --color always engines | count)"
printf '  NO_COLOR + always:   %s lines with colour  (explicit --color wins)\n' "$(NO_COLOR=1 tsi --color always engines | count)"

stop "JSON: versioned, and says which command made it" \
  "Every document starts with schema_version and command."
if has_jq; then
  show "tsi optimize ... -o json | jq '{schema_version, command, total_mass_kg, stages: [.stages[] | {engine, engine_count, delta_v_mps}]}'"
  tsi optimize "${R[@]}" -o json |
    jq '{schema_version, command, total_mass_kg, stages: [.stages[] | {engine, engine_count, delta_v_mps}]}'
  show "tsi engines -o json | jq -r '.engines[] | .name'"
  tsi engines -o json | jq -r '.engines[] | .name' | paste -sd' ' -
else
  run_head 20 optimize "${R[@]}" -o json
fi

# ─────────────────────────────────────────────────────────────────────────
act "V. Errors"

stop "Garbage in, named error out"
run optimize --payload NaN --target-dv 9400 --engine raptor-2
run optimize --payload 5000 --target-dv 9400 --engine raptor-3
run calculate --isp 350

# ─────────────────────────────────────────────────────────────────────────
act "VI. The library underneath"

stop "tsiolkovsky is a library first" \
  "The CLI is a thin layer. The examples need no CLI feature at all."
show "sed -n '/fn main/,/^}/p' examples/quickstart.rs"
sed -n '/fn main/,/^}/p' examples/quickstart.rs
show "cargo run --example falcon9"
cargo run -q --release --example falcon9

stop "Where ideal theory stops" \
  "Validation tests compare against real rockets, and say where the model is" \
  "too optimistic (Saturn V: no gravity losses)."
show "cargo test --test validation"
cargo test -q --release --test validation 2>&1 | grep -E 'test result'

stop "What's next" "The roadmap, straight from cairn."
if command -v cairn >/dev/null; then cairn roadmap; else sed -n '1,40p' ROADMAP.md; fi
printf '\n\033[1;36mEnd of tour.\033[0m\n'
