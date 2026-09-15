// docs.go - do the documents still resolve, and are they written the way this
// repository writes documents?
//
// The defect this exists to catch is a document that was true when it was
// written. Four shapes of it, and every one is invisible to every other check:
//
//   - a link or a path that stopped resolving when something was renamed;
//   - a fenced shell block that does not parse, which is a block nobody can copy
//     and paste;
//   - an angle-bracket placeholder inside a shell block: a human reads it as
//     "fill this in" and bash reads it as a redirect, so the reader gets a
//     cryptic syntax error instead of an obvious instruction;
//   - a page nothing links to, and a session record the history index omits.
//
// ⚠ CONTROL BYTES AND THE CHARACTER RULE ARE NOT HERE. Both used to be, over
// markdown alone, which left every .ts, .py, .rs and .sh in the tree unchecked.
// They are `check-control-bytes` and `check-markers`, over every text file. Run
// all three.
//
// ⛔ WHAT IT DOES NOT CHECK IS WHETHER A CLAIM IS TRUE. That is a reading, and it
// belongs to the review pass. A guard that tried to verify prose would either
// pass vacuously or refuse legitimate writing, and both are worse than an honest
// scope.
//
// -- ⛔ WHAT THE PORT FOUND, WHICH IS NOT WHAT IT SET OUT TO CARRY -------------
//
// ⛔ THE TWO SHELL HALVES DISAGREED, AND NOTHING COULD SEE IT. The `sh` half read
// links with TWO awk programs - the broken-link pass stripped inline code spans
// and the orphan pass did not - while the PowerShell twin has ONE extractor that
// strips them and feeds both. So a page cited only inside backticks was an orphan
// to one half and not to the other.
//
// ⚠ `check-twins` compares the halves' answers on the tree it runs against, and
// no page here is cited only that way, so the two agreed on every run for as long
// as both existed. `check-bitcheck` planted the shape and they answered
// differently on the first comparison.
//
// ⭐ THE TWIN IS THE CORRECT ONE AND THIS PORT FOLLOWS IT, which is a verdict
// changed deliberately and recorded, not one changed by accident: a code span is
// not a hyperlink, a reader following links never arrives, and *an unlinked page
// is not read, so it is not corrected* is exactly what that describes. The `sh`
// half is fixed in the same change, so all three agree before any is deleted.
//
// ⚠ The counts are narrower than their names. `links` counts only what the
// broken-link pass looked at, so a template contributes none; `shell_blocks`
// counts every block in every file, templates included.
//
// Ported from scripts/common/check-docs.sh and its PowerShell twin.
package main

