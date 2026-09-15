// adapters.go - every network fetch a capture adapter makes is bounded.
//
// ⛔ **A CAPTURE HOST THAT HANGS PRODUCES NO EVIDENCE.** Measured on
// `capture-client` runs 21 and 22, 2026-09-15: the RELEASE lane's *Install the
// client* step ran for thirty minutes where runs 19 and 20 took six seconds,
// every bound around it failed to end the step, the job was cancelled at its
// own limit, and a cancelled job leaves no log and no artifact - which runs 7,
// 8 and 9 had already measured. So a stalled fetch does not cost one capture,
// it costs the whole dispatch and teaches nothing.
//
// ⚠ **THE RULE WAS ALREADY WRITTEN AND NOTHING ENFORCED IT.**
// `docs/conventions/shell.md` section 9: *a script that shells out to unknown
// tools without a limit is a script that hangs, and a script that hangs is one
// nobody runs twice.* Four adapters fetched a release artifact with
// `curl -fsSL --retry 2` and no limit at all. ⛔ That is the shape this
// repository calls a preference stated as a rule, and it is the fourth instance
// it has recorded.
//
// ⚠ **AND THE CONVENTION HELD ON ONE OF TWO PATHS INTO ONE PRODUCT.**
// `aria2-next.sh` has carried `--max-time 20` on its JSON-RPC call since it was
// written, ninety lines above a release fetch that had no bound - which is the
// one-gated-door defect `docs/methodology/reviews.md` calls the most recurring
// hole there is.
//
// -- ⭐ WHAT IS REQUIRED, AND WHAT IS DELIBERATELY NOT --------------------------
//
// A `curl` that RETRIEVES - one with `-o` or `-O` - carries `--max-time`. A
// `git clone` or `git fetch` is wrapped in `timeout`, because git has no flag of
// its own and `timeout` is the spelling section 9 gives for a tool that does not
// bound itself.
//
// ⚠ **THE NUMBER IS THE ADAPTER'S AND THE BOUND IS THE CONTRACT'S.**
// `adapters/README.md` says an adapter owns the switches its own product needs,
// and a source build legitimately needs longer than a 14-megabyte download. So
// this refuses an ABSENT bound and never argues with a value.
//
// ⚠ **A `curl` THAT ONLY ASKS IS OUT OF SCOPE**, and that is a decision rather
// than an oversight: a HEAD or a probe that prints to stdout is bounded by
// whatever reads it, and widening this to every invocation would make the rule
// fire on shapes nobody has been bitten by.
//
// -- ⛔ WHAT THE DOOR SWEEP FOUND THAT THIS SCOPE DOES NOT COVER ---------------
//
// The sweep that followed this rule found the same defect twice more, one
// directory away, which is the shape the rule itself is about:
//
//   - `scripts/doctor/provision.sh` fetched every pinned tool with no limit.
//     ⭐ Fixed in the same change. It is out of SCOPE here because a stall there
//     costs a session's start in front of somebody rather than a whole dispatch.
//   - `scripts/common/mine-repo.sh` and its `.ps1` twin clone with no bound.
//     ⛔ NOT fixed, and deliberately: `timeout` does not exist as a bound on
//     Windows - `timeout.exe` is a PAUSE - so the two halves need two idioms,
//     and `check-twins` compares that pair. Bounding it is its own unit and
//     `TODO/ci.md` carries it under `CI-08`.
//
// ⚠ So this rule covers the capture adapters and says so, rather than claiming a
// reach it does not have. A rule that named the whole tree and enforced one
// directory is the shape this repository calls a preference stated as a rule.
package main

import (
	"fmt"
	"os"
	"regexp"
	"strings"
)

// adapterRe is the scope: the capture adapters, which are the scripts that run
// on a contained host with no operator watching.
var adapterRe = regexp.MustCompile(`^scripts/capture/adapters/[^/]+\.sh$`)

// retrievingCurlRe matches a curl invocation that writes a file.
//
// ⚠ In command position or after a pipe, which is the needle discipline
// `check-project`'s `Write-Error` rule already uses: a `curl` inside a comment
// or a string is not an invocation, and a rule that fired on its own
// description is one this repository has already shipped once.
// ⚠ LEADING WHITESPACE IS PART OF COMMAND POSITION. The first spelling
// anchored on `^` alone, and every invocation in these adapters is indented
// inside a `case` arm - so it matched nothing at all and the floor below
// reported *0 fetches across 4 adapters*. ⭐ That floor is what caught it: a
// rule that found nothing to ask about had asked nothing, and without it this
// would have reported every adapter bounded while reading none of them.
var retrievingCurlRe = regexp.MustCompile(`(^\s*|[|;&(]\s*)curl\b`)

// timeoutWrapRe matches a `timeout` that wraps the command after it.
var timeoutWrapRe = regexp.MustCompile(`(^\s*|[|;&(]\s*)timeout\s`)

