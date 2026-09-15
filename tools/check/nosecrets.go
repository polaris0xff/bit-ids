// nosecrets.go - does any file in this tree carry something that must not be
// published?
//
// The defect this exists to catch is a credential, or a fingerprint of a private
// system, reaching a remote. Once it does, a history rewrite does not undo it:
// the value was readable, and it may be cached, mirrored or already indexed.
// Rotation is the fix; this is what stops it needing one.
//
// ⛔ IT FINDS THE SHAPES IT KNOWS, AND A GREEN RUN IS NOT A CLEARANCE. It cannot
// find a password that looks like a word, a hostname that reads as prose, or a
// page of correct-looking examples that happens to describe a real system. It
// narrows the reading. It does not replace it.
//
// -- ⛔ WHAT THE PORT HAD TO CARRY, AND IT IS NOT THE PATTERNS ----------------
//
// The patterns are the easy half. What a port loses silently is the SHAPE of the
// pipeline the shell half built them into, and two properties of it decide
// verdicts:
//
//  1. ⛔ THE ALLOW EXPRESSIONS RUN OVER THE GREP OUTPUT LINE, `path:lineno:text`,
//     NOT OVER THE TEXT. One of them - the Cargo.lock digest - is anchored with
//     `^(.*Cargo\.lock:[0-9]+:checksum = )`, so it can only match with the path
//     prefix in front of it. A port that ran the allows over the file's own line
//     would refuse every lockfile digest in the tree.
//  2. ⛔ AN ALLOWED ITEM IS DELETED FROM THE LINE; THE LINE IS NOT DROPPED.
//     `grep -v` drops lines, not characters, so an allowed digest sitting beside
//     a real credential would take the credential out of the report with it.
//     docs/conventions/forbidden-patterns.md carries that row.
//
// ⚠ The home-path rule is the opposite shape and that difference is inherited
// rather than tidied: its two exclusions ARE `grep -v` over the whole line. A
// port that unified the two would be changing a verdict while claiming only to
// change a language.
//
// Modes: `--public` adds the rules that only matter for a repository that is or
// will be public - emails, absolute home paths, long hex identifiers. In a
// private project those are legitimate content, which is why they are not the
// default. `--all-history` reads every blob ever committed; it is slow and is
// deliberately not a gate row.
//
// Ported from scripts/common/check-no-secrets.sh and its PowerShell twin.
package main

import (
	"bufio"
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strings"
)

// optPublic and optAllHistory are this check's modes, parsed by the dispatcher
// the way check-licences' --permitted is.
var (
	optPublic     bool
	optAllHistory bool
)

// --- 1. a credential FILE is tracked -----------------------------------------
//
// The strongest signal there is: not a value that looks like a secret, but a
// file whose whole purpose is to hold one.
var (
	credFileRe = regexp.MustCompile(`(^|/)(\.env(\..+)?|\.dev\.vars(\..+)?|.*\.(pem|key|p12|pfx|keystore|jks)|id_rsa|id_ed25519|id_ecdsa|credentials\.json|service-account.*\.json)$`)
	credWaived = regexp.MustCompile(`\.example$|\.sample$|\.template$`)
)

// --- 2. secret-shaped strings ------------------------------------------------
//
// Each pattern is a vendor's documented token shape. ⚠ A generic "high entropy"
// rule is deliberately absent: it fires on hashes, minified code and base64
// fixtures, and a check that cries wolf is a check somebody switches off.
type contentRule struct {
	title string
	re    *regexp.Regexp
}

var contentRules = []contentRule{
	{"a private key block", regexp.MustCompile(`BEGIN (RSA |OPENSSH |EC |DSA |PGP )?PRIVATE KEY`)},
	{"an aws access key id", regexp.MustCompile(`AKIA[0-9A-Z]{16}`)},
	{"a github token", regexp.MustCompile(`gh[pousr]_[A-Za-z0-9]{30,}`)},
	{"a slack token", regexp.MustCompile(`xox[abprs]-[0-9A-Za-z-]{10,}`)},
	{"a google api key", regexp.MustCompile(`AIza[0-9A-Za-z_-]{35}`)},
	{"a stripe key", regexp.MustCompile(`sk_(live|test)_[0-9A-Za-z]{16,}`)},
	{"a npm token", regexp.MustCompile(`npm_[A-Za-z0-9]{36}`)},
	{"a bearer literal", regexp.MustCompile(`Bearer [A-Za-z0-9._-]{24,}`)},
	{"a password in a url", regexp.MustCompile(`://[A-Za-z0-9._%+-]+:[^@/[:space:]]{6,}@`)},
}

// --- 3. public-only: fingerprints of a private system ------------------------

var emailRe = regexp.MustCompile(`[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`)

var hexRe = regexp.MustCompile(`\b[0-9a-f]{24,}\b`)

// hexAllow blanks one allowed item on a line. ⛔ Whatever long hex survives all
// of them is a finding; nothing here drops a line.
type hexAllow struct {
	re   *regexp.Regexp
	repl string
}

