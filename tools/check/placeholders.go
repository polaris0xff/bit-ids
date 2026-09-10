// placeholders.go - did a template placeholder survive into a real file?
//
// Four categories, each one a sentence that looks authoritative and says
// nothing. ⚠ Deliberately narrow: a rule that fires on correct writing is a rule
// somebody switches off within a week, and `example.com` in a public document is
// the CORRECT thing to write.
//
// ⛔ EVERY IMPLEMENTATION OF THIS CHECK IS EXEMPT FROM IT, because each one
// contains the patterns it looks for. Exempting only one is how the halves
// disagree, and it did: the sh side scanned the new ps1 twin and reported four
// categories the ps1 side did not. ⚠ This file is the third such implementation
// and is exempt for the same reason, in all three.
//
// Ported from scripts/common/check-placeholders.sh and its PowerShell twin.
package main

import (
	"bytes"
	"fmt"
	"os"
	"regexp"
	"strings"
)

// ⚠ `${{ }}` is GitHub Actions expression syntax and `{{.Field}}` is a Go
// template, so neither is a placeholder. A rule that fired on either would fire
// on every correct workflow file and on every `docker inspect --format` string.
// ⭐ The exclusion is "a dot or a lowercase letter", because every placeholder
// this template ships begins with an UPPERCASE letter and a Go template calls
// functions - `{{json .X}}`, `{{range .X}}`, `{{end}}` - which do not.
var (
	braceRe     = regexp.MustCompile(`\{\{`)
	actionsRe   = regexp.MustCompile(`\$\{\{`)
	goTemplRe   = regexp.MustCompile(`\{\{ *[a-z.]`)
	guidanceRe  = regexp.MustCompile(`<!-- *TEMPLATE|delete this comment|Fill every`)
	standInRe   = regexp.MustCompile(`YOUR_(NAME|EMAIL|PROJECT|TOKEN)|CHANGEME|<your-|TODO: fill`)
	ownerRepoRe = regexp.MustCompile(`OWNER/REPO`)
)

func checkPlaceholders(r *repo) (verdict, error) {
	// ⛔ THE TEMPLATE EXEMPTION IS CONDITIONAL, and `bootstrap/BOOTSTRAP.md` is
	// the marker rather than the directory: an empty `bootstrap/` is not tracked
	// by git and a stray one is not evidence of anything. A directory-shaped
	// exemption inherited by a project grants itself to whatever lands in that
	// directory, and a real project kept `docs/templates/` whole with every
	// marker unfilled while this reported the tree clean.
	templatesExempt := false
	if _, err := os.Stat("bootstrap/BOOTSTRAP.md"); err == nil {
		templatesExempt = true
	}
	exempt := regexp.MustCompile(`^(dotfiles/|scripts/common/check-placeholders\.(sh|ps1)|tools/check/placeholders\.go)`)
	if templatesExempt {
		exempt = regexp.MustCompile(`^(docs/templates/|dotfiles/|bootstrap/|scripts/common/check-placeholders\.(sh|ps1)|tools/check/placeholders\.go)`)
	}

	// ⚠ NOT stat-filtered, because the shell half was not. Its file count is the
	// length of the list it hands to grep, which includes a tracked-but-deleted
	// path; matching that keeps `files_scanned` comparable.
	var files []string
	for _, f := range r.files() {
		if !exempt.MatchString(f) {
			files = append(files, f)
		}
	}
	if len(files) == 0 {
		return verdict{}, errCannotRun("no files in scope")
	}

	type category struct {
		title string
		hits  []string
	}
	cats := []*category{
		{title: "a placeholder survived"},
		{title: "a template guidance comment survived"},
		{title: "a stand-in value survived"},
		{title: "OWNER/REPO survived in a configuration file"},
	}

	for _, f := range files {
		raw, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		// ⚠ A BINARY FILE IS SKIPPED, which is what `grep -I` does. It stays in
		// the scanned count, because the shell half counted the list rather than
		// what grep managed to read.
		if bytes.IndexByte(raw, 0) >= 0 {
			continue
		}
		isMD := strings.HasSuffix(f, ".md")
		for i, lb := range splitLines(raw) {
			line := string(lb)
			hit := func(c int) {
				cats[c].hits = append(cats[c].hits, fmt.Sprintf("%s:%d:%s", f, i+1, line))
			}
			// ⛔ THE TWO EXCLUSIONS ARE APPLIED TO THE WHOLE LINE, not to the
			// match, because the shell half piped through `grep -v`. That is the
			// allowlist-on-the-line shape docs/conventions/forbidden-patterns.md
			// records, and reproducing it is the point: a port that narrowed it to
			// the matched item would refuse a line its predecessor accepted.
			if braceRe.MatchString(line) && !actionsRe.MatchString(line) && !goTemplRe.MatchString(line) {
				hit(0)
			}
			if guidanceRe.MatchString(line) {
				hit(1)
			}
			if standInRe.MatchString(line) {
				hit(2)
			}
			// ⚠ `OWNER/REPO` is the RECOMMENDED generic for a public document, so
			// it is a defect only where it is configuration rather than prose.
			if !isMD && ownerRepoRe.MatchString(line) {
				hit(3)
			}
		}
	}

	count := 0
	var report strings.Builder
	for _, c := range cats {
		if len(c.hits) == 0 {
			continue
		}
		count++
		fmt.Fprintf(&report, "\n== %s ==\n", c.title)
		for _, h := range c.hits {
			fmt.Fprintf(&report, "%s\n", h)
		}
	}

	v := verdict{
		json: fmt.Sprintf(`{"schema":"check-placeholders/1","categories":%d,"files_scanned":%d}`,
			count, len(files)),
	}
	if count > 0 {
		v.code = 1
		v.text = report.String() + "\n\n" +
			fmt.Sprintf("⛔ %d category/categories survived into real files.\n\n", count) +
			"Each one is a sentence that looks authoritative and says nothing.\n" +
			"Fill it in, or delete the section it is in. ⚠ Do not delete the\n" +
			"placeholder alone and leave the sentence around it: that produces a\n" +
			"claim nobody wrote.\n"
		return v, nil
	}
	note := "dotfiles is exempt; docs/templates is IN SCOPE because bootstrap/ has gone"
	if templatesExempt {
		note = "docs/templates, dotfiles and bootstrap are exempt"
	}
	v.text = fmt.Sprintf("no placeholders survived in %d files (%s)\n", len(files), note)
	return v, nil
}
