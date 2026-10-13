# .dvw / .vsm format notes

Everything learned about the formats, each entry with the corpus file(s) that show it.
When Courtside and openvolley disagree, check the file by hand, decide who is right, and record it here.

File references are relative to `testdata/`. `corpus-file` = `dvw/datavolley4/corpus-file`, `corpus-file` = `dvw/datavolley4/corpus-file`, `corpus-file` = `corpus-file`, `corpus-file*`, `corpus-file*`, `Ireland v *` = files in `vsm/`.

## Sections

- All 18 seed files have the same 14 sections in the same order: `3DATAVOLLEYSCOUT`, `3MATCH`, `3TEAMS`, `3MORE`, `3COMMENTS`, `3SET`, `3PLAYERS-H`, `3PLAYERS-V`, `3ATTACKCOMBINATION`, `3SETTERCALL`, `3WINNINGSYMBOLS`, `3RESERVE`, `3VIDEO`, `3SCOUT`. No lines before the first header. (all)
- Empty sections are just the header line followed by the next header (`3ATTACKCOMBINATION`, `3SETTERCALL`, `3RESERVE`, `3VIDEO` in corpus-file, corpus-file).
- `[3SET]` always has 5 rows; unplayed sets are `True;;;;;;`. Partial scores may be space-padded: `8 -7` (corpus-file), ` 7- 8` (corpus-file). DataVolley pads only the home score to two characters (`{home:<2}-{visiting}`: `8 -7`, `16-7`, `7 -10`); VolleyStation right-aligns both (` 7- 8`, `12-16`). Partial scores are the score after the point on which either team first reaches 8, 16 and 21, or 5, 10 and 12 in a deciding set (`3 -5;7 -10;7 -12;11-15`, corpus-file set 5). Unplayed sets may carry a duration of `0` (`True;;;;;0;`, corpus-file). vb-engine regenerates the scores and `[3TEAMS]` sets won from the fold and reproduces them byte-exact on all 18 seed .dvw files (`dvw_round_trips_through_the_engine`).
- `[3VIDEO]` line is `Camera0=<absolute local path>` (corpus-file). That path is personal data; don't echo it into exports by default.

## Header ([3DATAVOLLEYSCOUT])

- `FILEFORMAT: 2.0` in all files.
- `GENERATOR-PRG: Data Volley` / `GENERATOR-REL: Release 4.2023.03` (corpus-file) and `Release 4.2025.01` (corpus-file), `GENERATOR-VER: Media`.
- VolleyStation writes `GENERATOR-PRG: VolleyStation`, `GENERATOR-VER: Professional`, empty `REL` and `NAM`, keeping the trailing space after the colon (`GENERATOR-REL: `). Must round-trip that space. (corpus-file)
- Date order seen: `MM/DD/YYYY` in the DataVolley files (corpus-file, corpus-file), `DD/MM/YYYY` in the VolleyStation one (corpus-file). Same in `[3MATCH]` field 1. Ambiguous for day ≤ 12. The order does **not** follow from `GENERATOR-PRG`: DataVolley writes the scouting PC's Windows locale and VolleyStation a per-season setting (DD/MM, MM/DD or YYYY/MM/DD) (docs/research). So the corpus orders are those PCs' settings, not rules. Dates in one file can even differ in order: corpus-file and corpus-file have `GENERATOR-DAY` day-first (`01/08/2026 12:04:59`, `:` in the time) but `LASTCHANGE-DAY` and the match date month-first (`08/01/2026 13.43.58`), so the file was created on one PC and saved on another. `Dvw::match_date` therefore never borrows another date's order: serial first, then the written date if only one order is valid (`Date::parse_written`, strict: `A/B/YYYY` or `YYYY/MM/DD`, one separator), then the one reading within 14 days of an unambiguous `GENERATOR-DAY`/`LASTCHANGE-DAY`; otherwise none, and `courtside validate` warns that the order is ambiguous. Every corpus file resolves (17 by serial, corpus-file unambiguous).
- `[3MATCH]` line 2, field 3 (DataVolley only) is a spreadsheet date serial, day 0 = 1899-12-30: `46234` = 2026-07-31. It matches the written date (read month-first) in all 17 DataVolley files, so it is the most reliable date source. `12345` is a known placeholder (per openvolley). Empty in VolleyStation exports (corpus-file). `Dvw::match_date` uses serial → generator order → unambiguous parse, in that order.

## Encodings

