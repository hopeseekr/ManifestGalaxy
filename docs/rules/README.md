# PR2 rules and evidence conventions

This directory records what PR2 recovered about Stars! and what remains
unknown. It is not a claim that the January 1998 help describes
`stars26i.exe` exactly. No original program was executed during PR1 or PR2.
No gameplay code is added by this PR.

The machine-readable rule inventory is [`rule-index.json`](rule-index.json).
The narrative ledger is [`rule-ledger.md`](rule-ledger.md). Turn-order
dependencies and unresolved phase relationships are in
[`turn-order.md`](turn-order.md), and the tracked blockers are in
[`gap-ledger.md`](gap-ledger.md).

Controlled operator scenarios live in
[`../evidence/scenarios/`](../evidence/scenarios/). They are measurement
plans, not observed results. Every scenario must validate with:

```sh
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
```

## Inputs and hashes

| Input | SHA-256 | Role |
|---|---|---|
| `stars26i.exe` | `5770dade2db0e11918631d5c128c0bf7ab48e371f7b2267a4ff02531bb8e198e` | Executable under study |
| `Stars! - RIP/stars!.hlp` | `9c5cb10fd5e5d201813fcf6f470c495f290b322ee21e1795418a8412393270d1` | January 1998 player guide |

Both files are supplied inputs and remain outside Git. Raw decoded help,
generated inventories, extracted segments and private notes stay under ignored
`analysis/`.

## Evidence statuses

Only these statuses are used:

- **observed** — read from a controlled original-game run. No PR2 scenario uses
  this status because no operator run was performed.
- **statically-derived** — read from the supplied bytes without execution, such
  as a dialog control style or a located string. This is not proof of runtime
  behavior.
- **documented-only** — stated by the decoded help. The statement may be wrong
  for this executable version.
- **inferred** — follows from combining evidence, with the inference stated.
  No current rule depends on an unlabelled inference.
- **unknown** — not recovered. Unknown records must not contain expected
  results.

A rule or scenario cannot claim a stronger status than its evidence supports.
In particular:

- a `documented-only` record needs a help-topic citation;
- a `statically-derived` record needs an executable citation;
- an `observed` record needs an actual observation record;
- an `unknown` record must not contain a fabricated expected outcome.

The validator enforces these rules, duplicate-ID rejection, hash matching and
coverage of every trait and rule by at least one scenario.

## Citation format

Executable citations identify the PR1 file offset and, where applicable, the
dialog/resource and control. Segment-relative offsets are never written as file
offsets. Example:

```text
stars26i.exe, dialog #149 at file offset 0x2fa900, control 271, style 0x50020009
```

Help citations identify `TOPICPOS`, the absolute HLP file offset and the topic
number/title. PDF page numbers are not used because they depend on layout.
Example:

```text
stars!.hlp, TOPICPOS 1736867 (0x001a80a3), HLP file offset 437912 (0x0006ae98), topic 258 'Hyper-Expansion'
```

The exact locations are also stored in each scenario's `provenance` array. The
validator checks the cited input hashes and citation format, but it does not
resolve the offsets or topic positions in the source files.

## Reproducing the static checks

The PR1 inventory tools produce the decoded dialogs and help topics:

```sh
python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json
python3 tools/analysis/help.py 'Stars! - RIP/stars!.hlp' \
  --output analysis/help/inventory.json \
  --text-output analysis/help/topics.txt
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
```

The two static observations used most often in this ledger can also be checked
directly:

```sh
python3 - <<'PY'
from pathlib import Path
import json

data = Path('stars26i.exe').read_bytes()
for needle in [b'Hyper-Expansion', b'Super-Stealth', b'Turindrones',
               b'Improved Fuel Efficiency', b'Total Terraforming']:
    offsets = []
    start = 0
    while True:
        found = data.find(needle, start)
        if found < 0:
            break
        offsets.append(hex(found))
        start = found + 1
    print(needle.decode('ascii'), offsets)

index = json.loads(Path('analysis/inventory.json').read_text())
for dialog in index['dialogs']['resources']:
    name = dialog['resource']['id']['display']
    if name in {'#149', '#150', '#151'}:
        print(name, dialog['file_offset_hex'])
        for control in dialog['controls']:
            print(control['id'], control['text'], control['style_hex'])
PY
```

Expected static facts:

- Dialog `#149` has ten primary-trait auto-radio buttons. Control 271
  (`Hyper-Expansion`) has style `0x50020009`; controls 272-280 have style
  `0x50000009`. The low word `0x0009` is `BS_AUTORADIOBUTTON`, and control 271
  starts the `WS_GROUP` group. The dialog therefore statically permits at most
  one selected primary trait. The product requirement of exactly one primary is
  a PR5 design rule; the original runtime requirement and default selection are
  not observed.
- Dialog `#150` has fourteen auto-checkboxes (style `0x50010003`, low word
  `0x0003` = `BS_AUTOCHECKBOX`) with empty captions. It statically permits zero
  or more independent lesser selections, but the checkbox-to-trait mapping is
  unresolved.
- Dialog `#151` has eighteen auto-radio buttons in six three-choice groups,
  plus one auto-checkbox. This matches six research fields with three cost
  choices and an all-fields starting-tech option.
- Exact-byte search finds the ten primary-trait names in dialog `#149`, but
  finds no literal ASCII names for the fourteen lesser traits in the executable.
  The lesser names are confirmed only in decoded help and by the fourteen
  unnamed checkbox controls. Do not invent a mapping from tab order alone.

These are static leads. A dialog caption or control style does not prove which
code path runtime uses, nor does it recover point costs, exclusions or numeric
effects.

## Adding a rule or scenario

1. Use a stable ID. Rules use `RULE-<FAMILY>-<NAME>`; scenarios use
   `SCN-<FAMILY>-<NAME>-NNN`.
2. Add the rule to `rule-index.json` with a status, downstream PRs and at least
   one scenario ID.
3. Add a JSON scenario under `docs/evidence/scenarios/` with a citation,
   baseline/variable setup, reproducible procedure, expected claims (only for
   documented/static/observed statuses) and unresolved questions.
4. Run the validator. Do not mark an observation as complete unless an operator
   actually ran it and recorded the environment and trials.
5. If the rule is a required gameplay rule and remains unknown, add or update a
   gap in `gap-ledger.md` rather than substituting a guessed formula.

The help text and images are copyright Mare Crisium, LLC. Summarize factual
claims with citations; do not commit bulk decoded text, images or PDFs.
