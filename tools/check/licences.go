// licences.go - is every target and every third-party package in the register,
// with a disposition somebody actually decided?
//
// Six rules over catalogue/licences.toml, checked against catalogue/clients.toml
// and Cargo.lock in BOTH directions, plus one over the tree itself.
//
// ⛔ A REGISTER OF NOTHING SATISFIES EVERY RULE. Two empty lists agree perfectly,
// which is the shape a broken parser reports success as, so a register that
// parses to no rows is refused before any comparison is believed.
//
// ⛔ AND THE LAST RULE IS THE HALF A REGISTER CANNOT ANSWER ABOUT ITSELF. Every
// row says the bytes are never shipped; nothing in a register checks that none
// are here. An installer that arrived in a commit is exactly the thing a licence
// register exists to prevent and exactly the thing no amount of recording
// prevents.
//
// Ported from scripts/common/check-licences.sh and its PowerShell twin.
//
// ⚠ THE REGISTER'S SHAPE IS LINE-BASED ON PURPOSE and this port keeps it that
// way. A TOML library here and a hand parser in the half it replaces would be two
// answers to one question, and the two would disagree first on the file that
// matters. A block is closed by the NEXT header or by end of file, never by its
// own last key: getting that wrong loses exactly one row, at the end.
package main

import (
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"
)

type licRow struct {
	id      string
	version string
	licence string
	source  string
	redist  string
	notice  string
}

var bundledRe = regexp.MustCompile(`(?i)\.(exe|msi|dmg|pkg|deb|rpm|appimage|apk|jar|7z|zip|xz|bz2|gz|tgz|torrent|dll|so|dylib)$`)

// tomlValue lifts the string out of `key = "value"`.
func tomlValue(line string) string {
	i := strings.Index(line, `= "`)
	if i < 0 {
		return ""
	}
	v := line[i+3:]
	return strings.TrimSuffix(v, `"`)
}