- `[3MATCH]` field 9 is the Windows code page: `1252` in all files.
- 13 files are pure ASCII. 5 contain raw cp1252 bytes in player names, which are invalid UTF-8: `í` = 0xED (corpus-file), `ò` = 0xF2 (corpus-file, -05, -08, -10). Tools that sniff UTF-8 (grep, `file`) treat these as binary.
- DataVolley 4 stores a Unicode copy of text fields: `\x0F` (invisible control byte), a digit `n` (`2` or `4`), then `n` hex digits per character, each a **Unicode code point** (not a cp1252 byte; they coincide below U+0100). E.g. first name `Siobhán` → `\x0F253696F6268E16E`; empty → `\x0F2`. All 1,536 such fields in the seed corpus use width 2 and decode to exactly the plain column (verified in `typed_headers_are_sane`). Columns: `[3MATCH]` 12–13 (league, phase; openvolley also reads 14), `[3TEAMS]` 6–8 (name, coach, assistant), `[3MORE]` 6–9 (referees, city, arena, scout), `[3PLAYERS-*]` 17–19 (last, first, nickname). 0-based. VolleyStation omits them (corpus-file).
- Reading prefers the Unicode copy when it decodes to non-empty text, else the plain column (same as openvolley). Writing must keep both in step.
- Tokeniser works on bytes, so encoding is irrelevant to round-trip; decoding is task 4.

## Line endings

- DataVolley: CRLF throughout, trailing CRLF at EOF (corpus-file, corpus-file).
- VolleyStation: LF throughout, **no** trailing newline at EOF (corpus-file).
- Tokeniser stores each line's terminator, so both round-trip byte-exact.

## Field content

