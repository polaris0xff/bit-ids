# Session 2026-09-08: the first clients, and the four dispatches it took

What the client capture path cost, and the seven defects found by running it
rather than by reading it.

⚠ **This is narrative history and goes stale on purpose.** The current truth is
[`../architecture.md`](../architecture.md) and the entries in
[`../../TODO/`](../../TODO/). Nothing here is amended to match a later tree.

## What was measured

⭐ **This project observed a `BitTorrent` identity from a running build for the
first time**, and then from a second build.

| target | build | route | announces | peer ID on the wire |
| --- | --- | --- | ---: | --- |
| transmission | `4.0.5` | package | 2 | peer ID `2d5452343035302d756435383564356171646f73` |
| qbittorrent | `4.6.3` | package | 1 | peer ID `2d7142343633302d596939654d4d7e38664f7866` |

⛔ **Neither closes its entry, and the four gaps are the same for both.** One
route ran rather than two, so `ACQ-03` has nothing to compare. One connector
observed each, so the overlap is `not_corroborated` by construction. Nothing
wrote a `Profile` into the store. And each Prove names Windows, which is
untouched: every adapter is `sh` with no PowerShell twin.

## The thing that made a client capture possible at all

⛔ **`evidence-bundle` could not be driven by a client, and that was structural
rather than an omission.** Its torrent carries a hard-coded
`http://127.0.0.1:6969/announce`, so the only thing that could reach its observer
was something told the endpoint separately - which `curl` is and a stock build is
not. A client reads the address out of the file or it never announces.

⭐ So `client-capture` generates the torrent with `announce` set to the tracker
endpoint the operating system just handed it. That one change is the whole
difference between a fixture capture and a client one.

⚠ **The peer surface is dialled rather than offered, and the reason is a
constructor.** `TrackerResponse` is cloned into the responder while the lab is
still being built and `Lab` binds port zero, so a tracker answer naming this
lab's own peer port would have to predict one that does not exist yet.

## ⛔ Seven defects, and not one of them was found by reading

### 1. A guard masked by an earlier guard over the same input

`the build announced nothing` was unreachable: silence leaves no bytes, so the
segment guard refuses that run first. The harness found it on its own first run.
The case provokes a connection that reaches the tracker without announcing now.

### 2. A guard that searched a transcript for an English word

The peer-ID check took the observer's reported value unconditionally, and the
observer prints `absent` when an announce carries none. So the guard's verdict
depended on whether the word `absent` happened to appear in a JSON document.

### 3. A rule over one workflow that was not a rule over its sibling

`check-workflow`'s capture block asserted `capture.yml` alone while
`capture-client.yml` was being written beside it - a second file that installs
somebody else's binary and deletes a route, inheriting none of it. It reads every
`capture*.yml` now.

### 4. A check absent from half the matrix

The Windows gate's row list is written by hand. `check-capture-client` went into
the `sh` runner's list, and the Windows lane would have run one row fewer with
nothing anywhere naming the missing one. ⚠ The class - that nothing compares the
two runners' lists - is filed in `CI-07`, because `check-twins` deliberately does
not pair the gate runners.

### 5. No time limit on a product's own command

⛔ **Two of client capture run 1's three jobs sat in the install step for over
half an hour and reported nothing at all.** A hung install is indistinguishable
from a slow one until the job's own timeout kills the runner and takes the log
with it. The bound is in the callers now rather than in each adapter, because a
convention in each adapter is a bound the next adapter forgets.

### 6. An exit code read through a pipe

`LINE=$(... | head -1) || cannot` reads `head`'s status, and `head` exits 0 over
anything - so the refusal was dead code and a build that would not answer reached
the parse instead. ⚠ This repository's oldest stated rule, broken in every
adapter at once, on the day they were written.

### 7. A caller that discarded the diagnosis it then reported missing

`install-client` ran the adapter's `version` under `2>/dev/null`, so an adapter
that said exactly which of its three refusals fired arrived as a bare exit code.

⭐ **Six and seven together are why run 3 could finally print the product's own
words**, and those words refuted an assumption:

```text
qbittorrent: qbittorrent-nox --version exited 1: Bad command line:
  --confirm-legal-notice is an unknown command line parameter.
```

## ⛔ The assumption blocks earned their keep on their first outing

Each adapter carries a block listing what it assumes about a product and has not
measured. `--confirm-legal-notice` was in qBittorrent's, described as the thing
that stops a fresh profile blocking on a prompt.

⚠ **It was not a control at all. It was an argument the product refuses**, and it
was on **both** paths into that product: adding it to the version call to match
`start` looked like closing a one-gated door, and what it actually did was spread
a refused argument to a second place. ⭐ The acceptance is a profile key now,
which is the next assumption and is recorded as one.

## ⚠ What is still open, precisely

**The aria2 hang is isolated and not explained.** Its install step hung in run 1;
under `NEEDRESTART_MODE=a` the install succeeded and the step *after* it hung; and
under `l` it did the same. So the letter is not the cause.

⭐ **What four runs do establish is the boundary:** it is this one package
install, it survives the step that caused it, it is not the install command's own
duration, and two other adapters doing the same `apt-get` on the same image do
not hang.

⭐ **And it now leaves evidence.** The workflow uploads the install logs on
`always()`, because two dispatches lost their diagnosis to a step whose only
upload came after a capture that never happened. The next session reads
`update.log` and `install.log` out of `install-aria2-<run>-1` rather than
dispatching to find out.

## What a measured peer ID cost the secret scanner

⛔ **`check-no-secrets --public` turned CI run 67 red over an info hash**, which
is a measurement and is also forty hex digits, the shape of a credential. A
measured peer ID is twenty bytes and therefore the same shape, and a published
record will carry them constantly.

⭐ Both are allowed by phrase and backticks now, in both twins, and the allowance
is proved narrow: no phrase, no backticks, a longer run, and a second bare run on
the same line are each still refused.

⚠ **The process lesson is smaller and sharper.** A green subset of the gate is
not a green gate: `check-no-secrets` is two rows and only the second carries the
public rules, so re-running "the checks this edit touched" passed while the lane
went red.