// ⚠ Narrowed, not switched off. A pinned GitHub Action is a 40-hex commit on a
// PUBLIC repository, and pinning is the SAFE practice: a tag moves and a moved
// tag runs unreviewed code. A rule that fires on correct hardening is a rule
// somebody disables, so each shape below is excluded BY NAME and narrowly.
//
// ⛔ THE TRAILING CLASS ON THE INFOHASH EXPRESSION IS THE ANCHOR AND IT IS NOT
// DECORATION. Without it `{40}` matches the first forty characters of a LONGER
// run and blanks them, leaving a remainder too short to reach the threshold: a
// forty-six digit value after the field name went unreported, and so did a
// sixty-three. Both shell twins agreed, and both were wrong.
//
// ⛔ AND THE ORDER IS THE SHELL HALF'S ORDER. `sed -e ... -e ...` applies each
// expression to the line the previous one produced, so two expressions that can
// both match the same span are not commutative. Reordering them here would be a
// verdict changed by a port.
var hexAllows = []hexAllow{
	{regexp.MustCompile(`uses:[[:space:]]*[A-Za-z0-9._-]+/[A-Za-z0-9._-]+@[0-9a-f]{40}`), "uses: ALLOWED"},
	{regexp.MustCompile(`([Pp]inned(Ref|Sha256|Commit|Digest)|PINNED_(REF|SHA256))([^0-9a-f]*)[0-9a-f]{24,}`), "${1}${4}ALLOWED"},
	{regexp.MustCompile(`^(.*Cargo\.lock:[0-9]+:checksum = )"[0-9a-f]{64}"$`), `${1}"ALLOWED"`},
	{regexp.MustCompile(`(record:)?sha256:[0-9a-f]{64}`), "ALLOWED"},
	{regexp.MustCompile(`"(value|bytes|alphabet|detail)": "[0-9a-f]+"`), `"${1}": "ALLOWED"`},
	{regexp.MustCompile(`(peer_id|HexBytes::parse)\("[0-9a-f]+"`), `${1}("ALLOWED"`},
	{regexp.MustCompile(`(RFC[0-9]+_[A-Z0-9_]+: &str = )"[0-9a-f]{40}"`), `${1}"ALLOWED"`},
	{regexp.MustCompile(`(MSE_[A-Z0-9_]+: &str = )"[0-9a-f]{192}"`), `${1}"ALLOWED"`},
	{regexp.MustCompile(`([Ii]nfohash: )[0-9a-f]{40}([^0-9a-f]|$)`), "${1}ALLOWED${2}"},
	{regexp.MustCompile("(info hash `)[0-9a-f]{40}`"), "${1}ALLOWED`"},
	{regexp.MustCompile("(peer[ _][Ii][Dd][^`]*`)[0-9a-f]{40}`"), "${1}ALLOWED`"},
}

// ⚠ Narrowed rather than switched off. `/home/linuxbrew/` and `/home/runner/`
// are well-known generic paths, not a fingerprint of anybody's machine, and a
// check that fires on them is one somebody disables. Whenever this produces a
// false positive, add the generic path here; do not widen it to the whole rule.
var (
	homeRe      = regexp.MustCompile(`([A-Za-z]:[\\/]Users[\\/]|/home/|/Users/)[A-Za-z0-9._-]+`)
	homeGeneric = []*regexp.Regexp{
		regexp.MustCompile(`/home/(linuxbrew|runner|user|vagrant|ubuntu|node)/`),
		regexp.MustCompile(`/Users/(runner|user)/`),
	}
)

// --- 4. the whole history, on request ----------------------------------------
//
// ⚠ The history scan knows three shapes rather than nine. That is the shell
// half's list and it is inherited: a blob-by-blob read of every commit is
// minutes, and the three here are the ones whose presence is never legitimate.
var blobSecretRe = regexp.MustCompile(`BEGIN (RSA |OPENSSH |EC |DSA |PGP )?PRIVATE KEY|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{30,}`)

// finding is one category with at least one hit. ⛔ The JSON counts CATEGORIES
// and not hits, which is the shell half's `FOUND` and is what check-twins
// compared for as long as both halves existed.
type finding struct {
	title string
	lines []string
}

