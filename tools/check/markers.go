// markers.go - only the five defined characters, and not too many of them.
//
// Two rules, one subject, one home. docs/conventions/prose.md is the rule and
// this is the machine behind it.
//
//  1. THE CHARACTER SET. Every tracked text file is ASCII, with the three prose
//     markers and the two status glyphs as the only exception.
//  2. THE DENSITY. A file carrying more markers than the ceiling below is
//     refused, because a page where every paragraph shouts has no markers at all.
//
// ⚠ IT IS A CONSTANT, NOT A FLAG. A ceiling anybody can raise from a command
// line is a ceiling that gets raised instead of met. A project that genuinely
// needs a different number edits the line below, which is a change somebody
// reviews.
//
// ⚠ WHAT IT CANNOT SEE. Density is a count, and a marker used wrongly is a
// reading: a status glyph carrying a rule, or a stop marker on a preference,
// passes this check and fails a review.
//
// Ported from scripts/common/check-markers.sh and its PowerShell twin. The awk
// pass that half ran is reproduced here decision for decision, and the two
// places it is easy to get subtly wrong are marked below.
package main

import (
	"bytes"
	"fmt"
	"os"
	"regexp"
	"strings"
)

// ceiling is markers per 100 non-blank lines.
//
// The number was measured on 2026-08-28 over the tracked markdown of three
// trees: 38.6 overall in the one that reads worst, 9.0 in this one, 8.6 in the
// third. It passes every file in the two that read well and refuses 7 of the 12
// files in the one that does not.
const ceiling = 30

// The five, as the bytes they are. U+26D4 stop, U+2B50 star, U+26A0 warning,
// U+2705 pass, U+274C fail.
var markerBytes = [][]byte{
	{0342, 0233, 0224},
	{0342, 0255, 0220},
	{0342, 0232, 0240},
	{0342, 0234, 0205},
	{0342, 0235, 0214},
}

var bomBytes = []byte{0357, 0273, 0277}

var fenceRe = regexp.MustCompile("^[ \t]*```")
var codeSpanRe = regexp.MustCompile("`[^`]*`")

