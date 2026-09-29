// minerepo.go - fetch everything a reference sweep needs, and KEEP it.
//
// Ported from scripts/common/mine-repo.sh and its PowerShell twin, CI-10.
//
// ⭐ A HELPER, NOT A CHECK. It writes, so it is not in the `checks` map, it is no
// gate row, and `--rows` does not name it: `bit-check mine-repo` is dispatched
// before the map is read. It keeps the exit-code rule - 0 the subject was
// fetched, 1 it was not, 2 could not run.
//
// -- THE DEFECT THIS EXISTS TO CATCH -----------------------------------------
//
// ⛔ TWO SWEEPS, TWO WAYS OF LOSING THE SAME WORK, BOTH OBSERVED. One cloned
// eleven repositories, wrote its conclusions and kept none of the trees, so the
// next session that wanted to check a citation had to clone all eleven again.
// One wrote its own issue and pull request fetchers, produced real JSON, and
// deleted the JSON and the fetchers on the way out.
//
// ⭐ Both are the same defect: the DERIVED file was treated as the product and
// the EVIDENCE as scratch. A conclusion nobody can re-check is an opinion, and
// the cost of re-fetching is paid by every later session rather than once.
//
// -- THE TWO ROUTES, PROBED RATHER THAN ASSUMED ------------------------------
//
// ⚠ `gh` HAS BEEN PRESENT, ON PATH, AND HOLDING A DEAD TOKEN: a live run got
// `Bad credentials` from a CLI that had just been found. So the probe is
// `gh auth status` AND a real API call. Otherwise it takes the public proxy.
//
// ⚠ THREE THINGS ABOUT THAT PROXY WERE MEASURED ON 2026-08-28:
//   - ⛔ IT IS NOT UNAUTHENTICATED. It makes authenticated requests on behalf of
//     the PkgForge account. It carries none of YOUR credentials, which is not
//     the same as being unable to reach a private repository.
//   - ⚠ Its route set is wider than /repos/*: /users/*, /orgs/*, /search/* and
//     /rate_limit answer, and /user, the who-am-I endpoint, is refused.
//   - ⛔ A browser-like or empty user-agent is refused with HTTP 420, which no
//     HTTP library has a branch for. curl's own is sent, explicitly.
//
// ⭐ A 404 IS EVIDENCE ONLY BESIDE A CONTROL. Neither route can see a private
// repository, so a 404 means "not public" and equally "the route is down". A
// known-public control is asked in the same run, and PROVENANCE.md says which.
//
// ⛔ IT ASKS `gh`, `curl` AND `git`, AS ITS HALVES DID, rather than speaking HTTP
// itself: one program per question is what lets a stub on PATH stand in for
// each in `check-bitcheck`, so every route runs there with no network at all.
//
// -- WHAT IT WRITES ------------------------------------------------------------
//
//	<out>/<owner>__<repo>/
//	  PROVENANCE.md         the commit, the date, the route, and what it could
//	                        NOT fetch - a silently skipped source is the failure
//	                        the whole procedure exists to prevent
//	  api/repo.json         metadata
//	  api/issues.json       BOTH STATES, and it holds pull requests too
//	  api/comments.json, api/review-comments.json, api/releases.json, api/tags.json
//	  api/discussions.json  gh only; PROVENANCE.md says when it is absent
//	  tree/                 the clone, stripped, with the commit already captured
//
// ⛔ READS ONLY. No write verb reaches either route. docs/security/remote-ops.md.
//
// ⚠ A RELATIVE --out IS UNDER THE REPOSITORY ROOT when this runs inside one,
// which is where the corpus belongs. The halves resolved it from the working
// directory, and a `go run .` starts in `tools/check`.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"time"
)

const (
	mineProxy   = "https://api.gh.pkgforge.dev"
	mineControl = "pkgforge-dev/reverse-proxies"
	// ⛔ THE USER-AGENT IS CURL'S OWN AND IS SENT EXPLICITLY, so no edit can drop
	// it by accident: the proxy answers a browser-like or empty one with 420.
	mineUA = "curl/8"
	// ⚠ THE PROXY IS PAGED BY HAND, and no further than this many pages.
	minePerPage  = 100
	mineMaxPages = 10
)

