# The capture host

What a machine must be before this project installs a client on it, and what
refuses one that is not.

A capture runs a binary this project downloaded minutes earlier, from a vendor
it does not control, and points it at a network. That is the whole product and
it is also the one thing that can go wrong outside a record. Everything here
exists so the blast radius is a machine nobody keeps.

## The boundary is before the install, not in the record

⛔ **`E-MAN-30` refuses to record a capture whose host was not disposable. It
cannot stop one.** By the time a manifest exists, an untrusted installer has
already run somewhere. A record-layer refusal is a report.

So the boundary is
[`../scripts/acquisition/assert-disposable.sh`](../scripts/acquisition/assert-disposable.sh),
which runs before anything is installed, and which
[`../scripts/acquisition/check-runner.sh`](../scripts/acquisition/check-runner.sh)
mutation-proves on every gate.

## Two guards, and why they are not one

| guard | refuses | independent because |
| --- | --- | --- |
| `--claim` | a host that already ran a capture | it detects a survived host whatever the configuration claims |
| `--egress` | a host that can reach anything but loopback | it reads the routing table, which no claim can talk it out of |

A fresh host with an open route leaks the capture onto the public network. A
firewalled host that already ran a capture contaminates this one with the last
one's state. Neither guard implies the other, and
[`AGENTS.md`](AGENTS.md) section 5 requires both before any client is installed.

### The claim detects the failure rather than trusting the claim

⛔ **The obvious design is a token the provisioner writes saying "this host is
disposable", and it is the wrong one.** A promise is exactly what fails
silently: a runner misconfigured to persist its disk still carries the token,
still says disposable, and nothing notices until two captures share state.

So `--claim` writes a marker and refuses if one is already there. A second
capture on one host means the host survived the first, which means it was never
disposable whatever anything claimed. The evidence is the marker's existence,
and a misconfiguration cannot produce its absence.

⚠ **The marker lives in `/var/lib`, and that is load-bearing.** `/run` and
`/tmp` are cleared by a reboot, so a host that rebooted rather than being
destroyed would read as fresh.

The manifest records the guard's answer in `isolation.claim`: the fingerprint it
read and when it claimed the host. `E-MAN-33` refuses a claim stamped after the
run started, because a guard that ran after the install is a report again.

### The egress guard asks the kernel, not the network

⛔ **Nothing probes a third party.** Reaching out from a machine this guard
exists to establish is contained would be exactly the wrong order. It reads
`/proc/net/route` and looks for a default route that is up.

⚠ **POSIX awk only.** The first version used `and(strtonum(...))`, which are
gawk extensions. On a POSIX awk they are undefined functions, awk exits
non-zero, and the guard reported "could not establish" over a machine that
plainly had a default route. It failed closed, which is the right direction, but
a guard that cannot run on a minimal image does not run where it matters most.

## Reading the guards' output

Both guards print the input they trusted: `--egress` names the routing table it
read and `--claim` names the marker it wrote. The routing table is an optional
argument and the marker directory an environment variable, so the runner test
can drive them against fixtures.

⚠ **A seam a test can use is a seam a misconfiguration can use.** Printing the
input is what makes a run that passed over a fixture visible in a log rather
than indistinguishable from one that passed over the machine.

## Linux runner contract

1. The host is created for one capture and destroyed after it. Not reset, not
   reimaged in place: destroyed.
2. Before any download or install: `assert-disposable.sh --claim <run-id>` and
   `assert-disposable.sh --egress`, both exit 0. Exit 2 is not a pass.
3. No route off the host except the loopback endpoints the run starts itself.
   A run that needs more records why, and `E-MAN-31` refuses one that does not.
4. No credential of any kind is present. The run needs none: everything it
   fetches is public, and
   [`security/secrets.md`](security/secrets.md) is the rule.
5. The account running the client owns nothing that outlives the host.
6. After the run, the host is destroyed and the next job's
   `assert-disposable.sh --fingerprint` differs.

## Windows runner contract

The same six rules. ⛔ **The executable guards are Linux-only today**, because
they read `/proc/net/route` and `/etc/machine-id`. A Windows capture host needs
its own pair reading `Get-NetRoute` and the machine GUID, and until it exists a
Windows capture is not permitted: there is no boundary to run before the
install, and the contract above is not satisfied by intending to satisfy it.

`CI-03` owns the trusted capture runner matrix and is where the Windows pair
lands. `TODO/acquisition.md` carries it as a named residual rather than as an
assumption.

## What this does not establish

⚠ **The guards prove they fire. They do not prove they are sufficient.** A host
can be non-disposable in ways neither models: a mounted network share, a
hypervisor snapshot restored between runs, a container sharing a writable layer
with its siblings. Those are not detected here, and
[`security/remote-ops.md`](security/remote-ops.md) carries what the operator
owns.