- Literal tab characters inside fields: `Surname Forename<TAB>` (coach) in `[3TEAMS]`, `Forename<TAB>` in `[3PLAYERS-H]` (corpus-file). Never trim fields.
- Competition names can contain `|` (corpus-file: `CEV U20 ... 2024 | Men - Pool in IRL`).
- `[3PLAYERS-*]` columns (0-based): 0 team (0/1), 1 shirt number, 2 index, 3–7 starting zone in sets 1–5 (`1`–`6`, `*` = came on as sub, empty = did not play), 8 player id, 9 last name, 10 first name, 11 nickname, 12 special role (`L`, `C`), 13 role, 14 foreign (`True`/`False`/empty), 15 player id again (DataVolley), 16 ?, 17–19 Unicode copies. VolleyStation rows stop after column 15 (corpus-file).
- Roles: 1 libero, 2 outside, 3 opposite, 4 middle, 5 setter, 6 unknown (openvolley's name; one row in corpus-file). Also seen: empty (corpus-file, corpus-file) and the literal string `NaN` from VolleyStation for players with no role (corpus-file, 4 rows; a JavaScript NaN leaking into the export). Treat empty and `NaN` as "no role"; round-trip them as written.
- Placeholder player rows exist: id `unknown-4`, last name `Unknown Player 4`, and the same **plain** text (no `\x0F`) in column 17 (corpus-file). Not a DataVolley pattern; probably added by other software. The embedded-text decoder ignores fields without the `\x0F` prefix, so it falls back to the plain name.
- `[3MATCH]` line 1 (0-based): 0 date, 1 time, 2 season, 3 league, 4 phase, 5 home/away, 6 day number, 7 match number, 8 code page, 9 regulation (0 indoor sideout, 1 indoor rally point, 2 beach), 10 `Z`/`C` (zones or cones), 11 `0` in DataVolley, 12–14 Unicode copies.
- `[3MATCH]` line 2 (DataVolley): `2588;134;46234;;;;L;R;VIR;52515;` = federation match id, competition id, date serial, ?, ?, ?, `L`/`R` and `R`/`L` (court sides?), federation code (`VIR`, `CEV`), federation id. Only the serial is interpreted.
- `[3TEAMS]` (0-based): 0 id, 1 name, 2 sets won, 3 coach, 4 assistant, 5 shirt colour (Windows COLORREF int), 6–8 Unicode copies. Sets won equals the number of scored `[3SET]` rows in every seed file.

## [3SCOUT] lines

- Columns (0-based), per openvolley's `read_main`; full table in `crates/dvw-codec/src/scout.rs`: 0 code, 1 point phase (`p`/`s`), 2 attack phase (`r`/`s`/`p`), 3 action (rally) id, 4–6 start/mid/end coordinate (`-1-1` = none), 7 time `HH.MM.SS`, 8 set, 9–10 home/visiting setter position, 11 video file number, 12 video seconds, 13 DataVolley's transient "matches the current search" flag, 14–19 home players p1–p6, 20–25 visiting p1–p6, then a trailing `;`.
- Columns 3 and 13 are empty on every line of all 18 seed files; the meanings come from DataVolley's writer (docs/research/DATAVOLLEY4.md). Write them empty.
- Column 11 is empty in every DataVolley file and `1` on all 822 lines of corpus-file (VolleyStation always writes `1`).
- Parity: all of these except 3, 8–10 and 13 match openvolley (datavolley 1.18.5, `c231092`) exactly on every line of all 18 seed .dvw files (`tests/parity.rs`). openvolley doesn't expose 8–10 raw (it derives set number and setter positions from the codes), and ignores 3 and 13.
- DataVolley Media files have no video column values (no video). VolleyStation fills a video-seconds column (`236`, `244`, … in corpus-file).
- Set end line: `**3set;...` with on-court columns empty and column 8 already the **next** set (all 18 files: `**1set` shows `2`, `**5set` shows `6` in corpus-file, corpus-file, corpus-file).
- `$$&H#` / `$$&H=` (team prefix `*` or `a`) are green codes: a point credited to (`#`) or charged against (`=`) a team without a player touch, e.g. `*17AH#` then `a$$&H=` then `*p05:03` (corpus-file). Every seed file has them: 87–181 per DataVolley file, 107 in corpus-file, where VolleyStation also writes them for a rally with a winner but no rally-ending play and for each unit of an exported score correction. They are never player touches: the validator and stats skip them.

## .vsm (VolleyStation native)

Seed: 18 files in `vsm/`, all VolleyStation, 2023-01 to 2024-03.

### Container

- Compact UTF-8 JSON on one line: no BOM, no whitespace, no trailing newline. Non-ASCII is raw UTF-8 (`è`, `ü`, `’`), control chars escaped (`\t`). (all)
- Parsing and re-serialising compactly **with key order preserved** reproduces every file byte-exact. So the lossless layer is `serde_json::Value` with `preserve_order` (`dvw_codec::RawVsm`). Key order is not alphabetical and varies between objects of the same kind (e.g. plays put `hitType` before or after `team`), so never re-serialise through a typed struct for round-trip.
- Top level: `gameType` (`"indoor"`), `version` (`1` in all), `startDate` (ISO 8601 UTC), `scout`, `team`, `attackCombinations`, `setterCalls`; optional `createdAt` (9 files), `comment` (5 files; free-text coach notes, may contain `\n`).
- `scout` = `{sets, video?}`. `video` = `{path}`, a local absolute path; missing in 1 file.

### Teams

- `team.home|away`: `name`, `shortCode`, `code`, `libero` (array of shirt numbers, 1–2 seen), `players`, `staff` (`coach`, `assistantCoach`, `assistantCoach2`, `therapist`, `medicalDoctor`, each `{firstName, lastName}`).
- Player: `_id`, `firstName`, `lastName`, `shirtNumber` (int), `code`; optional `position` (int), `nickname`, `isForeign`.
- Trailing tabs inside names here too (`"firstName":"…\t"`). Same source data as the tab-in-field quirk in corpus-file's .dvw export.

### Sets and events

- `scout.sets[]`: `_id`, `startingLineup`, `events`, `score` (`{home, away}` final).
- Lineup: `{home|away: {positions: {"1".."6": shirtNumber}, setterAt: 1..6}}`. Position keys are strings, values ints.
- Each event has `_id`, `lineup` (state after the event), `score` (after), and exactly one of:
  - `exchange` = one rally: `{plays: [...], point: "*"|"a"}` (2,677 events)
  - `substitution` = `{team, playerIn, playerOut}` (ints) (254)
  - `timeout` = `{team}` (164)
  - none of these (6 events, only lineup + score): corrections. One changes a lineup (`corpus-file` set 3), four change the score by one point (`corpus-file` set 2 twice and set 3, `corpus-file` set 4), one changes nothing (`corpus-file` set 3). Matches VolleyStation's model, where an event with only score and lineup is a score, rotation or setter correction (docs/research/VOLLEYSTATION.md).
- `_id`s are 24-hex MongoDB ObjectIds (timestamp in the first 4 bytes).

### Plays (`exchange.plays[]`)

- Always: `_id`, `team` (`*`/`a`), `player` (**string**, zero-padded `"03"`), `skill`, `hitType`, `effect`.
- Usually: `time` (all but 2), `originalTime` (most), `code` (95%).
- Optional: `skillType`, `startZone`, `endZone` (ints), `startSubZone`, `endSubZone` (`A`–`D`), `combination`, `targetAttacker`, `players` (int; blockers etc.), `special`, `custom`, `travelPath` (`[{x, y}, ...]` court coordinates).
- Naming vs DataVolley: `hitType` = DV skill type (position 5), `effect` = evaluation, `skillType` = DV skill subtype, `players` = num_players.
- **`code` is the keystrokes the scout typed, not the expanded DV code**: `7S.`, `.15A!` (dot/compound entry), as well as full forms like `a01RH-`. Missing on some plays. It cannot be used as the DV code; build the DV code from the fields.
- `time` vs `originalTime`: different scales (`4551` vs `6932022` on one play). Units unknown; probably video offset vs wall clock. Check against a video before relying on either.
- Values seen. skill: A S R D B E F. hitType: H M T Q O N U. effect: `- # + = / !`. skillType: mostly H, T, P, then rare `3 A B E C 2 R`. special: I Z S P A C U.

### Re-saved pairs

- `corpus-file`/`Ireland v Luxembourg`, `corpus-file`/`Ireland v Malta`, `corpus-fileor`/`Ireland v Northern Ireland` are the same match saved twice. The later file has real post-match edits: changed `effect`/`skill`/`player`, plays added and removed, `_id`s regenerated on some plays. Good test data for the Correction model and for import diffing.

## Engine fold (vb-engine)

What DataVolley and VolleyStation write in the derived `[3SCOUT]` columns (8 set, 9–10 setter positions, 14–25 players on court), reproduced by `vb_engine::fold` on every corpus line except the two glitches below (`crates/vb-engine/tests/corpus.rs`).

- Columns show the state **after** the line. The `**1set` line already shows set 2, with all on-court columns empty and the setter positions carried over.
- A set's lineup comes from the `[3PLAYERS-*]` starting zones for that set, from the first line after the set end (the `>LUp` lines already show it). Setter positions on `>LUp` lines before the `z` code are leftovers: the previous set's (corpus-file set 2), or something else (`6`/`5` on corpus-file line 0, `0`/`0` on corpus-file line 0). The engine treats them as unknown.
- `z` without `>LUp` rotates the court so the current setter (last `P` code) is in that zone. It is usually one step after a side-out (1,307 of 1,407), but scouts also re-enter `z` to fix a rotation: 0, back one or back two steps (corpus-file, corpus-file). So the `z` code is authoritative and the rotation follows the setter, not a fixed step; the validator checks it against the side-out rule (see "Validator").
- `c` swaps a player in place. Media files never move the libero in these columns.
- DataVolley writes a setter change after a rotation (`*z4`, `*P8`, note no zero padding) meaning the rotation refers to the old setter (corpus-file, corpus-file, -04, -06, -07). VolleyStation writes `*z2`, `*P21` meaning the rotation refers to the **new** setter (corpus-file line 358). `vb_engine::Conventions::setter_change_follows_rotation`, set on import from `GENERATOR-PRG`.
- VolleyStation writes a rotation line with the substitutions straight after it already applied (corpus-file: `*z4` then `ac07:03`; the `z` line already shows 3 for 7).
- Export (`Match::to_dvw`) writes these columns from the fold in DataVolley's convention, so the VolleyStation rotation lines (7 in corpus-file) and both glitches below come out corrected; every other line is written back unchanged.
- Glitch: setter position `0` for ten lines of corpus-file set 3, with no code changing it.
- Glitch: corpus-file home on-court columns for set 1 (274 lines) contradict the player table's set 1 starting zones (three zones empty, a player with no starting zone on court). The table was probably edited after scouting. The engine follows the table.

## Validator (vb-engine)

What the six rules (`vb_engine::validate`) report on the corpus, and why (`validator_runs_on_the_corpus`; set `COURTSIDE_VALIDATOR_DETAIL=<text>` to list the lines).

- **Liberos are never in the six.** Both products keep the libero out of the on-court columns and exempt the team's liberos from "player not on court" (docs/research). Liberos are `L` in the special-role column; DataVolley also accepts role `1`, VolleyStation only `L`. In the corpus every role-1 player is also `L` (43 rows), so the two readings agree.
- **Libero rule:** a libero attack or block is an error (both products), a libero serve a warning (an error in DataVolley unless an option allows it; VolleyStation allows it). Seen: libero attacks in corpus-file, corpus-file (home 2, line 295, not on court), corpus-file; a libero block in corpus-file line 111.
- **Libero on court = team-sheet mislabel.** corpus-file home 10 is marked `L` but has starting zones 5 and 6 in sets 1–2 and serves and attacks from the six. Reported once per set as a warning, not on every touch.
- **"Player N is not on court"** (52 left after the exemptions; none are liberos) are scouting slips, not engine errors: a wrong shirt number (corpus-file line 633: `*12SH#` with 15 in zone 1, the next serve is `*15SH=`) or a missing substitution code (corpus-file set 3: the team sheet has 5 coming on, no `c` brings them on; set 2 line 507 `*c15:08` puts 8 on court twice). The message says which by checking the team sheet's starting zones for that set. Players missing from the team sheet get only the Code error.
- **Setter-position changes away from a side-out step are corrections, not mistakes.** DataVolley renumbers `z` codes after a setter change and scouts re-enter `z` by hand; VolleyStation writes a `z` for every setter-zone change (docs/research). Typical corpus case, a double switch: `*c13:08`, `*c21:04`, `*P4`, `*z1` (corpus-file line 282; also corpus-file, corpus-file). Only the `z` codes straight after the point code count as the side-out step: if they contradict the side-out rule it is a warning (10 in the corpus: the wrong team rotated, e.g. corpus-file lines 882 and 887, or the knock-on of a missing point code, corpus-file line 79); any other change is `Info` (36). "Won a side-out but didn't rotate" stays an error.
- What remains as errors looks like real scouting slips: missing point codes (second serve in one rally), skipped scores, missed rotations, wrong team on the point code.
- **Block pairing (task 7):** a blocked attack (`A/`) must be followed by the other team's block point (`B#`), and a `B#` preceded by the other team's `A/` (warnings, ServeReception). Both products check this (docs/research); openvolley's `dv_validate` does not. Corpus: 6, all real slips (`a11AH/` followed by the opponent's set in corpus-file; `a13AH+ a10BH#`, a block coded for the attacking team, in corpus-file).
- **`R/` is not "opponent wins".** The research's other pairing check (DataVolley offers to fix `R/` when the opponent didn't win) would flag 22 corpus rallies that are fine: an overpass the serving team then attacks into the net or out (`*03SH/ a03RH/ *12AH=`, corpus-file; also VolleyStation's corpus-file). Not adopted.
- **Knock-ons are Info.** An issue that follows from an earlier one is downgraded to Info with `cause` pointing at it (`courtside validate --info` prints "follows from line N"): after a point code given to the wrong team, the next rally's rotation issues and set-end issues that giving the point to the other team would fix; after a score error, a score issue on the next point code; after "won a side-out but didn't rotate", that team's rotation issues until the scout's next `z` correction. Rotation issues after a score-only error are not knock-ons (the score doesn't change who won).

### Bad-file set (task 7)

`crates/vb-engine/tests/bad_files` plants one slip per copy of a corpus .dvw (recipes in local `testdata/bad/recipes.toml`; the engine writes the file, so derived columns, `[3SET]` and `[3TEAMS]` are what DataVolley would have written) and compares Courtside with openvolley's `dv_validate` (validation level 3, `tools/parity/run.sh validate`). Terms in CONTEXT.md: a hit is a new Warning/Error between the slip's line and the next rally's serve; noise is any other new issue (issues already reported at the mapped line of the clean source don't count either way). datavolley 1.18.5, 30 files:

| Slip class | Courtside recall | dv_validate recall | Courtside noise | dv_validate noise |
|---|---|---|---|---|
| missing point code | 3/3 | 3/3 | 0 | 10 |
| skipped score | 3/3 | 3/3 | 0 | 7 |
| missed rotation | 3/3 | 3/3 | 0 | 4 |
| wrong-team point (rest of set renumbered) | 3/3 | 0/3 | 1 | 5 |
| 7th substitution | 3/3 | 0/3 | 0 | 0 |
| illegal re-entry | 3/3 | 0/3 | 0 | 0 |
| set ends 25:24 / 15:14 | 3/3 | 0/3 | 0 | 0 |
| libero attack | 3/3 | 3/3 | 0 | 0 |
| back-row block | 3/3 | 3/3 | 0 | 0 |
| `A/` written `A-` (opponent `B#` unpaired) | 3/3 | 0/3 | 0 | 0 |
| **Total** | **30/30** | **15/30** | **1** | **26** |

- Before the pairing check and knock-on suppression: Courtside 27/30, noise 22.
- `dv_validate` gets a point attributed against the touches only when the point code's score is also wrong ("Point assigned to incorrect team or scores incorrect"); a consistently renumbered set passes. Most of its noise is its level-3 style checks and "match result summary does not match the points recorded" after a missing point code.
- The one Courtside noise issue: wrong-team renumbering carries a set past 25 (set ends 28:21), which the knock-on rule doesn't trace back.

