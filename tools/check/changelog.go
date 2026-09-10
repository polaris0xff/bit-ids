// changelog.go - four rules over CHANGELOG.md, one per entry.
//
//  1. newest first, WITHIN a section. "Unreleased" sitting above "1.0.0" is
//     correct, so the ordering resets at every `## ` heading and comparing across
//     one would report a correct file as backwards.
//  2. a date in the heading. Nothing can order an entry without one.
//  3. a record. An entry naming no record is a claim.
//  4. a word about deployment. Silence is not an answer.
//
// ⛔ AND A CHANGELOG THAT PARSES TO NO ENTRIES IS A FAILURE, NOT A CLEAN RUN.
// All four rules are per entry, so a file whose headings the parser does not
// recognise satisfies every one of them by having nothing to check. This
// repository shipped exactly that: the file used `## ` for what the parser reads
// as a section, so from the bootstrap until somebody noticed, the changelog was
// never checked at all and said it was.
//
// ⚠ An ABSENT file is a different answer and stays exit 2. A project with no
// changelog has neither broken these rules nor satisfied them.
//
// Ported from scripts/common/check-changelog.sh and its PowerShell twin.
package main

import (
	"fmt"
	"os"
	"regexp"
	"strings"
)

// dateRe accepts a full ISO 8601 UTC stamp and a bare date alike. ⚠ Both order
// as strings only because ISO 8601 is designed to, which is why the format is a
// rule rather than a preference.
var dateRe = regexp.MustCompile(`[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9](T[0-9][0-9]:[0-9][0-9]:[0-9][0-9]Z)?`)

func checkChangelog(r *repo) (verdict, error) {
	const file = "CHANGELOG.md"
	raw, err := os.ReadFile(file)
	if err != nil {
		return verdict{}, errCannotRun("no " + file + " in this repository. That is \"could not run\", not \"passed\"")
	}

	problems := 0
	entries := 0
	var body strings.Builder

	prev := ""
	entryLine := 0
	started := false
	hasRecord := false
	hasDeploy := false

	flush := func() {
		if !started {
			return
		}
		if !hasRecord {
			fmt.Fprintf(&body, "  %s: the entry at line %d names no record. An entry with no record is a claim.\n", file, entryLine)
			problems++
		}
		if !hasDeploy {
			fmt.Fprintf(&body, "  %s: the entry at line %d does not say whether it deployed. Silence is not an answer.\n", file, entryLine)
			problems++
		}
		started = false
	}

	for i, lineB := range splitLines(raw) {
		line := string(lineB)
		nr := i + 1

		switch {
		case strings.HasPrefix(line, "## "):
			flush()
			// ⚠ prev resets per SECTION, which is rule 1's whole subtlety.
			prev = ""

		case strings.HasPrefix(line, "### "):
			flush()
			entries++
			entryLine = nr
			d := dateRe.FindString(line)
			if d == "" {
				fmt.Fprintf(&body, "  %s:%d no date in the heading. Nothing can order it.\n", file, nr)
				problems++
			}
			if d != "" && prev != "" && d > prev {
				fmt.Fprintf(&body, "  %s:%d out of order: %s comes after %s. Newest first.\n", file, nr, d, prev)
				problems++
			}
			if d != "" {
				prev = d
			}
			hasRecord = false
			hasDeploy = false
			started = true

		case started:
			low := strings.ToLower(line)
			if strings.Contains(low, "record:") {
				hasRecord = true
			}
			if strings.Contains(low, "deploy") {
				hasDeploy = true
			}
		}
	}
	flush()

	if entries == 0 {
		return verdict{
			code: 1,
			stderr: fmt.Sprintf("changelog check failed: %s has no entries this parser recognises.\n\n", file) +
				"An entry heading is \"### \", a section heading is \"## \", and a file of\n" +
				"section headings alone passes every per-entry rule by having nothing\n" +
				"to check. docs/conventions/docs.md carries the shape.\n",
		}, nil
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-changelog/1","problems":%d,"entries":%d}`, problems, entries),
	}
	if problems > 0 {
		v.code = 1
		v.text = fmt.Sprintf("changelog check failed, %d problem(s):\n\n%s\n\n", problems, body.String()) +
			"The rules are in docs/conventions/docs.md. ⛔ Fix the entry; do not\n" +
			"reorder the whole file in the commit that adds to it. Tidying is its\n" +
			"own commit, or both become unreviewable.\n"
		return v, nil
	}
	v.text = fmt.Sprintf("changelog ok: %d entries, in order, each dated with a record and a deploy line\n", entries)
	return v, nil
}
