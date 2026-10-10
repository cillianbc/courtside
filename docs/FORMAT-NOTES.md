# .dvw / .vsm format notes

Everything learned about the formats, each entry with the corpus file(s) that show it.
When Courtside and openvolley disagree, check the file by hand, decide who is right, and record it here.

File references are relative to `testdata/`. `corpus-file` = `dvw/datavolley4/corpus-file`, `corpus-file` = `dvw/datavolley4/corpus-file`, `corpus-file` = `corpus-file`, `corpus-file*`, `corpus-file*`, `Ireland v *` = files in `vsm/`.

## Sections

- All 18 seed files have the same 14 sections in the same order: `3DATAVOLLEYSCOUT`, `3MATCH`, `3TEAMS`, `3MORE`, `3COMMENTS`, `3SET`, `3PLAYERS-H`, `3PLAYERS-V`, `3ATTACKCOMBINATION`, `3SETTERCALL`, `3WINNINGSYMBOLS`, `3RESERVE`, `3VIDEO`, `3SCOUT`. No lines before the first header. (all)
- Empty sections are just the header line followed by the next header (`3ATTACKCOMBINATION`, `3SETTERCALL`, `3RESERVE`, `3VIDEO` in corpus-file, corpus-file).
- `[3SET]` always has 5 rows; unplayed sets are `True;;;;;;`. Partial scores may be space-padded: `8 -7` (corpus-file), ` 7- 8` (corpus-file).
- `[3VIDEO]` line is `Camera0=<absolute local path>` (corpus-file). That path is personal data; don't echo it into exports by default.

## Header ([3DATAVOLLEYSCOUT])

- `FILEFORMAT: 2.0` in all files.
- `GENERATOR-PRG: Data Volley` / `GENERATOR-REL: Release 4.2023.03` (corpus-file) and `Release 4.2025.01` (corpus-file), `GENERATOR-VER: Media`.
- VolleyStation writes `GENERATOR-PRG: VolleyStation`, `GENERATOR-VER: Professional`, empty `REL` and `NAM`, keeping the trailing space after the colon (`GENERATOR-REL: `). Must round-trip that space. (corpus-file)
- Date format differs by generator: DataVolley `MM/DD/YYYY` (corpus-file, corpus-file), VolleyStation `DD/MM/YYYY` (corpus-file). Same in `[3MATCH]` field 1. Ambiguous for day ≤ 12; decide by `GENERATOR-PRG`, not by guessing.
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

- Columns (0-based), per openvolley's `read_main`; full table in `crates/dvw-codec/src/scout.rs`: 0 code, 1 point phase (`p`/`s`), 2 attack phase (`r`/`s`/`p`), 3 ?, 4–6 start/mid/end coordinate (`-1-1` = none), 7 time `HH.MM.SS`, 8 set, 9–10 home/visiting setter position, 11 video file number, 12 video seconds, 13 ?, 14–19 home players p1–p6, 20–25 visiting p1–p6, then a trailing `;`.
- Parity: all of these except 3, 8–10 and 13 match openvolley (datavolley 1.18.5, `c231092`) exactly on every line of all 18 seed .dvw files (`tests/parity.rs`). openvolley doesn't expose 8–10 raw (it derives set number and setter positions from the codes), and ignores 3 and 13.
- DataVolley Media files have no video column values (no video). VolleyStation fills a video-seconds column (`236`, `244`, … in corpus-file).
- Set end line: `**3set;...` with on-court columns empty (corpus-file, corpus-file).
- Unknown-player attacks/blocks coded `$$&H#` (team prefix `*` or `a`).

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
  - none of these (6 events; only lineup + score; meaning unknown, look at them)
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

## Scout code grammar (vb-grammar)

- Skill code: team (`*`/`a`), player digits or `$$` (unknown), skill `S R E A B D F`, then 11 fixed positions (type, evaluation, combination ×2, target, start zone, end zone, end subzone, subtype, num players, special), each a character or `~` = empty, then free custom text from position 16. Trailing positions may be omitted (`a10AH#`) or written as `~` (`*19SM+~~~`); both round-trip.
- `$$&` is a "green code" (point credited without a skill). openvolley skips these entirely (no skill, player or evaluation), so parity compares only their team.
- Non-skill codes: `*p25:20` point (home:visiting after the point), `*z1` setter position, `*P08` setter player, `*c06:07` substitution (out:in), `*T` timeout, `**1set` set end. `>LUp` suffix on `P`/`z` marks the starting lineup.
- openvolley counts a setter-player code without `>LUp` (e.g. `aP11`, 196 in the seed corpus incl. LUp ones) as a substitution. Parity mirrors that; the engine will decide what it means for rotation.
- Seed corpus coding is shallow: 10,682 of 10,871 skill codes are 6 characters (`a10AH#`), the rest only add subtype and num players (`~~~~~~H2`). No combinations, zones or custom text yet, so the generated property tests carry that coverage until richer files arrive.

## Generator quirks

## Disagreements with openvolley

- **Date serial is off by one in openvolley.** `read_match` converts the `[3MATCH]` line-2 serial as `as.Date("1900-01-01") + serial - 1`, which gives 2026-08-01 for `46234`. The correct epoch is 1899-12-30 (the spreadsheet convention): `46234` = 2026-07-31, matching the written date in all 17 DataVolley seed files (corpus-file, corpus-file). Courtside uses 1899-12-30. openvolley only uses the serial to disambiguate the written date, so its final dates may still be right; check when the parity harness lands.
