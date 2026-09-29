// remoteitems.go - what is open against this repository, and does it say
// anything that survives being checked?
//
// Ported from scripts/common/check-remote-items.sh and its PowerShell twin,
// CI-10.
//
// The defect this exists to catch is a change accepted on the strength of its
// own description. A bot's pull request title says what it believes it is
// doing, and a contributor's issue what they believe is wrong. Both are CLAIMS,
// and both are usually right, which is exactly what makes the wrong one
// expensive: nobody is looking by the hundredth bump.
//
// ⭐ THIS WAS PAID FOR ON THIS REPOSITORY, TWICE, IN ONE HOUR. `actions/checkout`
// was pinned to a major that targets a runtime GitHub had deprecated, and its
// replacement was chosen by looking at two majors while a third already existed.
// Resolving a tag is not checking what it declares, and a tag resolving cleanly
// says nothing about whether it is current.
//
// -- WHAT IT VERIFIES, AND IT DOES NOT TAKE THE ITEM'S WORD ------------------
//
// For every pinned action a pull request ADDS: that the commit exists in the
// repository the ref names; that the tag in the trailing comment resolves to that
// commit; that the runtime the pinned commit DECLARES is not a deprecated one;
// and whether a newer release is already out.
//
// ⛔ IT IS READ ONLY. It never merges, closes, comments or approves; deciding is
// the operator's. docs/security/remote-ops.md.
//
// ⚠ AN UNREAD ITEM IS NOT A FAILED CHECK. An item needing a reading is counted,
// named, and exits 0; only a claim that was checked and did not hold exits 1.
//
// ⛔ IT ASKS `gh` AND `curl`, AS ITS SHELL HALF DID, rather than speaking HTTP
// itself. GraphQL and authenticated routes stay with `gh` under `AGENTS.md` rule
// 8, and one program per question is also what lets a stub on `PATH` stand in
// for both in `check-bitcheck`, so the cases run with no network at all.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os/exec"
	"regexp"
	"strings"
)

// optRepo is `--repo OWNER/NAME`, handed to every `gh` call that takes one.
var optRepo string

type ghAuthor struct {
	Login string `json:"login"`
}

type ghIssue struct {
	Number int      `json:"number"`
	Title  string   `json:"title"`
	Author ghAuthor `json:"author"`
}

type ghPull struct {
	Number int      `json:"number"`
	Title  string   `json:"title"`
	Author ghAuthor `json:"author"`
	Files  []struct {
		Path string `json:"path"`
	} `json:"files"`
}

// pinRe is the shell half's `grep -oE`, and every match on a line is taken.
var pinRe = regexp.MustCompile(`uses:[[:space:]]*[A-Za-z0-9._-]+/[A-Za-z0-9._-]+@[0-9a-f]{40}([[:space:]]*#[[:space:]]*[^[:space:]]+)?`)

var (
	pinActionRe = regexp.MustCompile(`^uses:[[:space:]]*`)
	pinShaRe    = regexp.MustCompile(`@([0-9a-f]{40})`)
	pinTagRe    = regexp.MustCompile(`#[[:space:]]*([^[:space:]]+)`)
	usingRe     = regexp.MustCompile(`^[[:space:]]*using:[[:space:]]*(.+)$`)
)

// gh runs `gh` and answers its stdout with every carriage return removed, and
// whether it exited 0.
//
// ⛔ gh ON WINDOWS EMITS CRLF, and a carriage return riding on a value is
// invisible until something types it. Every value read out of gh is stripped.
func gh(args ...string) (string, string, bool) {
	cmd := exec.Command("gh", args...)
	var out, errb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &errb
	err := cmd.Run()
	return strings.ReplaceAll(out.String(), "\r", ""), errb.String(), err == nil
}

func withRepo(args ...string) []string {
	if optRepo == "" {
		return args
	}
	// ⚠ After the subcommand, where the shell half put it.
	return append(append(append([]string{}, args[:2]...), "--repo", optRepo), args[2:]...)
}