## Scout code grammar (vb-grammar)

- Skill code: team (`*`/`a`), player digits or `$$` (unknown), skill `S R E A B D F`, then 11 fixed positions (type, evaluation, combination ×2, target, start zone, end zone, end subzone, subtype, num players, special), each a character or `~` = empty, then free custom text from position 16. Trailing positions may be omitted (`a10AH#`) or written as `~` (`*19SM+~~~`); both round-trip.
- `$$&` is a "green code" (point credited or charged without a player touch; see "[3SCOUT] lines"). openvolley skips these entirely (no skill, player or evaluation), so parity compares only their team.
- Non-skill codes: `*p25:20` point (home:visiting after the point), `*z1` setter position, `*P08` setter player, `*c06:07` substitution (out:in), `*T` timeout, `**1set` set end. `>LUp` suffix on `P`/`z` marks the starting lineup.
- openvolley counts a setter-player code without `>LUp` (e.g. `aP11`, 196 in the seed corpus incl. LUp ones) as a substitution. Parity and the JSON export's `substitution` column mirror that; vb-engine treats it as a setter change (see Decisions).
- Seed corpus coding is shallow: 10,682 of 10,871 skill codes are 6 characters (`a10AH#`), the rest only add subtype and num players (`~~~~~~H2`). No combinations, zones or custom text yet, so the generated property tests carry that coverage until richer files arrive.