func checkLicences(r *repo) (verdict, error) {
	const (
		register  = "catalogue/licences.toml"
		catalogue = "catalogue/clients.toml"
		lock      = "Cargo.lock"
	)
	for _, p := range []string{register, catalogue, lock} {
		if _, err := os.Stat(p); err != nil {
			return verdict{}, errCannotRun(p + " is missing")
		}
	}

	regBytes, err := os.ReadFile(register)
	if err != nil {
		return verdict{}, errCannotRun("cannot read " + register)
	}
	regText := string(regBytes)

	failures := []string{}
	fail := func(f string, a ...any) { failures = append(failures, fmt.Sprintf(f, a...)) }

	if !strings.Contains(regText, "\nschema = \"bit-ids/licences/1\"\n") &&
		!strings.HasPrefix(regText, "schema = \"bit-ids/licences/1\"\n") {
		fail("%s does not declare the licences schema", register)
	}

	var targets, deps []licRow
	{
		var cur licRow
		section := ""
		flush := func() {
			if cur.id == "" {
				cur = licRow{}
				return
			}
			if cur.notice == "" {
				cur.notice = "-"
			}
			switch section {
			case "target":
				targets = append(targets, cur)
			case "dep":
				deps = append(deps, cur)
			}
			cur = licRow{}
		}
		for _, lb := range splitLines(regBytes) {
			line := string(lb)
			switch {
			case strings.HasPrefix(line, "[[targets]]"):
				flush()
				section = "target"
			case strings.HasPrefix(line, "[[dependencies]]"):
				flush()
				section = "dep"
			case strings.HasPrefix(line, `id = "`), strings.HasPrefix(line, `name = "`):
				cur.id = tomlValue(line)
			case strings.HasPrefix(line, `version = "`):
				cur.version = tomlValue(line)
			case strings.HasPrefix(line, `licence = "`):
				cur.licence = tomlValue(line)
			case strings.HasPrefix(line, `licence_source = "`):
				cur.source = tomlValue(line)
			case strings.HasPrefix(line, `redistribute = "`):
				cur.redist = tomlValue(line)
			case strings.HasPrefix(line, `notice = "`):
				cur.notice = tomlValue(line)
			}
		}
		flush()
	}

	// ⚠ REPORTED BEFORE ANY RULE RUNS. A caller asking what is permitted is
	// asking about the file as written, not about whether it is coherent.
	if optPermitted {
		var ids []string
		for _, row := range targets {
			if row.redist == "permitted" {
				ids = append(ids, row.id)
			}
		}
		sort.Strings(ids)
		out := ""
		for _, id := range ids {
			out += id + "\n"
		}
		return verdict{code: 0, text: out, json: out}, nil
	}

	// -- 1. every catalogue target has exactly one row, and the reverse -------
	catBytes, err := os.ReadFile(catalogue)
	if err != nil {
		return verdict{}, errCannotRun("cannot read " + catalogue)
	}
	catIDs := map[string]bool{}
	closed := map[string]bool{}
	{
		id := ""
		for _, lb := range splitLines(catBytes) {
			line := string(lb)
			switch {
			case strings.HasPrefix(line, "[[targets]]"):
				id = ""
			case strings.HasPrefix(line, `id = "`):
				id = tomlValue(line)
				catIDs[id] = true
			case strings.HasPrefix(line, "open_source = "):
				if strings.TrimSpace(strings.TrimPrefix(line, "open_source = ")) == "false" && id != "" {
					closed[id] = true
				}
			}
		}
	}

	regIDs := map[string]int{}
	for _, t := range targets {
		regIDs[t.id]++
	}
	reportSet(fail, "catalogue targets with no register row", missing(catIDs, regIDs))
	reportSet(fail, "register rows naming no catalogue target", extra(catIDs, regIDs))
	var dupes []string
	for id, n := range regIDs {
		if n > 1 {
			dupes = append(dupes, id)
		}
	}
	reportSet(fail, "register carries a target more than once", dupes)

	// -- 2. every third-party package has exactly one row, at its version -----
	//
	// ⚠ A package with no source is a member of this workspace and is not third
	// party, which is the same rule check-project applies to the same file.
	lockBytes, err := os.ReadFile(lock)
	if err != nil {
		return verdict{}, errCannotRun("cannot read " + lock)
	}
	lockDeps := map[string]bool{}
	{
		name, version := "", ""
		hasSource := false
		for _, lb := range splitLines(lockBytes) {
			line := string(lb)
			switch {
			case strings.HasPrefix(line, "[[package]]"):
				name, version, hasSource = "", "", false
			case strings.HasPrefix(line, `name = "`):
				name = tomlValue(line)
			case strings.HasPrefix(line, `version = "`):
				version = tomlValue(line)
			case strings.HasPrefix(line, `source = "`):
				hasSource = true
			case strings.HasPrefix(line, `checksum = "`):
				if hasSource {
					lockDeps[name+"\t"+version] = true
				}
			}
		}
	}
	regDeps := map[string]int{}
	for _, d := range deps {
		regDeps[d.id+"\t"+d.version]++
	}
	reportSet(fail, "locked packages with no register row", missing(lockDeps, regDeps))
	reportSet(fail, "register rows naming no locked package", extra(lockDeps, regDeps))

	// -- 3. every row carries a disposition -----------------------------------
	var noDisp []string
	for _, t := range targets {
		if t.licence == "" || t.licence == "-" {
			noDisp = append(noDisp, t.id+" has no licence")
		} else if t.redist != "refused" && t.redist != "permitted" {
			noDisp = append(noDisp, t.id+" has an unknown redistribute value: "+t.redist)
		}
	}
	for _, d := range deps {
		if d.licence == "" || d.licence == "-" {
			noDisp = append(noDisp, d.id+" has no licence")
		} else if d.redist != "refused" && d.redist != "permitted" {
			noDisp = append(noDisp, d.id+" has an unknown redistribute value: "+d.redist)
		}
	}
	reportSet(fail, "register rows with no disposition", noDisp)

	// -- 4. permitted is the expensive value and it has to be earned ----------
	//
	// ⛔ Redistribution needs a licence somebody established and a notice to
	// carry with it. `unverified` plus `permitted` is the combination that would
	// publish somebody's bytes on nobody's authority.
	var unearned []string
	for _, row := range append(append([]licRow{}, targets...), deps...) {
		if row.redist == "permitted" && (row.licence == "unverified" || row.notice == "-") {
			unearned = append(unearned, row.id)
		}
	}
	reportSet(fail, "permitted without a verified licence and a notice", unearned)

	// -- 5. a closed-source target is never recorded under an open licence ----
	licOf := map[string]string{}
	for _, t := range targets {
		licOf[t.id] = t.licence
	}
	var mislabelled []string
	for id := range closed {
		switch licOf[id] {
		case "proprietary", "unverified", "":
		default:
			mislabelled = append(mislabelled, fmt.Sprintf("%s is closed source and recorded as %s", id, licOf[id]))
		}
	}
	reportSet(fail, "closed-source targets under an open licence", mislabelled)

	// -- 6. no artifact this repository may not redistribute is tracked -------
	var bundled []string
	for _, f := range r.files() {
		if bundledRe.MatchString(f) {
			bundled = append(bundled, f)
		}
	}
	reportSet(fail, "tracked artifact this repository may not redistribute", bundled)

	// ⛔ A REGISTER OF NOTHING IS NOT A CLEAN REGISTER. Every rule above is
	// satisfied by two empty lists, and that is how a broken parser reports
	// success.
	if len(targets) == 0 || len(deps) == 0 {
		fail("the register parsed to %d target row(s) and %d dependency row(s)", len(targets), len(deps))
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-licences/1","failures":%d,"targets":%d,"dependencies":%d}`,
			len(failures), len(targets), len(deps)),
	}
	if len(failures) > 0 {
		v.code = 1
		var b strings.Builder
		for _, f := range failures {
			fmt.Fprintf(&b, "FAIL: %s\n", f)
		}
		v.text = b.String()
		return v, nil
	}
	v.text = fmt.Sprintf("licence register: %d target(s) and %d dependency row(s), every one with a disposition\n",
		len(targets), len(deps))
	return v, nil
}

// reportSet turns a set of findings into one failure line, sorted so two runs
// over one tree produce one report.
func reportSet(fail func(string, ...any), label string, items []string) {
	if len(items) == 0 {
		return
	}
	sort.Strings(items)
	fail("%s: %s", label, strings.Join(items, " "))
}

func missing(have map[string]bool, in map[string]int) []string {
	var out []string
	for k := range have {
		if in[k] == 0 {
			out = append(out, strings.ReplaceAll(k, "\t", " "))
		}
	}
	return out
}

func extra(have map[string]bool, in map[string]int) []string {
	var out []string
	for k := range in {
		if !have[k] {
			out = append(out, strings.ReplaceAll(k, "\t", " "))
		}
	}
	return out
}