func checkRemoteItems(r *repo) (verdict, error) {
	if needTools("gh") != nil {
		return verdict{}, errCannotRun("gh not found")
	}
	if _, _, ok := gh("auth", "status"); !ok {
		return verdict{}, errCannotRun("gh is not authenticated")
	}

	var rep strings.Builder
	problems, needsHuman := 0, 0
	say := func(s string) { rep.WriteString(s + "\n") }
	note := func(s string) { say("  " + s) }
	bad := func(s string) { say("  ⛔ " + s); problems++ }
	human := func(s string) { say("  ⚠ " + s); needsHuman++ }

	// ⛔ A LISTING THAT FAILS IS COULD-NOT-RUN, after the report so far and with
	// what gh said, in the order the shell half wrote them.
	cannot := func(msg, errText string) (verdict, error) {
		m := "check-remote-items: " + msg + "\n"
		if t := strings.TrimRight(errText, "\n"); t != "" {
			m += t + "\n"
		}
		return verdict{code: 2, text: rep.String(), jsonReport: rep.String(), stderr: m}, nil
	}

	// -- open issues ----------------------------------------------------------
	// ⚠ Reported, not judged. An issue is a person's account of a problem and
	// nothing here can verify it; what this can do is stop one going unnoticed.
	say("")
	say("OPEN ISSUES")
	out, errText, ok := gh(withRepo("issue", "list", "--state", "open", "--limit", "50",
		"--json", "number,title,author,createdAt")...)
	if !ok {
		return cannot("could not list issues", errText)
	}
	var issues []ghIssue
	if json.Unmarshal([]byte(out), &issues) != nil {
		return cannot("could not list issues", "")
	}
	if len(issues) == 0 {
		note("none")
	} else {
		for _, i := range issues {
			say(fmt.Sprintf("  #%d [%s] %s", i.Number, i.Author.Login, i.Title))
		}
		human(fmt.Sprintf("%d open issue(s). Read them; nothing here can verify a report.", len(issues)))
	}

	// -- open pull requests ---------------------------------------------------
	say("")
	say("OPEN PULL REQUESTS")
	out, errText, ok = gh(withRepo("pr", "list", "--state", "open", "--limit", "50",
		"--json", "number,title,author,headRefName,files")...)
	if !ok {
		return cannot("could not list pull requests", errText)
	}
	var pulls []ghPull
	if json.Unmarshal([]byte(out), &pulls) != nil {
		return cannot("could not list pull requests", "")
	}

	if len(pulls) == 0 {
		note("none")
	}
	for _, pr := range pulls {
		say("")
		say(fmt.Sprintf("  #%d [%s] %s", pr.Number, pr.Author.Login, pr.Title))

		diff, _, ok := gh(withRepo("pr", "diff", fmt.Sprint(pr.Number))...)
		if !ok {
			human(fmt.Sprintf("#%d: could not read the diff", pr.Number))
			continue
		}

		// Every action pin the diff ADDS. The trailing comment is captured too,
		// because a pin whose label disagrees with it is its own defect.
		var pins []string
		for _, line := range strings.Split(diff, "\n") {
			if strings.HasPrefix(line, "+") {
				pins = append(pins, pinRe.FindAllString(line, -1)...)
			}
		}
		if len(pins) == 0 {
			var paths []string
			for _, f := range pr.Files {
				paths = append(paths, f.Path)
			}
			note("touches: " + strings.Join(paths, ", "))
			human(fmt.Sprintf("#%d: nothing mechanically checkable here. Read it.", pr.Number))
			continue
		}

		for _, pin := range pins {
			action := pinActionRe.ReplaceAllString(pin, "")
			action = action[:strings.Index(action, "@")]
			sha := pinShaRe.FindStringSubmatch(pin)[1]
			tag := ""
			if m := pinTagRe.FindStringSubmatch(pin); m != nil {
				tag = m[1]
			}
			label := tag
			if label == "" {
				label = "no label"
			}
			say(fmt.Sprintf("    %s@%s  (labelled %s)", action, sha[:12], label))

			// 1. does the commit exist, and in THAT repository?
			if _, _, ok := gh("api", "repos/"+action+"/commits/"+sha, "--jq", ".sha"); !ok {
				bad(action + "@" + sha + " does not exist in that repository. A pin naming a commit the repo does not have is not a bump.")
				continue
			}
			note("      commit exists in " + action)

			// 2. does the label resolve to that same commit?
			if tag != "" {
				tSha, _, _ := gh("api", "repos/"+action+"/git/ref/tags/"+tag, "--jq", ".object.sha")
				tTyp, _, _ := gh("api", "repos/"+action+"/git/ref/tags/"+tag, "--jq", ".object.type")
				tSha, tTyp = strings.TrimSpace(tSha), strings.TrimSpace(tTyp)
				if tTyp == "tag" {
					tSha, _, _ = gh("api", "repos/"+action+"/git/tags/"+tSha, "--jq", ".object.sha")
					tSha = strings.TrimSpace(tSha)
				}
				switch {
				case tSha == "":
					human("      the label " + tag + " is not a tag in " + action)
				case tSha != sha:
					bad("the label says " + tag + " but that tag is " + firstN(tSha, 12) + ", not the pinned commit. The comment has drifted from the pin.")
				default:
					note("      label " + tag + " matches the pin")
				}
			} else {
				human("      no tag comment beside the pin. A bare SHA tells a reader nothing.")
			}

			// 3. ⭐ what runtime does the PINNED COMMIT declare? This is the check
			//    the Node 20 deprecation got past.
			rt := declaredRuntime(action, sha)
			switch rt {
			case "":
				human("      could not read action.yml at that commit; runtime unverified")
			case "node12", "node16", "node20":
				bad("it declares " + rt + ", which GitHub has deprecated. It will run under a forced newer runtime, with a warning nobody reads, until it does not.")
			case "node24", "docker", "composite":
				note("      runtime: " + rt)
			default:
				human("      runtime: " + rt + " (unrecognised; check it)")
			}

			// 4. is anything newer already out?
			latest, _, _ := gh("api", "repos/"+action+"/releases/latest", "--jq", ".tag_name")
			latest = strings.TrimSpace(latest)
			if latest != "" && tag != "" && latest != tag {
				human("      " + latest + " is already released; this proposes " + tag)
			} else if latest != "" {
				note("      " + latest + " is the latest release")
			}
		}
	}

	// ⛔ THE TWO MODES REPORT THE SAME VERDICT. They differed once: text exited 1
	// whenever anything needed a reading and json exited 0 over the same items.
	say("")
	code := 0
	switch {
	case problems > 0:
		say(fmt.Sprintf("⛔ %d claim(s) did not survive checking. Do not merge on the description.", problems))
		code = 1
	case needsHuman > 0:
		say(fmt.Sprintf("⚠ %d item(s) need a reading. Nothing failed a check; nothing was verified either.", needsHuman))
	default:
		say("✅ every mechanically checkable claim held.")
		say("⚠ That is not approval. Whether you want a change is a reading, not a check.")
	}

	return verdict{
		code: code,
		text: rep.String(),
		// ⛔ IN JSON MODE, STDOUT IS RESERVED FOR THE DOCUMENT and the report goes
		// to stderr, where a human reading a terminal still sees it.
		jsonReport: rep.String(),
		json: fmt.Sprintf(`{"schema":"check-remote-items/1","problems":%d,"needs_human":%d,"open_prs":%d}`,
			problems, needsHuman, len(pulls)),
	}, nil
}