// mineTargetRe is OWNER/NAME and nothing else. ⚠ The `sh` half accepted any
// string with a slash in it until 2026-09-29, so `a/b/c` was mined into
// `a__b/c`; its twin was strict, and the port is.
var mineTargetRe = regexp.MustCompile(`^[^/]+/[^/]+$`)

// mineJunk is what the trim deletes, by directory name: build output,
// dependency trees and binaries. Source, tests and docs stay.
var mineJunk = map[string]bool{
	"node_modules": true, "target": true, "build": true, "dist": true,
	".next": true, ".venv": true, "__pycache__": true,
}

// ⚠ DISCUSSIONS ARE GRAPHQL ONLY, so this is the one source the proxy, a REST
// route, cannot reach.
const mineDiscussionsQuery = `query($o:String!,$n:String!){ repository(owner:$o,name:$n){ discussions(first:100){ nodes{ number title body createdAt author{login} comments(first:50){ nodes{ body author{login} } } } } } }`

const mineUsage = `bit-check mine-repo - fetch everything a reference sweep needs, and KEEP it.

Usage:
  bit-check mine-repo OWNER/NAME [--out DIR] [--route auto|gh|proxy] [--no-clone] [--json]
  bit-check mine-repo --selftest [--json]    prove the page joiner, offline

Exit codes: 0 the subject was fetched, 1 it was not, 2 could not run.
`

type miner struct {
	jsonOut bool
	route   string
	gaps    []string
}

func (m *miner) say(s string) {
	if !m.jsonOut {
		fmt.Println(s)
	}
}

// gap records a source this fetch did NOT get. ⛔ Every one reaches
// PROVENANCE.md; a source missing without being named reads exactly like a
// source that had nothing in it.
func (m *miner) gap(s string) { m.gaps = append(m.gaps, "  - "+s) }

// mineRepo is `bit-check mine-repo`, and it answers its own exit code.
func mineRepo(args []string) int {
	m := &miner{}
	target, out, route := "", "references", "auto"
	clone, selftest := true, false
	for i := 0; i < len(args); i++ {
		switch a := args[i]; a {
		case "--out", "--route":
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "mine-repo: %s needs a value\n", a)
				return 2
			}
			i++
			if a == "--out" {
				out = args[i]
			} else {
				route = args[i]
			}
		case "--no-clone":
			clone = false
		case "--json":
			m.jsonOut = true
		case "--selftest":
			selftest = true
		case "-h", "--help":
			fmt.Print(mineUsage)
			return 0
		default:
			// ⛔ A SECOND TARGET IS REFUSED, not taken. The `sh` half kept the
			// last one it was given, so `a/b c/d` mined only `c/d` and said so
			// nowhere; its twin's parameter binder refused it with exit 1.
			if strings.HasPrefix(a, "-") || target != "" {
				fmt.Fprintf(os.Stderr, "mine-repo: unknown argument: %s\n", a)
				return 2
			}
			target = a
		}
	}

	// ⭐ --selftest RUNS BEFORE EVERYTHING ELSE. It needs no target, no network
	// and no credential, so requiring any of them would make the one part of
	// this that can be proved the part hardest to run.
	if selftest {
		return m.selftest()
	}

	// ⛔ AN UNKNOWN ROUTE IS REFUSED, NOT PROBED. The `sh` half read anything but
	// `gh` and `proxy` as `auto`, so `--route prxy` - asked for precisely to keep
	// the operator's token out of it - probed gh and used the token.
	switch route {
	case "auto", "gh", "proxy":
	default:
		fmt.Fprintf(os.Stderr, "mine-repo: the route is auto, gh or proxy, not: %s\n", route)
		return 2
	}
	if !mineTargetRe.MatchString(target) {
		fmt.Fprintln(os.Stderr, "mine-repo: give a target as OWNER/NAME")
		return 2
	}
	for _, t := range []string{"curl", "git"} {
		if _, err := exec.LookPath(t); err != nil {
			fmt.Fprintf(os.Stderr, "mine-repo: %s not found\n", t)
			return 2
		}
	}
	if !filepath.IsAbs(out) {
		if top, err := exec.Command("git", "rev-parse", "--show-toplevel").Output(); err == nil {
			if t := strings.TrimSpace(string(top)); t != "" {
				_ = os.Chdir(t)
			}
		}
	}
	return m.fetch(target, out, route, clone)
}