## Disagreements with openvolley

- **Date serial is off by one in openvolley.** `read_match` converts the `[3MATCH]` line-2 serial as `as.Date("1900-01-01") + serial - 1`, which gives 2026-08-01 for `46234`. The correct epoch is 1899-12-30 (the spreadsheet convention): `46234` = 2026-07-31, matching the written date in all 17 DataVolley seed files (corpus-file, corpus-file). Courtside uses 1899-12-30. openvolley only uses the serial to disambiguate the written date, and its final `meta$match$date` equals `Dvw::match_date` on all 18 seed .dvw files (checked by hand against the pinned oracle), so the off-by-one never reaches its output.
- **Serve phase.** openvolley gives a serve `phase` "Reception" when it falls in the same team touch as a reception, which only happens when a point code is missing before it (corpus-file lines 75, 681, 719; corpus-file lines 544, 683, 1009; corpus-file line 273). A serve is always serve phase in Courtside. Everything else in openvolley's derived columns (set, scores, setter positions, rally and team-touch grouping, serving team, rally winner, winning attack, phase) matches on every touch (`crates/vb-engine/tests/parity.rs`). Courtside adopts openvolley's rule that a block straight after a reception-phase attack is in reception phase.

## Decisions

Agreed at the phase 1 review (task 8).

- **One JSON surface (task 8, 2026-10-13):** the plays, stats and validation JSON live in `courtside-ffi`; the CLI and the WASM build both call it, and `tools/wasm/smoke.mjs` checks they agree on the corpus. The WASM functions take file bytes and return JSON strings (no serde-wasm-bindgen), so the web side parses one documented shape.
- **Phase 2 validator gate (task 7, 2026-10-13):** on the bad-file set, Courtside's recall is at least `dv_validate`'s in every slip class and higher overall, and it adds no more noise. Recall alone would let a noisy validator win. `validator_beats_dv_validate` asserts it when the oracle output is current.

- **Setter code without `>LUp`** (`aP11`) is a change of setter in vb-engine and feeds rotation and setter-position tracking; it is not a substitution. The JSON export keeps openvolley's `substitution` flag for compatibility.
- **Lenient parsing.** The codec and grammar never reject a file: odd values are kept as written (`Code::Unrecognised`, `Role::Unknown`, unparsed dates) and `courtside validate` reports them. Rule checks belong to the validator and vb-engine, not the reader.
- **No `winnow`.** The formats are positional; hand-written parsers are simpler. Revisit only for a real grammar.
- **Default and dot codes** are scouting-input features (expanding shorthand while typing) and are built with the scouting screen, not the file reader.
- **Corpus:** next priority is fully coded DataVolley files (combinations, zones, subzones, custom codes). Other code pages, beach and VolleyStation with video are wanted but not yet available.