func checkNoSecrets(r *repo) (verdict, error) {
	files := r.files()
	var found []finding
	hit := func(title string, lines []string) {
		if len(lines) > 0 {
			found = append(found, finding{title, lines})
		}
	}

	// 1. a credential file is tracked.
	var creds []string
	for _, f := range files {
		if credFileRe.MatchString(f) && !credWaived.MatchString(f) {
			creds = append(creds, f)
		}
	}
	hit("a credential file is tracked", creds)

	// 2 and 3. one read per file, every rule over each line.
	//
	// ⚠ The shell half ran one `grep` per pattern and therefore read the tree
	// once per rule. Reading it once and asking every rule per line is the same
	// question asked in a different order; what it may NOT change is which lines
	// each rule reports, which is why the per-rule lists are kept separate and
	// in the shell half's order rather than merged into one pass's output.
	contentHits := make([][]string, len(contentRules))
	var emailHits, hexHits, homeHits []string

	for _, f := range files {
		raw, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		// ⚠ A BINARY FILE IS SKIPPED, which is what `grep -I` does.
		if bytes.IndexByte(raw, 0) >= 0 {
			continue
		}
		for i, lb := range splitLines(raw) {
			line := string(lb)
			// ⛔ THE OUTPUT LINE IS `path:lineno:text` AND THE ALLOW EXPRESSIONS
			// READ IT. Built once per line and only where a rule has already
			// matched, because forming it for every line of the tree is the one
			// place this port could be slower than the pipeline it replaces.
			out := func() string { return fmt.Sprintf("%s:%d:%s", f, i+1, line) }

			for ri, rule := range contentRules {
				if rule.re.MatchString(line) {
					contentHits[ri] = append(contentHits[ri], out())
				}
			}
			if !optPublic {
				continue
			}
			if emailRe.MatchString(line) {
				emailHits = append(emailHits, out())
			}
			if hexRe.MatchString(line) {
				o := out()
				for _, a := range hexAllows {
					o = a.re.ReplaceAllString(o, a.repl)
				}
				if hexRe.MatchString(o) {
					hexHits = append(hexHits, o)
				}
			}
			if homeRe.MatchString(line) {
				o := out()
				generic := false
				for _, g := range homeGeneric {
					if g.MatchString(o) {
						generic = true
						break
					}
				}
				if !generic {
					homeHits = append(homeHits, o)
				}
			}
		}
	}

	for ri, rule := range contentRules {
		hit(rule.title, contentHits[ri])
	}
	if optPublic {
		hit("an email address", emailHits)
		hit("a long hex identifier", hexHits)
		hit("an absolute home path", homeHits)
	}

	// 4. the whole history, on request.
	if optAllHistory {
		h, err := scanHistory()
		if err != nil {
			return verdict{}, err
		}
		hit("a secret shape in history (rotate first, then decide about the history)", h)
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-no-secrets/1","findings":%d,"public_rules":%t,"history_scanned":%t}`,
			len(found), optPublic, optAllHistory),
	}
	if len(found) > 0 {
		var b strings.Builder
		for _, c := range found {
			fmt.Fprintf(&b, "\n== %s ==\n%s", c.title, strings.Join(c.lines, "\n"))
		}
		v.code = 1
		v.text = b.String() + "\n\n" +
			fmt.Sprintf("⛔ %d category/categories matched.\n\n", len(found)) +
			"If any of it is a real credential, IN THIS ORDER:\n" +
			"  1. ROTATE IT. Now, before anything else. It is compromised from the\n" +
			"     moment it was written, and removing the file does not change that.\n" +
			"  2. Tell the operator. They own the account.\n" +
			"  3. Remove it from the tree, and add the ignore rule.\n" +
			"  4. A history rewrite is the operator's call and the operator's action.\n" +
			"     It is tidying after the fix, not the fix.\n\n" +
			"If it is a false positive, narrow the pattern in this check rather than\n" +
			"switching the check off. See docs/security/secrets.md.\n"
		return v, nil
	}

	pub := ""
	if optPublic {
		pub = " (public rules included)"
	}
	v.text = fmt.Sprintf("no secret shapes found in %d files (tracked plus untracked-not-ignored)%s\n", len(files), pub) +
		"⚠ This finds the shapes it knows. It is not a clearance: read the diff.\n"
	return v, nil
}

// scanHistory reads every blob ever committed.
//
// ⚠ Slow. On a large repository it is minutes rather than seconds. Worth running
// once before a repository is first published, and not on every commit.
//
// ⛔ EVERY FAILURE HERE IS A COULD-NOT-RUN. A history scan that half-ran and
// reported nothing would be the strongest clean bill this check can give, issued
// over a question it never asked.
func scanHistory() ([]string, error) {
	list := exec.Command("git", "rev-list", "--objects", "--all")
	names, err := list.Output()
	if err != nil {
		return nil, errCannotRun("cannot list the history")
	}

	// `git cat-file --batch-check` takes `<object> <rest>` per line and echoes
	// `%(rest)` back, which is how the path travels with the object name.
	batch := exec.Command("git", "cat-file", "--batch-check=%(objecttype) %(objectname) %(rest)")
	batch.Stdin = bytes.NewReader(names)
	typed, err := batch.Output()
	if err != nil {
		return nil, errCannotRun("cannot read the history's objects")
	}

	var out []string
	sc := bufio.NewScanner(bytes.NewReader(typed))
	sc.Buffer(make([]byte, 0, 64*1024), 4*1024*1024)
	for sc.Scan() {
		fields := strings.SplitN(sc.Text(), " ", 3)
		if len(fields) < 2 || fields[0] != "blob" {
			continue
		}
		sha := fields[1]
		path := ""
		if len(fields) == 3 {
			path = fields[2]
		}
		blob, err := exec.Command("git", "cat-file", "blob", sha).Output()
		if err != nil {
			continue
		}
		// ⚠ `grep -qI` skips a binary blob rather than matching inside it.
		if bytes.IndexByte(blob, 0) >= 0 {
			continue
		}
		if blobSecretRe.Match(blob) {
			out = append(out, fmt.Sprintf("%s  %s", sha, path))
		}
	}
	return out, nil
}