// gitFetchRe matches the git subcommands that reach a network.
//
// ⚠ `clone` and `fetch` only. `git rev-parse` and `git log` are local and
// bounding them would be noise around the rule this exists for.
//
// ⛔ IT MATCHES WHEREVER THE COMMAND SITS, INCLUDING AFTER A WRAPPER, and the
// first spelling did not. Anchored in command position it could not match
// `timeout 600 git clone` at all - so a BOUNDED clone was invisible and only an
// unbounded one was counted. The rule was still right and the COUNT meant two
// different things: every curl whether bounded or not, and only the clones that
// were broken. A floor over a number like that is a floor over nothing.
var gitFetchRe = regexp.MustCompile(`(^|\s)git\s+(clone|fetch)\b`)

// logicalLines joins a script's backslash continuations, so a flag on the second
// physical line of one invocation is read as part of it.
//
// ⛔ WITHOUT THIS THE RULE IS BACKWARDS. Every bounded fetch in this tree spans
// three physical lines, so a per-line reader would refuse exactly the
// invocations that comply and accept a one-line unbounded one.
func logicalLines(text string) []string {
	var out []string
	var cur strings.Builder
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimRight(line, "\r")
		if strings.HasSuffix(trimmed, `\`) {
			cur.WriteString(strings.TrimSuffix(trimmed, `\`))
			cur.WriteString(" ")
			continue
		}
		cur.WriteString(trimmed)
		out = append(out, cur.String())
		cur.Reset()
	}
	if cur.Len() > 0 {
		out = append(out, cur.String())
	}
	return out
}

// isComment reports whether a logical line is only a comment.
func isComment(line string) bool {
	return strings.HasPrefix(strings.TrimLeft(line, " \t"), "#")
}

func checkAdapters(r *repo) (verdict, error) {
	files := r.matching(adapterRe, nil)
	// ⛔ A SCOPE TOO SMALL TO BE REAL IS REFUSED, which is the guard
	// `check-ignores` and `ACQ-01`'s catalogue scan both carry. An empty set
	// satisfies every rule below perfectly, so a matcher that stopped matching
	// would report every adapter bounded over no adapters at all.
	if len(files) < 3 {
		return verdict{}, errCannotRun(
			fmt.Sprintf("%d adapter(s) in scope, which is too few to be this tree", len(files)))
	}

	var problems []string
	fetches := 0
	for _, f := range files {
		text, err := os.ReadFile(f)
		if err != nil {
			return verdict{}, errCannotRun("cannot read " + f)
		}
		for n, line := range logicalLines(string(text)) {
			if isComment(line) {
				continue
			}
			switch {
			case retrievingCurlRe.MatchString(line):
				// ⚠ Only a curl that RETRIEVES. One that prints is bounded by
				// its reader; see the header.
				if !strings.Contains(line, " -o ") && !strings.Contains(line, " -O") {
					continue
				}
				fetches++
				if !strings.Contains(line, "--max-time") {
					problems = append(problems, fmt.Sprintf(
						"%s:%d a curl that writes a file carries no --max-time", f, n+1))
				}
			case gitFetchRe.MatchString(line):
				fetches++
				// ⛔ THE WRAPPER MUST COME BEFORE THE COMMAND IT BOUNDS. Asking
				// only whether `timeout` appears somewhere on the line would
				// accept `git clone ... && timeout 5 true`, which bounds
				// nothing. So the prefix up to the git invocation is what is
				// asked, and it is asked for a `timeout` in command position.
				at := gitFetchRe.FindStringIndex(line)
				if at == nil || !timeoutWrapRe.MatchString(line[:at[0]]) {
					problems = append(problems, fmt.Sprintf(
						"%s:%d a git clone or fetch is not wrapped in timeout", f, n+1))
				}
			}
		}
	}

	// ⛔ AND A RULE THAT FOUND NOTHING TO ASK ABOUT HAS ASKED NOTHING. Every
	// adapter here retrieves something; a reader that matched no invocation at
	// all would report a clean tree while examining none of them.
	if fetches < 3 {
		return verdict{}, errCannotRun(
			fmt.Sprintf("%d fetch(es) found across %d adapter(s), which is too few to be this tree",
				fetches, len(files)))
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-adapters/1","problems":%d,"adapters":%d,"fetches":%d}`,
			len(problems), len(files), fetches),
	}
	if len(problems) > 0 {
		var b strings.Builder
		fmt.Fprintf(&b, "an adapter fetch has no time limit, %d problem(s):\n\n", len(problems))
		for _, p := range problems {
			fmt.Fprintf(&b, "  %s\n", p)
		}
		b.WriteString("\n⛔ A capture host that hangs produces no evidence: the job is cancelled at\n")
		b.WriteString("its own bound and a cancelled job leaves no log and no artifact.\n")
		b.WriteString("docs/conventions/shell.md section 9 is the rule.\n")
		v.code = 1
		v.text = b.String()
		return v, nil
	}
	v.text = fmt.Sprintf("all %d fetch(es) across %d adapter(s) are bounded\n", fetches, len(files))
	return v, nil
}
