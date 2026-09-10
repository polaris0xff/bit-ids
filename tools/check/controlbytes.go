// controlbytes.go - is there a literal control byte in any text file?
//
// The defect this exists to catch is a file that is invisible to review. A
// literal control byte makes a file unreadable to BOTH review tools at once:
// `grep` calls it binary and SKIPS it, saying so in a line nobody reads, and
// `git diff` prints "Binary files differ", so a code review of the file shows NO
// DIFF AT ALL. `git diff --text` renders it fine, which is the proof that only
// reviewability was ever at stake.
//
// ⭐ The runtime value is identical either way. Write the escape, not the byte.
// Because correctness never depends on it, this survives a long time unnoticed.
//
// ⚠ IT IS NOT A RULE PEOPLE CAN REMEMBER. In the project this came from the rule
// was stated in a shared source file and four source files broke it anyway, and
// the post-mortem writing up the lesson reintroduced the byte TWICE while
// writing about it.
//
// Ported from scripts/common/check-control-bytes.sh and its PowerShell twin,
// whose verdicts this reproduces exactly. The twin was 67.9 seconds against the
// shell half's 1.7 and this binary's own timing is in TODO/ci.md.
package main

import (
	"fmt"
	"os"
	"strings"
)

// isControl reports whether b is a C0 control other than the three that are
// legitimately in text.
//
// ⚠ TAB, NEWLINE AND CARRIAGE RETURN ARE NOT CONTROLS FOR THIS PURPOSE. They are
// what text is made of. NUL is, and it is the single commonest offender - it is
// what somebody reaches for as a composite-key separator - but it is counted
// separately below, because it could not live in the shell half's variable.
//
// ⛔ THE CLASS IS THE SHELL HALF'S, CHARACTER FOR CHARACTER: 0x01-0x08, 0x0B,
// 0x0C, 0x0E-0x1F. DEL is NOT in it. A port that added 0x7f would be refusing a
// file its predecessor accepted, which is a verdict changed by a translation.
func isControl(b byte) bool {
	switch b {
	case 0x00, '\t', '\n', '\r':
		return false
	}
	return b < 0x20
}

func checkControlBytes(r *repo) (verdict, error) {
	files := r.matching(textRe, nil)
	if len(files) == 0 {
		return verdict{}, errCannotRun("no text files in scope")
	}

	count := 0
	var report strings.Builder

	for _, f := range files {
		b, err := os.ReadFile(f)
		if err != nil {
			continue
		}

		// ⛔ THE SCAN IS OVER BYTES AND THE LINE NUMBER IS COUNTED THE SAME WAY.
		// The shell half reported `grep -n`'s line for a C0 control and the file
		// alone for a NUL, and the two are separate findings in that order: a
		// file carrying both is reported once, as the control.
		line := 1
		found := -1
		var nul bool
		for i := 0; i < len(b); i++ {
			c := b[i]
			if c == 0 {
				nul = true
				continue
			}
			if c == '\n' {
				line++
				continue
			}
			if found < 0 && isControl(c) {
				found = line
			}
		}

		switch {
		case found >= 0:
			count++
			fmt.Fprintf(&report, "  %s:%d a C0 control byte\n", f, found)
		case nul:
			count++
			fmt.Fprintf(&report, "  %s a NUL byte\n", f)
		}
	}

	v := verdict{
		code: 0,
		json: fmt.Sprintf(`{"schema":"check-control-bytes/1","problems":%d,"files":%d}`, count, len(files)),
	}
	if count > 0 {
		v.code = 1
		v.text = fmt.Sprintf("literal control bytes in %d file(s):\n\n%s\n", count, report.String()) +
			"Write the ESCAPE, not the byte. The escape is the same character at\n" +
			"runtime, and the byte is what makes the file invisible to grep and\n" +
			"unreviewable in git diff. docs/conventions/shell.md section 6.\n"
		return v, nil
	}
	v.text = fmt.Sprintf("no literal control bytes in %d text files (tracked plus untracked-not-ignored)\n", len(files))
	return v, nil
}