func (m *miner) fetch(target, out, route string, clone bool) int {
	owner, name, _ := strings.Cut(target, "/")
	dest := out + "/" + owner + "__" + name
	if os.MkdirAll(dest+"/api", 0o755) != nil {
		fmt.Fprintf(os.Stderr, "mine-repo: cannot write to %s\n", dest)
		return 2
	}

	// ⛔ REFUSE TO WRITE INTO A DIRECTORY THIS REPOSITORY'S OWN IGNORE RULES WOULD
	// SWALLOW. The corpus is the evidence; an ignored one exists on one machine,
	// and every claim built on it is unsourced the moment that machine is not
	// the one asking. A `references/` ignore rule once shipped in this
	// template's own dotfiles for exactly the reasoning this refuses.
	// ⚠ It fires late enough to have made the directory and early enough to have
	// fetched nothing, and it asks about the DIRECTORY, because a rule may name
	// the directory rather than its contents.
	if exec.Command("git", "-C", filepath.Dir(dest), "rev-parse", "--show-toplevel").Run() == nil &&
		exec.Command("git", "check-ignore", "-q", dest).Run() == nil {
		fmt.Fprintf(os.Stderr, "mine-repo: %s is ignored by this repository.\n", dest)
		fmt.Fprint(os.Stderr, "mine-repo: the corpus IS the evidence. An ignored one is lost on the\n"+
			"mine-repo: next machine, and every citation built on it goes unsourced.\n"+
			"mine-repo: un-ignore it, choose another --out, or put the corpus on its\n"+
			"mine-repo: own branch. docs/methodology/references.md section 4.\n"+
			"mine-repo: the rule that did it:\n")
		rule := exec.Command("git", "check-ignore", "-v", dest)
		rule.Stdout = os.Stderr
		_ = rule.Run()
		return 2
	}

	if route == "auto" {
		route = "proxy"
		if _, err := exec.LookPath("gh"); err == nil &&
			exec.Command("gh", "auth", "status").Run() == nil &&
			exec.Command("gh", "api", "rate_limit").Run() == nil {
			route = "gh"
		}
	}
	m.route = route
	m.say("route: " + route)

	// -- the control, before any 404 is believed ------------------------------
	var control string
	if route == "proxy" {
		cf := dest + "/api/.control.json"
		c := mineFetchProxy("/repos/"+mineControl, cf)
		_ = os.Remove(cf)
		if c == "200" {
			control = "reachable (" + mineControl + " answered 200)"
		} else {
			control = "⛔ UNREACHABLE (" + mineControl + " answered " + c + "). A 404 below means nothing."
		}
	} else if exec.Command("gh", "api", "repos/"+mineControl).Run() == nil {
		control = "reachable (" + mineControl + " answered)"
	} else {
		control = "⛔ UNREACHABLE (" + mineControl + " did not answer). A 404 below means nothing."
	}
	m.say("control: " + control)

	// -- the subject ----------------------------------------------------------
	m.say("fetching " + target)
	repoFile := dest + "/api/repo.json"
	if route == "gh" {
		if !ghToFile(repoFile, "api", "repos/"+target) {
			fmt.Fprintf(os.Stderr, "mine-repo: could not fetch repos/%s\n", target)
			fmt.Fprintf(os.Stderr, "mine-repo: control says: %s\n", control)
			return 1
		}
	} else if c := mineFetchProxy("/repos/"+target, repoFile); c != "200" {
		fmt.Fprintf(os.Stderr, "mine-repo: proxy returned %s for repos/%s\n", c, target)
		fmt.Fprintf(os.Stderr, "mine-repo: control says: %s\n", control)
		return 1
	}

	// ⛔ BOTH STATES, AND THE ISSUES ENDPOINT RETURNS PULL REQUESTS TOO. The
	// open-issue count in the metadata counts both, so a sweep that does not
	// discriminate on the pull_request field reports a dependency bump as an
	// issue.
	base := "/repos/" + target
	m.fetchList(base+"/issues?state=all", dest+"/api/issues.json", "issues and pull requests")
	m.fetchList(base+"/issues/comments", dest+"/api/comments.json", "comments")
	m.fetchList(base+"/pulls/comments", dest+"/api/review-comments.json", "review comments")
	m.fetchList(base+"/releases", dest+"/api/releases.json", "releases")
	m.fetchList(base+"/tags", dest+"/api/tags.json", "tags")

	// ⛔ Recorded as a gap rather than skipped in silence: discussions are where
	// several projects keep the argument that never made it into an issue.
	disc := dest + "/api/discussions.json"
	if route == "gh" {
		if ghToFile(disc, "api", "graphql", "-f", "query="+mineDiscussionsQuery, "-f", "o="+owner, "-f", "n="+name) {
			m.say("  discussions: ok")
		} else {
			_ = os.Remove(disc)
			m.gap("discussions: the GraphQL query failed, or the repository has them disabled")
			m.say("  discussions: FAILED")
		}
	} else {
		m.gap("discussions: NOT FETCHED. The proxy is a REST route and discussions are GraphQL only. Re-run with an authenticated gh to get them.")
		m.say("  discussions: skipped (proxy cannot reach GraphQL)")
	}

	// -- the tree -------------------------------------------------------------
	commit := "-"
	if clone {
		tree := dest + "/tree"
		_ = os.RemoveAll(tree)
		// ⛔ BOUNDED, AND BY GIT ITSELF RATHER THAN BY A WRAPPER. A clone with no
		// limit against a server that accepts and never answers waits forever:
		// measured on 2026-09-17, an unbounded clone was still waiting at 20
		// seconds and with these two settings git abandoned it at 5. A wrapper
		// was tried first and reverted, because `timeout.exe` on Windows is a
		// PAUSE. ⚠ The numbers are the ones the adapters and
		// `install-rootless.sh` bound a stalled curl with. docs/conventions/shell.md
		// section 9.
		cl := exec.Command("git", "-c", "http.lowSpeedLimit=1024", "-c", "http.lowSpeedTime=60",
			"clone", "--depth", "1", "-q", "https://github.com/"+target+".git", tree)
		if cl.Run() == nil {
			// ⛔ CAPTURED BEFORE THE STRIP: once the git directory is gone the
			// commit is unrecoverable and every line citation is unverifiable.
			if head, err := exec.Command("git", "-C", tree, "rev-parse", "HEAD").Output(); err == nil {
				if h := strings.TrimSpace(string(head)); h != "" {
					commit = h
				}
			}
			m.say("  tree: " + commit)
			_ = os.RemoveAll(tree + "/.git")
			mineTrim(tree)
		} else {
			m.gap("tree: the clone failed. Line citations from this reference cannot be verified.")
			m.say("  tree: FAILED")
		}
	} else {
		m.gap("tree: --no-clone was passed. No source was kept, so no citation can be checked.")
	}

	if err := os.WriteFile(dest+"/PROVENANCE.md", []byte(m.provenance(target, commit, route, control)), 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "mine-repo: cannot write %s/PROVENANCE.md\n", dest)
		return 2
	}

	if m.jsonOut {
		fmt.Printf(`{"schema":"mine-repo/1","target":%s,"route":%s,"commit":%s,"gaps":%d,"dest":%s}`+"\n",
			jsonString(target), jsonString(route), jsonString(commit), len(m.gaps),
			jsonString(strings.ReplaceAll(dest, `\`, "/")))
		return 0
	}
	fmt.Printf("\nmined %s into %s\n", target, dest)
	fmt.Printf("commit %s, route %s, %d gap(s). Read %s/PROVENANCE.md.\n", commit, route, len(m.gaps), dest)
	fmt.Println("⭐ Keep the tree. A conclusion nobody can re-check is an opinion.")
	return 0
}

// fetchList fetches one paginated list endpoint into one JSON array.
//
// ⚠ THE SEPARATOR IS CHOSEN, NOT ASSUMED. A path that already carries a query,
// which `/issues?state=all` does, needs `&`: appending `?` unconditionally sent
// `state=all?per_page=100` and GitHub answered 422 with that string quoted back.
func (m *miner) fetchList(path, outFile, label string) {
	sep := "?"
	if strings.Contains(path, "?") {
		sep = "&"
	}
	// ⚠ `gh api --paginate` joins the pages into one array itself, which is why
	// the page-join defect below never reached this route.
	if m.route == "gh" {
		if ghToFile(outFile, "api", "--paginate", path+sep+"per_page=100") {
			m.say("  " + label + ": ok")
			return
		}
		m.gap(label + ": gh could not fetch " + path)
		m.say("  " + label + ": FAILED")
		return
	}

	// ⚠ THE PROXY IS PAGED BY HAND. A page shorter than per_page is the last one;
	// a page exactly per_page long is followed by another request, because "it
	// returned 100 items" and "there are exactly 100" read the same.
	var pages []string
	page := 1
	for ; page <= mineMaxPages; page++ {
		pf := fmt.Sprintf("%s.page.%d", outFile, page)
		code := mineFetchProxy(fmt.Sprintf("%s%sper_page=%d&page=%d", path, sep, minePerPage, page), pf)
		if code != "200" {
			m.gap(fmt.Sprintf("%s: proxy returned %s on page %d", label, code, page))
			m.say("  " + label + ": http " + code)
			removePages(outFile)
			return
		}
		pages = append(pages, pf)
		if strings.Count(readString(pf), `"url"`) < minePerPage {
			break
		}
	}

	// ⛔ A FULL LAST PAGE AT THE CAP IS A TRUNCATION, AND IT IS NAMED. Both halves
	// stopped after ten pages and said "ok", so a tracker with more than a
	// thousand items arrived cut short with nothing anywhere saying so - the
	// silently skipped source this helper exists to prevent.
	truncated := page > mineMaxPages
	if truncated {
		m.gap(fmt.Sprintf("%s: stopped at %d pages of %d on the proxy route. There may be more, and they are not here.", label, mineMaxPages, minePerPage))
	}
	if m.joinPages(outFile, label, pages) {
		removePages(outFile)
		if truncated {
			m.say("  " + label + ": TRUNCATED")
		} else {
			m.say("  " + label + ": ok")
		}
		return
	}
	m.say("  " + label + ": JOIN FAILED")
}

// joinPages joins list pages into ONE array, with a real parser, each page its
// own document.
//
// ⛔ THE JOINER THIS DESCENDS FROM counted `[` and `]` over the RAW TEXT to
// find the array bounds, which counts the brackets inside string values too,
// and comment bodies are markdown. It was reported by a consumer, not found
// here: measured on 2026-08-30 against one third-party repository, comments
// went from 0 to 202 when it was replaced, and the run printed `comments: ok`
// both times. docs/methodology/references.md calls a comment the one source
// only it has - "the maintainer's ruling is nearly always in a comment".
//
// ⚠ The halves needed python3 or node to parse, and probed each by running it,
// because `command -v python3` on one Windows machine found the Store's App
// Execution Alias. This needs neither.
func (m *miner) joinPages(out, label string, pages []string) bool {
	failed := func() bool {
		m.gap(label + ": the page join failed. Pages are left as " + out + ".page.N")
		return false
	}
	all := []json.RawMessage{}
	for _, p := range pages {
		raw, err := os.ReadFile(p)
		if err != nil {
			return failed()
		}
		var doc json.RawMessage
		if json.Unmarshal(raw, &doc) != nil {
			return failed()
		}
		if t := bytes.TrimSpace(doc); len(t) > 0 && t[0] == '[' {
			var items []json.RawMessage
			if json.Unmarshal(t, &items) != nil {
				return failed()
			}
			all = append(all, items...)
		} else {
			all = append(all, doc)
		}
	}
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", " ")
	if enc.Encode(all) != nil || os.WriteFile(out, buf.Bytes(), 0o644) != nil {
		return failed()
	}

	// ⛔ THE JOIN READS ITS OWN EFFECT BACK. A step that succeeds having written
	// nothing is how the halves' joiner first passed its own self-test.
	if fi, err := os.Stat(out); err != nil || fi.Size() == 0 {
		m.gap(label + ": the join exited 0 and wrote no output. Pages are left as " + out + ".page.N")
		return false
	}
	return m.guardNonEmpty(out, label, pages)
}

// guardNonEmpty refuses an empty join over a page that has records in it.
//
// ⛔ THAT IS A FAILURE, NOT AN EMPTY TRACKER. The original defect was invisible
// because nothing asked: `[]` and "this repository has no comments" are the
// same bytes, and only the input tells them apart. ⭐ A function of its own so
// the self-test drives it directly - a guard reachable only through the thing
// it guards is proved by accident or not at all.
func (m *miner) guardNonEmpty(out, label string, pages []string) bool {
	if strings.Join(strings.Fields(readString(out)), "") != "[]" {
		return true
	}
	for _, p := range pages {
		if strings.Contains(readString(p), `"url"`) {
			m.gap(label + ": the join produced an empty array from a page that has records in it. Pages are left as " + out + ".page.N")
			return false
		}
	}
	return true
}

// selftest drives the joiner and its guard against an oracle, offline.
//
// ⛔ THE FIXTURE CARRIES THE SHAPE THAT BROKE IT: bracket characters inside
// string values, unbalanced, spread over two pages. A fixture of well-formed
// records passes under the old joiner too and proves nothing.
//
// ⚠ IT ASSERTS ON THE JOIN AND THE GUARD ONLY. Paging, the routes and the clone
// are proved by `check-bitcheck`, against stubs.
func (m *miner) selftest() int {
	dir, err := os.MkdirTemp("", "mine-repo-selftest-")
	if err != nil {
		fmt.Fprintln(os.Stderr, "mine-repo: --selftest needs a writable temporary directory")
		return 2
	}
	defer os.RemoveAll(dir)

	p1 := filepath.Join(dir, "p.page.1")
	p2 := filepath.Join(dir, "p.page.2")
	blank := filepath.Join(dir, "blank.page.1")
	joined := filepath.Join(dir, "joined.json")
	empty := filepath.Join(dir, "empty.json")
	for f, body := range map[string]string{
		p1: "[\n" +
			` {"url": "https://example.com/1", "body": "see [the report](https://example.com/r"},` + "\n" +
			` {"url": "https://example.com/2", "body": "log: [ERROR] [WARN] two opened, none closed"},` + "\n" +
			` {"url": "https://example.com/3", "body": "plain"}` + "\n]\n",
		p2:    "[\n" + ` {"url": "https://example.com/4", "body": "]["}` + "\n]\n",
		blank: "[]\n",
		empty: "[]\n",
	} {
		if os.WriteFile(f, []byte(body), 0o644) != nil {
			fmt.Fprintln(os.Stderr, "mine-repo: --selftest needs a writable temporary directory")
			return 2
		}
	}

	cases, failed, note := 0, 0, ""
	check := func(name, want, got string) {
		cases++
		if want == got {
			if !m.jsonOut {
				fmt.Printf("  ok    %s = %s\n", name, got)
			}
			return
		}
		failed++
		note += fmt.Sprintf("%s: expected %s, got %s. ", name, want, got)
		if !m.jsonOut {
			fmt.Printf("  FAIL  %s: expected %s, got %s\n", name, want, got)
		}
	}
	verdict := func(ok bool) string {
		if ok {
			return "accepted"
		}
		return "refused"
	}

	// 1. every record from every page survives the join
	n := "refused"
	if m.joinPages(joined, "selftest", []string{p1, p2}) {
		n = fmt.Sprint(strings.Count(readString(joined), `"url"`))
	}
	check("records-joined", "4", n)

	// 2. the result is ONE array, not several concatenated
	arrays := 0
	for _, line := range strings.Split(readString(joined), "\n") {
		if strings.HasPrefix(line, "[") {
			arrays++
		}
	}
	check("arrays-in-output", "1", fmt.Sprint(arrays))

	// ⛔ 3. the guard refuses an empty join over a page that has records - the
	// state the original defect produced, exactly
	check("empty-over-records", "refused", verdict(m.guardNonEmpty(empty, "selftest-guard", []string{p1})))

	// ⚠ 4. and accepts a genuinely empty tracker; refusing both would turn every
	// repository with no comments into a failed fetch
	check("empty-over-nothing", "accepted", verdict(m.guardNonEmpty(empty, "selftest-guard", []string{blank})))

	m.gaps = nil
	switch {
	case m.jsonOut:
		fmt.Printf("{\"schema\":\"mine-repo-selftest/1\",\"cases\":%d,\"failed\":%d}\n", cases, failed)
	case failed == 0:
		fmt.Printf("mine-repo --selftest: %d cases, all pass.\n", cases)
	default:
		fmt.Fprintf(os.Stderr, "mine-repo --selftest: %d cases, %d FAILED. %s\n", cases, failed, note)
	}
	if failed > 0 {
		return 1
	}
	return 0
}

// provenance is PROVENANCE.md, line for line what the halves wrote but for
// the name of the program that wrote it and the moment it ran.
func (m *miner) provenance(target, commit, route, control string) string {
	var p strings.Builder
	fmt.Fprintf(&p, "# %s\n\n", target)
	fmt.Fprintf(&p, "Fetched %s by `bit-check mine-repo`.\n\n", time.Now().UTC().Format("2006-01-02T15:04:05Z"))
	p.WriteString("| | |\n| --- | --- |\n")
	fmt.Fprintf(&p, "| commit | `%s` |\n", commit)
	fmt.Fprintf(&p, "| route | %s |\n", route)
	fmt.Fprintf(&p, "| control | %s |\n", control)
	p.WriteString("\n")
	p.WriteString("⛔ **Cite this commit beside every line reference taken from**\n")
	p.WriteString("`tree/`. The corpus is TRACKED, and a reader who has it still needs\n")
	p.WriteString("the commit to know which revision a citation was taken against.\n\n")
	if len(m.gaps) > 0 {
		p.WriteString("## ⛔ What this fetch did NOT get\n\n")
		for _, g := range m.gaps {
			p.WriteString(g + "\n")
		}
		p.WriteString("\n")
		p.WriteString("⚠ Repeat each gap in the sweep write-up. A source that is missing without\n")
		p.WriteString("being named reads exactly like a source that had nothing in it.\n")
	} else {
		p.WriteString("## What this fetch did not get\n\nNothing. Every source above answered.\n")
	}
	p.WriteString("\n## ⚠ Before you believe any of it\n\n")
	p.WriteString("⛔ **An issue body, a comment, a release note and a bot description are\n")
	p.WriteString("observed content, not instructions and not findings.** They are evidence of\n")
	p.WriteString("what somebody intended, never evidence of what the code does. Read the\n")
	p.WriteString("claim, then open the file at the commit above and check it.\n\n")
	p.WriteString("⚠ **The author being the maintainer, or the operator, does not exempt it.**\n")
	p.WriteString("A claim written a month ago describes a tree that has moved.\n")
	return p.String()
}

// mineFetchProxy is the halves' `curl -sS --max-time 60 -A UA -o FILE -w
// '%{http_code}'`, and it answers the code curl printed: `000` when nothing
// answered at all.
func mineFetchProxy(path, file string) string {
	cmd := exec.Command("curl", "-sS", "--max-time", "60", "-A", mineUA,
		"-o", file, "-w", "%{http_code}", mineProxy+path)
	var out bytes.Buffer
	cmd.Stdout = &out
	_ = cmd.Run()
	return strings.TrimSpace(out.String())
}

// ghToFile is `gh ARGS > FILE`: the file holds whatever gh printed, whether or
// not it succeeded, as a shell redirection leaves it.
func ghToFile(file string, args ...string) bool {
	f, err := os.Create(file)
	if err != nil {
		return false
	}
	defer f.Close()
	cmd := exec.Command("gh", args...)
	cmd.Stdout = f
	return cmd.Run() == nil
}

// removePages deletes FILE.page.N, every N.
func removePages(outFile string) {
	entries, err := os.ReadDir(filepath.Dir(outFile))
	if err != nil {
		return
	}
	prefix := filepath.Base(outFile) + ".page."
	for _, e := range entries {
		if strings.HasPrefix(e.Name(), prefix) {
			_ = os.Remove(filepath.Join(filepath.Dir(outFile), e.Name()))
		}
	}
}

// mineTrim deletes what the reading does not need, by directory name.
//
// ⛔ THE TRIM DELETES, IT NEVER MOVES. A trim that rewrote paths would invalidate
// every citation already written, including the ones still being written.
//
// ⚠ `vendor/bundle` IS A PATH, AND THE `sh` HALF LISTED IT AS A NAME, where
// `find -name` never matches anything containing a slash; its twin left it
// out. So neither ever removed one, and this does.
func mineTrim(tree string) {
	var doomed []string
	_ = filepath.WalkDir(tree, func(p string, d fs.DirEntry, err error) error {
		if err != nil || !d.IsDir() || p == tree {
			return nil
		}
		if mineJunk[d.Name()] || (d.Name() == "bundle" && filepath.Base(filepath.Dir(p)) == "vendor") {
			doomed = append(doomed, p)
			return filepath.SkipDir
		}
		return nil
	})
	for _, p := range doomed {
		_ = os.RemoveAll(p)
	}
}

// jsonString is a JSON string with no HTML escaping, so a value prints as the
// halves printed it.
func jsonString(s string) string {
	var b bytes.Buffer
	enc := json.NewEncoder(&b)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(s)
	return strings.TrimSuffix(b.String(), "\n")
}
