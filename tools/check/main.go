// bit-check - this repository's checking layer, as one binary on both platforms.
//
// CI-10. The layer this replaces was a `.sh` and a hand-written `.ps1` twin per
// rule, plus `check-twins` to compare them, and the twin was most of the wall
// clock: measured on 2026-09-10 over eleven pairs, the `sh` halves together took
// 7.4 seconds and the PowerShell halves 96, of which `check-control-bytes.ps1`
// alone was 67.9. `check-workflow` runs the whole gate about nine times, so that
// cost is paid nine times on every push.
//
// ⭐ ONE BINARY REMOVES THE CLASS RATHER THAN CHECKING FOR IT. `check-twins`
// exists because two hand-written halves drift. A single implementation that
// runs on both platforms cannot drift from itself, so the comparison stops being
// necessary rather than getting faster.
//
// -- ⛔ WHAT A PORT MAY NOT CHANGE -------------------------------------------
//
// A ported check refuses exactly what its shell half refused, over the same
// planted defect, with the same exit-code vocabulary:
//
//	0  the rule held
//	1  the rule was refused
//	2  the check could not run
//
// ⚠ A port that changed one verdict while getting faster would be trading the
// thing being measured for the measurement. `scripts/common/check-bitcheck.sh`
// is what compares each ported check against the halves it replaces, case for
// case over the same plants, and it runs BEFORE any half is deleted.
//
// -- ⭐ NO DEPENDENCIES, WHICH IS A SUPPLY-CHAIN PROPERTY AND NOT A PREFERENCE -
//
// This module has an empty require list and therefore no `go.sum`. Nothing here
// is fetched at build time, so a build needs no network and no pin audit, and
// `CI-04`'s dependency surface does not grow by a language.
//
// Usage:
//
//	bit-check <check> [--json]
//	bit-check --rows
//
// ⛔ Read the exit code from this process, unpiped.
package main

import (
	"fmt"
	"os"
	"sort"
)

// cannotRun is the error type that means exit 2 rather than exit 1.
//
// ⛔ THE TWO ARE NEVER FOLDED TOGETHER. A harness exit of 2 is *could not run*
// and never *refused*; this repository has counted one as the other and reported
// a guard proved over a plant that had not compiled.
type cannotRun struct{ msg string }

func (e cannotRun) Error() string { return e.msg }

func errCannotRun(msg string) error { return cannotRun{msg} }

// optPermitted is check-licences' listing mode. ⚠ It is reported BEFORE any
// rule runs, because a caller asking what is permitted is asking about the
// file as written rather than about whether it is coherent.
var optPermitted bool

// verdict is what a check answers: the exit code plus the one JSON line that
// `check-bitcheck.sh` compares against the shell half's.
type verdict struct {
	code int
	json string
	text string
	// ⛔ stderr IS PRINTED IN BOTH MODES, and it exists for one shape: a refusal
	// that happens BEFORE a check has a countable answer. check-changelog's
	// no-entries case is the instance - a file whose headings the parser does not
	// recognise has no `problems` to report, so its shell half wrote to stderr
	// and exited 1 without emitting JSON at all. A port that invented a JSON line
	// there would be answering a question its predecessor refused to answer.
	stderr string
}

// check is one rule. The name is the gate row label, so a row this map does not
// carry is a row the gate cannot run.
type check func(r *repo) (verdict, error)

var checks = map[string]check{
	"check-changelog":     checkChangelog,
	"check-control-bytes": checkControlBytes,
	"check-licences":      checkLicences,
	"check-markers":       checkMarkers,
	"check-one-home":      checkOneHome,
	"check-placeholders":  checkPlaceholders,
}

func names() []string {
	out := make([]string, 0, len(checks))
	for k := range checks {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

func main() {
	args := os.Args[1:]
	if len(args) == 0 {
		fmt.Fprintf(os.Stderr, "bit-check: which check? one of: %v\n", names())
		os.Exit(2)
	}

	// ⭐ --rows PRINTS THE NAMES AND RUNS NOTHING, so the gate can compare the
	// set it queues against the set this binary carries. A row the runner names
	// and the binary does not have is rot, exactly as a step a harness names and
	// a workflow no longer has is.
	if args[0] == "--rows" {
		for _, n := range names() {
			fmt.Println(n)
		}
		os.Exit(0)
	}

	name := args[0]
	jsonOut := false
	for _, a := range args[1:] {
		switch a {
		case "--json":
			jsonOut = true
		case "--permitted":
			// ⚠ ONE CHECK'S MODE RATHER THAN A GLOBAL ONE, and it is parsed here
			// because the dispatcher owns the argument list. check-cache asks
			// check-licences which targets may be redistributed; a second
			// derivation of that answer in the caller would be the value in two
			// places this repository refuses everywhere else.
			optPermitted = true
		default:
			fmt.Fprintf(os.Stderr, "bit-check: unknown argument: %s\n", a)
			os.Exit(2)
		}
	}

	fn, ok := checks[name]
	if !ok {
		fmt.Fprintf(os.Stderr, "bit-check: no such check: %s\n", name)
		os.Exit(2)
	}

	r, err := openRepo()
	if err != nil {
		fmt.Fprintf(os.Stderr, "%s: %s\n", name, err)
		os.Exit(2)
	}

	v, err := fn(r)
	if err != nil {
		fmt.Fprintf(os.Stderr, "%s: %s\n", name, err)
		os.Exit(2)
	}

	if v.stderr != "" {
		fmt.Fprint(os.Stderr, v.stderr)
	} else if jsonOut {
		fmt.Println(v.json)
	} else {
		fmt.Print(v.text)
	}
	os.Exit(v.code)
}
