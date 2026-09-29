// cache.go - does the artifact cache survive a source that moved, and does it
// keep bytes only where the licence register permits?
//
// `ACQ-05`'s Prove is both halves at once: the cache tests enforce the licence
// register, and the artifact's identity survives a source URL change.
//
// Ported from scripts/acquisition/check-cache.sh and its PowerShell twin, CI-10.
// ⚠ Both halves drove the SAME Rust example, so they could never disagree about
// the cache; what they could disagree about was the harness underneath it, and
// that is what one implementation removes.
//
// -- ⭐ THE PERMITTED SET COMES FROM THE REGISTER, NOT FROM HERE ------------
//
// This asks `readRegister` what the register permits and hands the answer to the
// scenario, so the tie between `FOUND-04`'s register and this cache is a call
// rather than a second reading. ⚠ Today it answers nothing, which is the state
// the first case asserts rather than assumes.
//
// ⛔ AND THAT MAKES THE REFUSAL CASE AMBIGUOUS ON ITS OWN. A cache that refused
// to keep any bytes for any reason would pass a case built only on the empty
// answer. The control hands the scenario a target explicitly and watches the same
// cache accept it, so the refusal is shown to be the register's answer rather
// than an inability to store at all.
//
// ⚠ NOTHING HERE PLANTS IN A TRACKED FILE. The probe guards plant in a COPY of
// the register, because a gate check that edits a tracked file leaves a dirty
// tree behind when it is interrupted.
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

func checkCache(r *repo) (verdict, error) {
	h := &harness{me: "check-cache"}

	scenario, err := exampleBinary(r.root, "cache-scenario", "bit-ids")
	if err != nil {
		return verdict{}, err
	}

	work, err := scratch("checkcache")
	if err != nil {
		return verdict{}, err
	}
	defer os.RemoveAll(work)

	// ⛔ A register that cannot be read is COULD NOT RUN and never an empty
	// permitted list: an empty answer would report every target as
	// redistribution-refused, which is the safe-looking direction and still a
	// verdict nobody measured. The wording is the shell half's, whose call this
	// is.
	_, targets, _, err := readRegister()
	if err != nil {
		return verdict{}, errCannotRun("check-licences --permitted exited 2")
	}
	permitted := permittedTargets(targets)

	if len(permitted) == 0 {
		h.ok("register  nothing in the register permits keeping an artifact's bytes")
	} else {
		h.bad(fmt.Sprintf("register  the register permits %d target(s): %s", len(permitted), strings.Join(permitted, " ")))
	}

	// The scenario, run with exactly what the register said.
	var args []string
	for _, id := range permitted {
		args = append(args, "--permitted", id)
	}
	refusedOut := filepath.Join(work, "refused.out")
	refusedErr := filepath.Join(work, "refused.err")
	rc := runTo(refusedOut, refusedErr, scenario, args...)
	refused := fileLines(refusedOut)

	if rc == 0 {
		h.ok("scenario  the cache behaves as the model says under the register's answer")
	} else {
		h.bad(fmt.Sprintf("scenario  exit %d: %s", rc, headJoin(fileLines(refusedErr), 3)))
	}

	// ⛔ THE IDENTITY HALF OF THE PROVE. One artifact and two retrievals means the
	// digest named the same thing from both locations.
	if anyContains(refused, "artifacts: 1") && anyContains(refused, "retrievals: 2") {
		h.ok("identity  a moved source adds a retrieval and not an artifact")
	} else {
		var saw []string
		for _, l := range refused {
			if strings.HasPrefix(l, "artifacts:") || strings.HasPrefix(l, "retrievals:") {
				saw = append(saw, l)
			}
		}
		h.bad("identity  a moved source produced: " + headJoin(saw, len(saw)))
	}

	if anyContains(refused, "keeping nothing: accepted") {
		h.ok("policy    a cache that keeps no bytes is accepted")
	} else {
		h.bad("policy    a cache that keeps no bytes was refused")
	}

	if anyContains(refused, "keeping the bytes: refused as E-CAC-01") {
		h.ok("E-CAC-01  keeping the bytes is refused while the register refuses them")
	} else {
		h.bad("E-CAC-01  the refusal did not appear: " + tailJoin(refused, 2))
	}

	// ⛔ THE CONTROL THE REFUSAL NEEDS. Without this the case above passes over a
	// cache that can never store anything, which is a different program from one
	// that asks the register.
	permittedOut := filepath.Join(work, "permitted.out")
	permittedErr := filepath.Join(work, "permitted.err2")
	prc := runTo(permittedOut, permittedErr, scenario, "--permitted", "aria2")
	kept := fileLines(permittedOut)
	switch {
	case prc != 0:
		h.bad(fmt.Sprintf("control   the permitted run exited %d: %s", prc, headJoin(fileLines(permittedErr), 2)))
	case anyContains(kept, "keeping the bytes: permitted by the register"):
		h.ok("control   the same cache keeps the bytes when a target is permitted")
	default:
		h.bad("control   the permitted run did not keep the bytes")
	}

	// ⚠ And the two runs differ only in that line, so the refusal is about the
	// permission and not about anything else the scenario did.
	switch n := diffCount(refused, kept); {
	case n == 0:
		h.bad("control   the refused and permitted runs are identical, so nothing changed")
	case n == 2:
		h.ok("control   the two runs differ on exactly the permission line")
	default:
		h.bad(fmt.Sprintf("control   the two runs differ in %d line(s), expected 2", n))
	}

	// ⚠ The probe guards need a file to plant in, and it is a copy rather than
	// the register itself.
	regCopy := filepath.Join(work, "register.toml")
	regBytes, err := os.ReadFile(licRegister)
	if err != nil || os.WriteFile(regCopy, regBytes, 0o644) != nil {
		return verdict{}, errCannotRun("cannot copy " + licRegister)
	}
	h.probeGuards(regCopy, `artifact_policy = "measurements-only"`, "redistribute")

	return h.report("check-cache/1", "cases"), nil
}
