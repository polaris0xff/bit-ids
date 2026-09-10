// onehome.go - one fact, one home: is any sentence living in two documents?
//
// The rule is docs/conventions/prose.md. A fact stated in two places is a fact
// that will be corrected in one of them, and the reader who finds the other has
// no way to tell which is current. ⚠ This repository has counted four of its own
// numbers going stale in prose for exactly that reason.
//
// ⛔ ROUTERS ARE EXEMPT AS A SET, NOT INDIVIDUALLY. A sentence shared only
// between routing documents is a pointer doing its job. A sentence shared
// between a router and anything else is the defect, which is why the exemption
// tests whether EVERY file carrying it is a router rather than whether any is.
//
// Ported from scripts/common/check-one-home.sh and its PowerShell twin. The awk
// pass is reproduced decision for decision; two places where a Go idiom would
// have changed the answer are marked.
package main

import (
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"
)

// minWords is the length below which a repeated phrase is not a fact.
//
// ⚠ A CONSTANT RATHER THAN A FLAG, for the reason the density ceiling is one: a
// threshold anybody can raise from a command line is a threshold that gets
// raised instead of met.
const minWords = 12

var mdRe = regexp.MustCompile(`\.md$`)
var historyRe = regexp.MustCompile(`^docs/history/`)
var sentenceSplitRe = regexp.MustCompile(`[.:!?]+[ \t]+`)
var linkTailRe = regexp.MustCompile(`\]\([^)]*\)`)
var tableOrHeadingRe = regexp.MustCompile(`^[ \t]*[|#]`)

// routers are the documents whose job is to point at other documents.
var routers = map[string]bool{
	"AGENTS.md":                true,
	"ROUTE.md":                 true,
	"docs/templates/AGENTS.md": true,
}

func checkOneHome(r *repo) (verdict, error) {
	files := r.matching(mdRe, historyRe)
	if len(files) == 0 {
		return verdict{}, errCannotRun("no markdown files in scope")
	}
	// ⚠ THE SCOPE IS ASSERTED BEFORE THE VERDICT. A run over fewer than two
	// documents cannot find a sentence in two of them, and reporting that as
	// clean is a pass over a question nobody asked.
	if len(files) < 2 {
		return verdict{}, errCannotRun(fmt.Sprintf("only %d file(s) in scope; nothing to compare", len(files)))
	}

	// sentence -> the set of files carrying it.
	seen := map[string]map[string]bool{}

	for _, f := range files {
		raw, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		var buf strings.Builder
		fence := false
		for _, lineB := range splitLines(raw) {
			line := string(lineB)
			if fenceRe.MatchString(line) {
				fence = !fence
				continue
			}
			if fence {
				continue
			}
			// A table row is not a sentence, and nor is a heading.
			if tableOrHeadingRe.MatchString(line) {
				continue
			}
			// An inline code span becomes a SPACE rather than nothing, which is
			// what joins the words either side into two words instead of one.
			line = codeSpanRe.ReplaceAllString(line, " ")
			line = strings.ReplaceAll(line, "[", " ")
			line = linkTailRe.ReplaceAllString(line, " ")
			buf.WriteString(" ")
			buf.WriteString(line)
		}

		for _, part := range sentenceSplitRe.Split(buf.String(), -1) {
			s := normalise(part)
			if s == "" || len(strings.Fields(s)) < minWords {
				continue
			}
			if seen[s] == nil {
				seen[s] = map[string]bool{}
			}
			// ⚠ A SET RATHER THAN A COUNT. The shell half ran `sort -u` over
			// (sentence, file) pairs before counting, so one sentence twice in
			// ONE document is not a duplicate. A port that counted occurrences
			// would report every repeated heading in a single file.
			seen[s][f] = true
		}
	}

	type dup struct {
		sentence string
		files    []string
	}
	var dups []dup
	for s, fs := range seen {
		if len(fs) < 2 {
			continue
		}
		allRouters := true
		names := make([]string, 0, len(fs))
		for f := range fs {
			names = append(names, f)
			if !routers[f] {
				allRouters = false
			}
		}
		if allRouters {
			continue
		}
		sort.Strings(names)
		dups = append(dups, dup{s, names})
	}
	// ⛔ SORTED, BECAUSE A MAP HAS NO ORDER. Two runs over one tree must produce
	// one report; the shell half iterated an awk array, whose order is
	// unspecified, and this is strictly the better behaviour rather than a
	// changed verdict - the count it reports is the same either way.
	sort.Slice(dups, func(i, j int) bool { return dups[i].sentence < dups[j].sentence })

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-one-home/1","problems":%d,"files":%d,"min_words":%d}`,
			len(dups), len(files), minWords),
	}
	if len(dups) > 0 {
		var b strings.Builder
		fmt.Fprintf(&b, "one fact, one home: %d sentence(s) appear in more than one document:\n\n", len(dups))
		for _, d := range dups {
			s := d.sentence
			if len(s) > 88 {
				s = s[:88]
			}
			fmt.Fprintf(&b, "  %q\n", s)
			for _, f := range d.files {
				fmt.Fprintf(&b, "      %s\n", f)
			}
			b.WriteString("\n")
		}
		b.WriteString("Keep the fact in the document that owns it and make the other a pointer.\n")
		b.WriteString("docs/conventions/prose.md, \"one fact, one home\".\n")
		v.code = 1
		v.text = b.String()
		return v, nil
	}
	v.text = fmt.Sprintf("one fact one home: %d documents, no sentence of %d+ words in two of them\n",
		len(files), minWords)
	return v, nil
}

// normalise lowercases, replaces everything outside [a-z0-9 ] with a space,
// squeezes runs of spaces and trims.
//
// ⛔ IT IS ASCII-ONLY ON PURPOSE. The shell half ran under LC_ALL=C, where
// tolower touches only A-Z; Go's strings.ToLower is Unicode-aware and would fold
// characters the shell half left alone. Since everything outside the class then
// becomes a space the two agree on this tree, and they would stop agreeing on a
// document nobody has written yet.
func normalise(s string) string {
	out := make([]byte, 0, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c >= 'A' && c <= 'Z' {
			c += 'a' - 'A'
		}
		if (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == ' ' {
			out = append(out, c)
			continue
		}
		out = append(out, ' ')
	}
	return strings.Join(strings.Fields(string(out)), " ")
}