func checkMarkers(r *repo) (verdict, error) {
	files := r.matching(markerTextRe, licensesRe)
	if len(files) == 0 {
		return verdict{}, errCannotRun("no text files in scope")
	}

	problems := 0
	markers := 0
	worst := 0
	worstFile := "-"
	var report strings.Builder

	for _, f := range files {
		raw, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		isMD := strings.HasSuffix(f, ".md")

		fileMarkers := 0
		nonBlank := 0
		fence := false

		for i, line := range splitLines(raw) {
			// ⚠ A LEADING BOM IS EXEMPT, AND ONLY A LEADING ONE. Stripped from
			// the first line of the file and nowhere else, so a BOM a merge left
			// in the middle of a file is still a finding.
			if i == 0 && bytes.HasPrefix(line, bomBytes) {
				line = line[len(bomBytes):]
			}

			// ⛔ THE DENOMINATOR COUNTS A BARE CARRIAGE RETURN AS NON-BLANK,
			// because awk's `[^ \t]` does and every .ps1 in this tree keeps CRLF.
			// A port that trimmed "\r" first would shrink the denominator of every
			// PowerShell file and move its density - a verdict changed by a
			// translation rather than by a rule.
			//
			// ⚠ AND IT IS A BYTE LOOP RATHER THAN A RUNE ONE. The shell half ran
			// under LC_ALL=C, which states that the pass is byte-oriented; a rune
			// scan would fold every malformed sequence into one replacement
			// character and answer differently on a file nobody has yet written.
			for _, c := range line {
				if c != ' ' && c != '\t' {
					nonBlank++
					break
				}
			}

			// Markers are counted BEFORE anything is stripped, because the
			// density rule is about what a reader sees on the page.
			stripped := line
			for _, m := range markerBytes {
				for {
					p := bytes.Index(stripped, m)
					if p < 0 {
						break
					}
					fileMarkers++
					stripped = append(append([]byte{}, stripped[:p]...), stripped[p+len(m):]...)
				}
			}

			// ⭐ THE SPECIMEN EXEMPTION, markdown only. A fenced block is skipped
			// entire and an inline code span is cut out, so a page can name the
			// character it bans. Outside markdown there is no exemption: a source
			// file has no reader who needs a specimen.
			//
			// ⚠ THE FENCE TEST READS THE ORIGINAL LINE, not the marker-stripped
			// one, exactly as the awk pass did.
			if isMD {
				if fenceRe.Match(line) {
					fence = !fence
					continue
				}
				if fence {
					continue
				}
				for {
					loc := codeSpanRe.FindIndex(stripped)
					if loc == nil {
						break
					}
					stripped = append(append([]byte{}, stripped[:loc[0]]...), stripped[loc[1]:]...)
				}
			}

			// Whatever survives must be ASCII. Report the first offender per
			// line; a line with one wrong character usually has several.
			//
			// ⭐ THE CODEPOINT IS DECODED AND REPORTED, not just the position.
			// U+2014 tells a reader exactly which character to search for.
			if cp, ok := firstNonASCII(stripped); ok {
				fmt.Fprintf(&report, "  %s:%d U+%04X is outside the five. docs/conventions/prose.md\n", f, i+1, cp)
				problems++
			}
		}

		markers += fileMarkers
		den := nonBlank
		if den < 1 {
			den = 1
		}
		// Integer arithmetic on purpose: a ceiling comparison does not need the
		// fraction, and the shell half could not have produced one.
		dens := fileMarkers * 100 / den
		if dens > worst {
			worst = dens
			worstFile = f
		}
		if dens > ceiling {
			fmt.Fprintf(&report, "  %s %d markers in %d non-blank lines, %d per 100. The ceiling is %d. docs/conventions/prose.md\n",
				f, fileMarkers, den, dens, ceiling)
			problems++
		}
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-markers/1","problems":%d,"files":%d,"markers":%d,"ceiling":%d,"worst_density":%d}`,
			problems, len(files), markers, ceiling, worst),
	}
	if problems > 0 {
		v.code = 1
		v.text = fmt.Sprintf("marker check failed, %d problem(s):\n\n%s\n", problems, report.String()) +
			"The five are the three prose markers and the two status glyphs.\n" +
			"Everything else is ASCII. docs/conventions/prose.md is the rule.\n"
		return v, nil
	}
	v.text = fmt.Sprintf("markers ok: %d files, %d markers, densest %d per 100 non-blank lines (%s), ceiling %d\n",
		len(files), markers, worst, worstFile, ceiling)
	return v, nil
}

// firstNonASCII decodes the first byte at or above 0x80 into the codepoint the
// shell half's awk pass would have reported.
//
// ⚠ IT IS THE awk DECODER, NOT Go's. utf8.DecodeRune answers U+FFFD for a
// malformed sequence; the awk pass composed whatever bytes followed the lead
// byte, so a mangled sequence produced a number rather than a replacement
// character. Reproducing that keeps the two halves comparable on a file nobody
// has yet written.
func firstNonASCII(b []byte) (int, bool) {
	at := func(i int) int {
		if i < 0 || i >= len(b) {
			return 0
		}
		return int(b[i])
	}
	for i := 0; i < len(b); i++ {
		b1 := int(b[i])
		if b1 < 128 {
			continue
		}
		switch {
		case b1 >= 240:
			return (b1%8)*262144 + (at(i+1)%64)*4096 + (at(i+2)%64)*64 + at(i+3)%64, true
		case b1 >= 224:
			return (b1%16)*4096 + (at(i+1)%64)*64 + at(i+2)%64, true
		case b1 >= 192:
			return (b1%32)*64 + at(i+1)%64, true
		}
		return 0, true
	}
	return 0, false
}