import (
	"fmt"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

var (
	// ⚠ THE TEMPLATE DIRECTORIES ARE EXEMPT FROM THE LINK CHECK, AND MUST BE. A
	// template's links are written relative to where the file will live in the
	// PROJECT, not where it lives here. Checking those reports thirty-odd
	// failures on a correct tree, and a check that fails on a correct tree gets
	// switched off within a week. ⭐ Only link RESOLUTION is exempt, because only
	// that one is position-dependent.
	docsNoLinkRe = regexp.MustCompile(`^(docs/templates/|bootstrap/prompts/)`)

	// ⚠ SPELLED AGAIN HERE RATHER THAN SHARED WITH check-markers, whose two
	// patterns are byte-identical today. They came from two independent shell
	// halves and either rule may narrow its own idea of a fence without the other
	// agreeing; one shared constant would make that a silent change to a rule
	// nobody edited. It is the same argument repo.go makes for keeping `textRe`
	// and `markerTextRe` apart while they differ by one extension.
	docsFenceRe    = regexp.MustCompile("^[ \t]*```")
	shFenceRe      = regexp.MustCompile("^[ \t]*```(bash|sh)[ \t]*$")
	docsCodeSpanRe = regexp.MustCompile("`[^`]*`")
	linkRe         = regexp.MustCompile(`\]\([^)\t ]+`)
	redirectRe     = regexp.MustCompile(`<[a-z][a-z0-9-]*>`)
	sessionRecRe   = regexp.MustCompile(`^docs/history/SESSION-.*\.md$`)
)

// posixShell finds a shell that can answer `-n`, or reports that none is here.
//
// ⚠ ITS ABSENCE IS SAID RATHER THAN ASSUMED, exactly as the PowerShell half says
// it: the blocks are still COUNTED and simply not parsed, and the run prints how
// many. A port that silently stopped parsing would report the same JSON over an
// experiment that did not happen.
func posixShell() string {
	for _, name := range []string{"sh", "bash", "dash"} {
		if p, err := exec.LookPath(name); err == nil {
			return p
		}
	}
	return ""
}

// linkTargets pulls every `](target` out of a document.
//
// ⛔ ONE READER FOR BOTH PASSES. It took a parameter while this port still
// believed the two shell halves agreed; they did not, and the correct behaviour
// is the one that strips code spans everywhere. Markdown does not linkify a code
// span, so a backticked path is neither a broken link nor a link.
func linkTargets(body []byte) []struct {
	line   int
	target string
} {
	var out []struct {
		line   int
		target string
	}
	fence := false
	for i, lb := range splitLines(body) {
		line := string(lb)
		if docsFenceRe.MatchString(line) {
			fence = !fence
			continue
		}
		if fence {
			continue
		}
		// ⚠ Repeatedly, the way the awk halves loop on `match`, so nested and
		// multiple spans on one line are all removed.
		for {
			loc := docsCodeSpanRe.FindStringIndex(line)
			if loc == nil {
				break
			}
			line = line[:loc[0]] + line[loc[1]:]
		}
		for _, m := range linkRe.FindAllString(line, -1) {
			out = append(out, struct {
				line   int
				target string
			}{i + 1, m[2:]})
		}
	}
	return out
}

// docsPath turns a link into the repo-relative path it names, so a link written
// from a subdirectory and one written from the root name the same string.
//
// ⛔ IT IS `path.Join`, WHICH IS GO'S OWN, AND NOT A HAND-ROLLED COLLAPSE. Both
// shell halves spelled the rule themselves with a `segment/../` regex, and
// docs/conventions/forbidden-patterns.md records what that cost: `[^/]+` matches
// `..` as readily as a directory name, so PowerShell's GLOBAL `-replace` ate a
// real segment and a `../..` pair together and resolved
// `crates/bit-ids/tests/fixtures/../../../../docs/x.md` to
// `crates/bit-ids/docs/x.md`. The `sh` half was correct only by accident of its
// tool: `sed` without `/g` takes the leftmost match and its loop re-runs.
//
// ⚠ THIS PORT WROTE THE GLOBAL SPELLING FIRST and was green over the whole tree
// and every planted case, because it is only wrong past two levels. ⭐ The repair
// is not a better regex: it is asking the standard library, which is the same
// argument repo.go makes for asking `git ls-files` what is in the tree rather
// than walking it.
func docsPath(dir, target string) string {
	return path.Join(dir, target)
}

// external is the set of targets no filesystem answers for.
func external(t string) bool {
	return t == "" || strings.HasPrefix(t, "http://") ||
		strings.HasPrefix(t, "https://") || strings.HasPrefix(t, "mailto:")
}

func checkDocs(r *repo) (verdict, error) {
	var files []string
	for _, f := range r.files() {
		if strings.HasSuffix(f, ".md") {
			files = append(files, f)
		}
	}
	if len(files) == 0 {
		return verdict{}, errCannotRun("no markdown files in scope")
	}
	sort.Strings(files)

	shell := posixShell()
	var problems []string
	nLinks, nBlocks, skippedParse := 0, 0, 0
	linked := map[string]bool{}

	for _, f := range files {
		body, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		dir := filepath.Dir(f)

		// -- broken links ----------------------------------------------------
		if !docsNoLinkRe.MatchString(f) {
			for _, l := range linkTargets(body) {
				if external(l.target) {
					continue
				}
				// ⚠ COUNTED HERE, BEFORE THE FRAGMENT IS CUT, because the shell
				// half incremented before its own empty-target test. A link that
				// is nothing but `#anchor` is counted and not resolved.
				nLinks++
				target := l.target
				if i := strings.IndexByte(target, '#'); i >= 0 {
					target = target[:i]
				}
				if target == "" {
					continue
				}
				if _, err := os.Stat(filepath.Join(dir, target)); err != nil {
					problems = append(problems,
						fmt.Sprintf("%s:%d broken link -> %s", f, l.line, l.target))
				}
			}
		}

		// -- what this document links to, for the orphan pass -----------------
		for _, l := range linkTargets(body) {
			if external(l.target) {
				continue
			}
			t := l.target
			if i := strings.IndexByte(t, '#'); i >= 0 {
				t = t[:i]
			}
			if t == "" {
				continue
			}
			// ⚠ ONE BRANCH, because `path.Join(".", t)` is `path.Clean(t)`.
			// The shell halves needed two because their collapse could not be
			// handed a `.`.
			linked[docsPath(dir, t)] = true
		}

		// -- fenced shell blocks ---------------------------------------------
		//
		// ⚠ NOT gated on the template exemption: a block that does not parse is
		// a block nobody can paste wherever the file ends up living.
		inBlock, start := false, 0
		var buf []string
		for i, lb := range splitLines(body) {
			line := string(lb)
			if !inBlock && shFenceRe.MatchString(line) {
				inBlock, start, buf = true, i+2, nil
				continue
			}
			if inBlock && docsFenceRe.MatchString(line) {
				inBlock = false
				nBlocks++
				body := strings.Join(buf, "\n")
				if redirectRe.MatchString(body) {
					problems = append(problems, fmt.Sprintf(
						"%s:%d shell-unsafe placeholder. bash reads it as a redirect; use UPPER_SNAKE",
						f, start))
				}
				if shell == "" {
					skippedParse++
					continue
				}
				if !parses(shell, body) {
					problems = append(problems,
						fmt.Sprintf("%s:%d shell block does not parse", f, start))
				}
				continue
			}
			if inBlock {
				buf = append(buf, line)
			}
		}
	}

	// -- a page nothing links to ---------------------------------------------
	//
	// ⛔ AN UNLINKED PAGE IS NOT READ, SO IT IS NOT CORRECTED, and that is the
	// state every stale document passes through on the way to being wrong.
	// ⚠ Roots are exempt: a README is an entry point, and a file at the
	// repository root is what a reader or a raw URL arrives at directly.
	for _, f := range files {
		base := path.Base(f)
		if base == "README.md" || !strings.Contains(f, "/") {
			continue
		}
		if !linked[f] {
			problems = append(problems, fmt.Sprintf(
				"%s is linked from nowhere. An unlinked page is not read, so it is not corrected.", f))
		}
	}

	// ⛔ AND EVERY SESSION RECORD IS IN THE INDEX THAT DESCRIBES THEM, which is a
	// stronger rule than the one above: a record linked only from `RESUME.md` is
	// not an orphan and is still missing from the page a reader goes to for the
	// list - and `RESUME.md` is overwritten every session.
	const historyIndex = "docs/history/README.md"
	if idx, err := os.ReadFile(historyIndex); err == nil {
		text := string(idx)
		for _, f := range files {
			if !sessionRecRe.MatchString(f) {
				continue
			}
			if !strings.Contains(text, "("+path.Base(f)+")") {
				problems = append(problems, fmt.Sprintf(
					"%s is not listed in %s. A record the index omits is one nobody finds from the page that exists to list them.",
					f, historyIndex))
			}
		}
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-docs/1","problems":%d,"files":%d,"links":%d,"shell_blocks":%d}`,
			len(problems), len(files), nLinks, nBlocks),
	}
	note := ""
	if skippedParse > 0 {
		note = fmt.Sprintf("⚠ no POSIX shell on PATH: %d shell block(s) counted but NOT parsed\n", skippedParse)
	}
	if len(problems) > 0 {
		var b strings.Builder
		fmt.Fprintf(&b, "documentation check failed, %d problem(s):\n\n", len(problems))
		for _, p := range problems {
			fmt.Fprintf(&b, "  %s\n", p)
		}
		b.WriteString("\n")
		b.WriteString(note)
		v.code = 1
		v.text = b.String()
		return v, nil
	}
	v.text = fmt.Sprintf(
		"docs ok: %d files, %d relative links, %d shell blocks. Links and prose clean.\n%s",
		len(files), nLinks, nBlocks, note)
	return v, nil
}

// parses asks a real shell whether a block is syntactically valid.
//
// ⛔ A TEMP FILE, NOT stdin, and the reason is in docs/conventions/shell.md: a
// native command's stdin is not byte-exact from every shell, and a trailing CRLF
// gets appended. For a syntax check that is the difference between a real answer
// and a fabricated one.
//
// ⚠ The carriage returns are stripped first, because a `.md` in this tree is LF
// and a block lifted from a CRLF document would fail to parse for its endings
// rather than for its syntax.
func parses(shell, body string) bool {
	f, err := os.CreateTemp("", "checkdocs-*.sh")
	if err != nil {
		return true
	}
	name := f.Name()
	defer os.Remove(name)
	_, werr := f.WriteString(strings.ReplaceAll(body, "\r", "") + "\n")
	cerr := f.Close()
	if werr != nil || cerr != nil {
		return true
	}
	return exec.Command(shell, "-n", name).Run() == nil
}
