// ignores.go - can a credential file this repository refuses still be staged?
//
// ⛔ TWO DEFENCES STAND BETWEEN A CREDENTIAL AND A REMOTE, AND NOTHING COMPARED
// THEM. `.gitignore` is the one that acts BEFORE the file exists - its own header
// says so: *listed before the files exist, so one can never be staged by
// accident* - and `check-no-secrets` rule 1 is the backstop that refuses one that
// got in anyway. A name the backstop knows and the ignore list does not is a hole
// in the first defence, and the only thing standing in front of it is a check
// that runs after `git add -A` has already taken the file.
//
// ⚠ FOUND BY A DOOR SWEEP ON 2026-09-15, not by this rule. `*.jks` and `id_ecdsa`
// were both in check-no-secrets' name list and in neither `.gitignore` line, so a
// Java keystore and an ECDSA private key were trackable by `git add -A` with
// nothing refusing them until the gate ran. ⭐ The two lines were added in the
// same change as this check, which is the half that matters: a fix with no rule
// behind it is the state the tree was already in.
//
// -- ⭐ IT ASKS git RATHER THAN RE-READING .gitignore ------------------------
//
// `git check-ignore --no-index` answers whether the RULES would ignore a name,
// which is the question, and it answers it the way `git add` will. A second
// parser of the ignore format here would be a second answer to what git ignores -
// and where two answers exist they drift, in the direction that keeps a check
// green. It is the same argument repo.go makes for asking `git ls-files` what is
// in the tree.
//
// -- ⛔ THE SPECIMENS ARE CHECKED AGAINST THE RULE THEY STAND FOR -------------
//
// A list of filenames is a second list, and a second list goes stale. So every
// specimen below is first asserted to be a name check-no-secrets rule 1 would
// actually refuse, using that rule's own expressions. A shape narrowed out of
// the regex without its specimen being removed is a finding here rather than a
// case that quietly stops meaning anything.
//
// ⚠ What it cannot see, stated: a shape ADDED to the regex with no specimen
// added beside it. Nothing can derive a filename from a regular expression, so
// that direction stays with the reviewer, and it is why the specimen list sits
// next to the expressions rather than in a data file somewhere else.
//
// ⚠ And it asks about names at the repository root. `.gitignore` patterns with
// no slash match at every level, so a root answer covers the tree; a nested
// `.gitignore` that re-permitted one of these names deeper down is outside what
// this asks.
package main

import (
	"fmt"
	"os/exec"
	"strings"
)

// ignoreSpecimens are one filename per credential shape rule 1 knows.
//
// ⛔ EVERY ONE OF THEM MUST BE IGNORED. There is no "some are prose" branch here
// the way there is in check-placeholders: a file whose whole purpose is to hold a
// credential has no legitimate reason to be trackable.
var ignoreSpecimens = []string{
	".env",
	".env.local",
	".dev.vars",
	".dev.vars.local",
	"secrets.pem",
	"server.key",
	"store.p12",
	"store.pfx",
	"debug.keystore",
	"release.jks",
	"id_rsa",
	"id_ed25519",
	"id_ecdsa",
	"credentials.json",
	"service-account-prod.json",
}

// specimenFloor is the smallest list that can still be real.
//
// ⛔ A CHECK OVER AN EMPTY LIST REPORTS A CLEAN BILL OVER NOTHING, which is the
// shape check-licences refuses in its register and ACQ-01's catalogue scan
// refuses in its vocabulary. ⚠ Measured by planting: with the specimens emptied
// this answered *every one of the 0 credential shapes is also ignored* and
// exited 0. The floor is below the list and above nothing, so a shape may be
// retired without tripping it while a list that stopped being built cannot pass.
const specimenFloor = 12

func checkIgnores(r *repo) (verdict, error) {
	if _, err := exec.LookPath("git"); err != nil {
		return verdict{}, errCannotRun("git not found")
	}
	if len(ignoreSpecimens) < specimenFloor {
		return verdict{}, errCannotRun(fmt.Sprintf(
			"only %d specimen(s), below the floor of %d: this check cannot be meaningful over a list that small",
			len(ignoreSpecimens), specimenFloor))
	}

	var problems []string
	for _, name := range ignoreSpecimens {
		// ⛔ THE SPECIMEN IS TIED TO THE RULE FIRST. Without this the list is a
		// set of names somebody wrote once, and a shape narrowed out of
		// check-no-secrets would leave a case here still passing over a name
		// nothing refuses any more.
		if !credFileRe.MatchString(name) || credWaived.MatchString(name) {
			problems = append(problems,
				fmt.Sprintf("%s is no longer a name check-no-secrets rule 1 refuses, so it is a stale specimen", name))
			continue
		}

		// ⚠ --no-index asks about the RULES rather than about the index, which
		// is the question: a name already tracked is reported as not ignored by
		// default, and that would turn this into a check on what has been staged
		// rather than on what could be.
		cmd := exec.Command("git", "check-ignore", "-q", "--no-index", "--", name)
		err := cmd.Run()
		if err == nil {
			continue
		}
		// ⛔ 1 IS *NOT IGNORED* AND ANYTHING ELSE IS *COULD NOT RUN*. Folding
		// them would report a clean ignore list over a git that never answered.
		ee, ok := err.(*exec.ExitError)
		if !ok || ee.ExitCode() != 1 {
			return verdict{}, errCannotRun("git check-ignore could not answer for " + name)
		}
		problems = append(problems,
			fmt.Sprintf("%s is refused by check-no-secrets and is NOT ignored, so `git add -A` would stage it", name))
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-ignores/1","problems":%d,"shapes":%d}`,
			len(problems), len(ignoreSpecimens)),
	}
	if len(problems) > 0 {
		var b strings.Builder
		fmt.Fprintf(&b, "the ignore list and the secret-file rule disagree, %d problem(s):\n\n", len(problems))
		for _, p := range problems {
			fmt.Fprintf(&b, "  %s\n", p)
		}
		b.WriteString("\n⛔ .gitignore is the defence that acts BEFORE the file exists. Add the\n")
		b.WriteString("missing line there rather than relying on check-no-secrets, which only\n")
		b.WriteString("refuses a file that has already been staged.\n")
		v.code = 1
		v.text = b.String()
		return v, nil
	}
	v.text = fmt.Sprintf("every one of the %d credential shapes check-no-secrets refuses is also ignored by git\n",
		len(ignoreSpecimens))
	return v, nil
}