// declaredRuntime reads `runs: using:` out of an action's own action.yml at a
// commit, through `curl` as the shell half did.
//
// ⚠ THE DECLARED VALUE MAY BE QUOTED. `using: "node24"` is valid YAML and real
// actions write it that way; before the shell half stripped quotes, a quoted
// "node20" matched nothing and was reported as unrecognised instead of refused.
func declaredRuntime(action, sha string) string {
	cmd := exec.Command("curl", "-sSL", "-m", "20", "https://raw.githubusercontent.com/"+action+"/"+sha+"/action.yml")
	var out bytes.Buffer
	cmd.Stdout = &out
	_ = cmd.Run()

	// `sed -n '/^runs:/,/^[^ ]/p'`: from a `runs:` line to the next line that
	// begins with anything but a space, both included - and a range that closes
	// can open again at a later `runs:`, as a sed range does. Then the first
	// `using:` among the lines printed.
	// ⚠ A line is cut on "\n" alone, so a CRLF blank line is "\r" and closes the
	// range, exactly as it does for the shell half's `sed`.
	in := false
	rt := ""
	for _, line := range strings.Split(out.String(), "\n") {
		if !in {
			if !strings.HasPrefix(line, "runs:") {
				continue
			}
			in = true
		} else if line != "" && line[0] != ' ' {
			in = false
		}
		if m := usingRe.FindStringSubmatch(line); m != nil {
			rt = m[1]
			break
		}
	}
	rt = strings.NewReplacer(`"`, "", "'", "", "\r", "").Replace(rt)
	return strings.TrimSpace(rt)
}

func firstN(s string, n int) string {
	if len(s) < n {
		return s
	}
	return s[:n]
}
