#!/bin/sh
# aria2-next.sh - the CLIENT-14 capture adapter.
#
# ⛔ A DIFFERENT PRODUCT FROM `aria2.sh`, NOT A VARIANT OF IT. `AnInsomniacy/aria2-next`
# keeps aria2's command-line vocabulary and does not keep its internals: this
# build links `libtorrent/2.1.1` and carries its own version string. Every value
# below was measured against the installed build on 2026-09-09 rather than
# copied from the adapter beside it, and three of those measurements contradict
# what copying would have produced.
#
# -- ⭐ WHAT WAS MEASURED, AND WHERE COPYING WOULD HAVE LIED ------------------
#
# Measured on 2026-09-09 against `aria2-next-2.7.4-linux-x86_64`, fetched from
# the project's own release and verified against the vendor's published
# `aria2-next-2.7.4-checksums.sha256` with `sha256sum -c` before it was run.
# Installing a product and asking its version is not a capture, so it needed no
# disposable host; nothing here was driven against a tracker.
#
#   ⛔ THE VERSION IS THE FOURTH FIELD, NOT THE THIRD. aria2 prints
#      `aria2 version 1.37.0` and this build prints `Aria2 Next version 2.7.4`.
#      `awk '{print $3}'`, which is what `aria2.sh` correctly does for ITS
#      product, returns the literal string `version` here. So the parse below is
#      anchored on the `version` token rather than on a column, because the
#      product name is prose whose word count is the vendor's to change.
#
#   ⛔ `--enable-dht6` DOES NOT EXIST AND IS ACCEPTED ANYWAY. The build logs
#      `Legacy aria2 input from command line: enable-dht6; accepted and skipped;
#      libtorrent uses one DHT switch for both families`. ⚠ A flag that is
#      accepted and skipped is worse than one that is refused: the step succeeds
#      and the switch it named was never set. `--enable-dht=false` covers both
#      families, and `start` VERIFIES that over RPC rather than trusting it.
#
#   ⭐ RPC ANSWERS `version` BETTER THAN THE COMMAND LINE DOES.
#      `aria2.getVersion` returns `{"product":"aria2-next","version":"2.7.4",...}`
#      - a structured field, with no prose to parse. ⛔ It is still NOT what
#      `version` below uses, and that is the contract rather than an oversight:
#      `install-client` calls `version` BEFORE the route runs, on a host where
#      nothing is installed and no daemon exists. An adapter whose `version`
#      started one would be changing the host at the one moment the caller is
#      trying to observe it unchanged. RPC cannot answer a question asked of a
#      host with nothing running, so this is exactly the case CLIENT-14's
#      "wherever RPC answers the same question" clause excludes.
#
#   ⛔ AND THE FACT THIS PROJECT EXISTS TO CATCH: the build's default
#      `bt-peer-id-prefix` is `-qB5230-`, read back over `aria2.getGlobalOption`.
#      A stock aria2-next presents itself on the wire as qBittorrent 5.2.3.0.
#      ⚠ NOTHING HERE OVERRIDES IT. The identity a stock build emits is the
#      measurement; an adapter that set a prefix would be publishing this
#      project's own string and calling it an observation. A capture is what
#      establishes what actually goes on the wire, and this note is a reason to
#      run one rather than a result standing in for one.
#
# ⛔ THERE IS NO PACKAGE ROUTE AND THAT IS MEASURED, NOT ASSUMED. `apt-cache
# search aria2` on `ubuntu-24.04` lists `aria2`, `libaria2-0`, `libaria2-0-dev`
# and `persepolis`, and no fork. ⚠ The dangerous failure is not the missing
# package: it is that `apt-get install aria2` SUCCEEDS and installs a DIFFERENT
# PRODUCT, after which `version` would answer 1.37.0 and the record would carry
# a second route that acquired somebody else's build. The package route below
# refuses by name for that reason.
#
# ⚠ SO THIS TARGET HAS ONE ROUTE TODAY, and `E-ACQ-01` refuses a record with
# one. CLIENT-14 carries that as the open question it is; this file does not
# paper over it by declaring a route it cannot take.
#
# See adapters/README.md for the contract.
#
# Exit codes: 0 done, 1 refused, 2 could not run.

set -u

ME=aria2-next

# ⚠ THE RPC PORT IS A DEFAULT THIS FILE STATES, not one inherited from the
# product. aria2's own default is 6800; a capture host runs one build at a time,
# and an override exists so a host already using that port is a configuration
# rather than a collision nobody can see.
RPC_PORT="${BIT_IDS_ARIA2_NEXT_RPC_PORT:-6800}"

refuse() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 1
}

cannot() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 2
}

# ⛔ THE INSTALLED EXECUTABLE, LOOKED UP ONCE. A path composed per subcommand is
# a second spelling, and the day a route installs somewhere else one subcommand
# finds the build and another reports it absent.
binary() {
  if [ -n "${BIT_IDS_ARIA2_NEXT:-}" ]; then
    printf '%s' "$BIT_IDS_ARIA2_NEXT"
    return 0
  fi
  if [ -n "${BIT_IDS_PREFIX:-}" ] && [ -x "$BIT_IDS_PREFIX/bin/aria2-next" ]; then
    printf '%s' "$BIT_IDS_PREFIX/bin/aria2-next"
    return 0
  fi
  command -v aria2-next 2>/dev/null
}

# ⭐ ONE JSON-RPC CALL, AND THE THREE STREAMS STAY APART. The response body goes
# to a file the caller names, curl's own diagnosis to another, and the exit code
# comes back from curl itself rather than from a pipeline.
#
# ⛔ `--noproxy` IS NOT OPTIONAL. This is a loopback call to a daemon on this
# host; a proxy in the environment would send a capture's own control channel off
# the machine, which on a contained host is the one thing that must not happen.
rpc() { # method params-json body-file
  _method="$1"
  _params="$2"
  _body="$3"
  curl -sS --noproxy '*' --max-time 20 \
    -X POST "http://127.0.0.1:$RPC_PORT/jsonrpc" \
    -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":\"bit-ids\",\"method\":\"$_method\",\"params\":$_params}" \
    -o "$_body" 2>>"${RPC_ERR:-/dev/null}"
}

# ⚠ A JSON-RPC ERROR ARRIVES WITH HTTP 200 AND AN `error` MEMBER. A caller
# reading only curl's status would take a refusal for an answer, which is the
# shape adapters/README.md warns about: an adapter failure and a product refusal
# reach this file through one channel and must not leave it as one status.
rpc_failed() { # body-file
  grep -q '"error"' "$1" 2>/dev/null
}

[ $# -ge 1 ] || cannot "a subcommand is required"
COMMAND="$1"
shift

# What each route of this adapter delivers, as `Build.package` spells it.
#
# ⭐ Both routes install a bare executable and neither installs a package: the
# release asset IS the binary, measured at 1.2 seconds for a download and a
# chmod, and the source route compiles one. ⚠ The `package` route refuses by
# name, so it reaches nothing here.
package_format() { # route
  case "$1" in
    release | source) printf 'elf-binary\n' ;;
    *) return 1 ;;
  esac
}

case "$COMMAND" in
  describe)
    printf 'target=aria2-next\n'
    printf 'kind=stock\n'
    _where=$(binary) || _where=""
    [ -z "$_where" ] || printf 'binary=%s\n' "$_where"
    # ⭐ THE RELEASE DECLARATIONS, MEASURED AGAINST THE LIVE LISTING on
    # 2026-09-09. Tags are `v2.7.4`, so the prefix is `v` and a version is three
    # components. A release publishes eight assets - seven builds over four
    # platforms, Android arm64 only and the other three in two architectures,
    # plus a checksums file - and exactly one of them is a Linux x86-64 build, so
    # the pattern needs no wildcard beyond `{version}`.
    #
    # ⚠ AND THE ASSET IS A BINARY, WHICH IS THE ONE PLACE THIS TARGET IS EASIER
    # THAN `aria2`. That project's release carries no Linux binary at all, so its
    # release route is a source build taking minutes; this one is a download and
    # a `chmod`, measured at 1.2 seconds.
    printf 'release_repo=AnInsomniacy/aria2-next\n'
    printf 'release_tag_prefix=v\n'
    printf 'release_min_components=3\n'
    printf 'release_max_components=3\n'
    printf 'release_asset=aria2-next-{version}-linux-x86_64\n'
    ;;

  install)
    [ $# -ge 2 ] || cannot "install takes <route> <workdir>"
    ROUTE="$1"
    WORKDIR="$2"
    mkdir -p "$WORKDIR" || cannot "cannot create $WORKDIR"
    _format=$(package_format "$ROUTE") ||
      cannot "this adapter does not say how the $ROUTE route packages a build"
    case "$ROUTE" in
      package)
        # ⛔ REFUSED BY NAME, AND THE REFUSAL IS THE MEASUREMENT. No package
        # index reachable from a capture host carries this fork; what they carry
        # is `aria2`, a different product at a different version. An
        # `apt-get install aria2` here would exit 0, install somebody else's
        # build, and hand `ACQ-03` a second route that agreed with nothing.
        cannot "aria2-next is published only through its own releases; \
no package index carries it, and installing aria2 would acquire a different product"
        ;;
      release)
        # ⭐ THE VENDOR'S OWN BUILD, INSTALLED RATHER THAN MERELY FETCHED. Every
        # release route in this tree once dropped its artifact in the workdir and
        # returned 0, so `version` answered from whatever was already on PATH.
        # This one puts the executable where `binary()` looks and then asks the
        # file whether it is there.
        [ -n "${BIT_IDS_RELEASE_URL:-}" ] ||
          cannot "the release route needs BIT_IDS_RELEASE_URL, resolved before the route was cut"
        command -v curl >/dev/null 2>&1 || cannot "curl is not on this host"
        PREFIX=${BIT_IDS_PREFIX:-/usr/local}
        mkdir -p "$PREFIX/bin" || cannot "cannot create $PREFIX/bin"
        curl -fsSL --retry 2 -o "$WORKDIR/aria2-next" "$BIT_IDS_RELEASE_URL" \
          </dev/null >"$WORKDIR/install.log" 2>&1 ||
          refuse "the release route could not be fetched"
        [ -s "$WORKDIR/aria2-next" ] ||
          refuse "the release route fetched an empty file"
        chmod +x "$WORKDIR/aria2-next" ||
          refuse "the release route could not make its artifact executable"
        # ⚠ INTO PLACE AS ONE STEP. A copy that is interrupted leaves a truncated
        # executable somewhere `binary()` will happily find; a rename within one
        # directory either happened or did not.
        cp "$WORKDIR/aria2-next" "$PREFIX/bin/.aria2-next.new" ||
          refuse "the release route could not stage its artifact in $PREFIX/bin"
        mv "$PREFIX/bin/.aria2-next.new" "$PREFIX/bin/aria2-next" ||
          refuse "the release route could not install into $PREFIX/bin"
        [ -x "$PREFIX/bin/aria2-next" ] ||
          refuse "the release route reported an install and left no aria2-next in $PREFIX/bin"
        ;;
      source)
        # ⭐ THE SECOND ROUTE, AND IT EXISTS BECAUSE THIS TARGET HAS NO PACKAGE.
        # `E-ACQ-01` refuses a record built from one route, and absolute 4 wants
        # two routes resolving the SAME stable version. Measured on 2026-09-09:
        # this build from tag `v2.7.5` answers `Aria2 Next version 2.7.5`, the
        # same version the release asset reports, from a 256-megabyte
        # `RelWithDebInfo` executable where the published one is 14 megabytes
        # stripped. ⭐ One version, two provably different builds, which is the
        # case `ACQ-03` exists to classify.
        #
        # ⛔ THESE TWO ROUTES DIFFER IN DELIVERY AND NOT IN RESOLVER, AND THIS
        # COMMENT SAID OTHERWISE UNTIL 2026-09-09. It claimed they "differ in
        # resolver - the releases API against git refs"; the artifacts refute it.
        # This route is HANDED `BIT_IDS_RELEASE_TAG` by *Resolve the release
        # artifact*, which is the same step and the same listing the release lane
        # reads - measured on capture-client run 14, whose two
        # `release/resolution.txt` differ only in their timestamps.
        # ⛔ `E-ACQ-07` calls two routes sharing a resolver ONE route, so the
        # pair cannot become a record until this route resolves its own tag.
        # `CI-09` carries the measurement and the argument; what a fix needs is a
        # resolution from git refs here, with the two versions COMPARED after
        # installation rather than made equal beforehand.
        # ⛔ They also do NOT differ in origin: whoever controls the repository
        # controls both. That is weaker than a distribution index against a
        # vendor release, and the record says so rather than letting two green
        # checks imply otherwise.
        #
        # ⚠ MEASURED COST: about 350 seconds for configure and build together on
        # a four-core host, against the caller's 540-second bound. It builds the
        # project's test binaries too, because `--preset default` is what the
        # project's own README and CI run and a narrower target would be this
        # repository inventing a build the vendor does not document.
        [ -n "${BIT_IDS_RELEASE_TAG:-}" ] ||
          cannot "the source route needs BIT_IDS_RELEASE_TAG, resolved before the route was cut"
        for _need in git cmake ninja c++; do
          command -v "$_need" >/dev/null 2>&1 ||
            cannot "the source route builds from source and $_need is not on this host"
        done
        PREFIX=${BIT_IDS_PREFIX:-/usr/local}
        mkdir -p "$PREFIX/bin" || cannot "cannot create $PREFIX/bin"
        # ⚠ THE REPOSITORY IS THE ONE `describe` ALREADY NAMES, so the two routes
        # cannot drift onto different upstreams. ⛔ A shallow clone of one tag:
        # the history is not the measurement and fetching it would be minutes of
        # network for bytes nothing reads.
        _repo=AnInsomniacy/aria2-next
        git clone --depth 1 --branch "$BIT_IDS_RELEASE_TAG" \
          "https://github.com/$_repo.git" "$WORKDIR/src" \
          </dev/null >"$WORKDIR/install.log" 2>&1 ||
          refuse "the source route could not clone $BIT_IDS_RELEASE_TAG"
        # ⛔ THE COMMIT IS WHAT `E-ACQ-06` ASKS FOR AND A TAG IS NOT IT. A tag is
        # a name somebody can move; `SourceIdentity::SourceCommit` takes a full
        # object name and refuses an abbreviation. Measured on 2026-09-09 by
        # assembling run 14: nothing the capture path wrote carried one, so a
        # source route could not become a record at all, and the object name
        # existed only inside the git chatter in `install.log`. ⚠ It is
        # `rev-parse HEAD` rather than the tag's own object, because
        # `v2.7.5` is an annotated tag - the clone said `is not a commit!` and
        # checked out what it points at, which is what was built.
        git -C "$WORKDIR/src" rev-parse HEAD >"$WORKDIR/source-commit" 2>>"$WORKDIR/install.log" ||
          refuse "the source route cloned $BIT_IDS_RELEASE_TAG and could not name its commit"
        (cd "$WORKDIR/src" && cmake --preset default) \
          </dev/null >>"$WORKDIR/install.log" 2>&1 ||
          refuse "the source route could not configure a build"
        (cd "$WORKDIR/src" && cmake --build --preset default) \
          </dev/null >>"$WORKDIR/install.log" 2>&1 ||
          refuse "the source route could not build aria2-next"
        # ⚠ THE BUILT PATH IS THE PRESET'S, found rather than composed twice: the
        # `default` preset writes into `build/default` and that is where the
        # project's own CI reads its binary from.
        [ -x "$WORKDIR/src/build/default/aria2-next" ] ||
          refuse "the source route built nothing at build/default/aria2-next"
        cp "$WORKDIR/src/build/default/aria2-next" "$PREFIX/bin/.aria2-next.new" ||
          refuse "the source route could not stage its build in $PREFIX/bin"
        mv "$PREFIX/bin/.aria2-next.new" "$PREFIX/bin/aria2-next" ||
          refuse "the source route could not install into $PREFIX/bin"
        [ -x "$PREFIX/bin/aria2-next" ] ||
          refuse "the source route reported an install and left no aria2-next in $PREFIX/bin"
        ;;
      *) cannot "unknown route: $ROUTE" ;;
    esac
    # ⛔ WHAT THE ROUTE DELIVERED, WRITTEN WHERE `install-client` READS IT.
    # `Build.package` is part of the identity tuple a store path is derived
    # from, so a record filed without it - or with one this project guessed -
    # files two packagings of one version at one name. Measured on 2026-09-09 by
    # assembling capture-client run 14: nothing any adapter wrote said how the
    # build arrived, and `assemble-capture` refused both lanes for it.
    # ⚠ THE KEY IS `package` BECAUSE THAT IS THE RECORD'S OWN VOCABULARY, and
    # the ROUTE also called `package` is the host's package-manager route. They
    # are different things and this is the one file where both appear.
    printf '%s\n' "$_format" >"$WORKDIR/package" ||
      refuse "the route installed and could not record how it packaged the build"
    ;;

  version)
    BINARY=$(binary) || cannot "aria2-next is not installed"
    [ -n "$BINARY" ] || cannot "aria2-next is not installed"
    # ⛔ THE EXIT CODE IS READ FROM THE PROCESS THAT PRODUCED IT, UNPIPED.
    OUTPUT=$("$BINARY" --version 2>/dev/null </dev/null)
    VERSION_RC=$?
    [ "$VERSION_RC" = 0 ] || cannot "aria2-next --version exited $VERSION_RC"
    LINE=$(printf '%s\n' "$OUTPUT" | head -1)
    # ⛔ ANCHORED ON THE `version` TOKEN, NOT ON A COLUMN. Measured: this build's
    # first line is `Aria2 Next version 2.7.4`, so the third field is the word
    # `version` and the fourth is the number. A column index here would have
    # published the string `version` as this product's version, and it would have
    # validated: it is a non-empty field.
    VERSION=$(printf '%s' "$LINE" |
      awk '{ for (i = 1; i < NF; i++) if ($i == "version") { print $(i + 1); exit } }')
    [ -n "$VERSION" ] || cannot "aria2-next reported no parseable version: [$LINE]"
    # ⭐ WHAT THE BUILD SAID ABOUT ITSELF IS KEPT, not only the field parsed out
    # of it. This build's own answer names `libtorrent/2.1.1` among its
    # libraries, which is a fact about what it will put on the wire that no
    # version number carries.
    printf '%s reported:\n' "$ME" >&2
    printf '%s\n' "$OUTPUT" >&2
    printf '%s\n' "$VERSION"
    ;;

  start)
    [ $# -ge 3 ] || cannot "start takes <torrent> <workdir> <peer-port>"
    TORRENT="$1"
    WORKDIR="$2"
    PORT="$3"
    BINARY=$(binary) || cannot "aria2-next is not installed"
    [ -n "$BINARY" ] || cannot "aria2-next is not installed"
    [ -s "$TORRENT" ] || cannot "there is no torrent at $TORRENT"
    mkdir -p "$WORKDIR/downloads" || cannot "cannot create $WORKDIR/downloads"
    command -v curl >/dev/null 2>&1 || cannot "curl is not on this host"
    command -v base64 >/dev/null 2>&1 || cannot "base64 is not on this host"
    RPC_ERR="$WORKDIR/rpc.err"

    # ⛔ A PER-RUN TOKEN, GENERATED HERE AND WRITTEN TO RUN STATE. A literal in
    # this file would be a shared secret in the tree, which rule 12 refuses; a
    # daemon with no token would accept an RPC call from anything that reached
    # the port.
    #
    # ⛔ AND `stop` DELETES IT, BECAUSE THE WORKDIR IS THE EVIDENCE BUNDLE.
    # Measured on capture-client run 11: this file shipped inside the uploaded
    # artifact as `client/rpc-token`, which is rule 12's "a secret never enters
    # ... artifacts" in the one place no reading had looked - the run wrote it,
    # the run bundled the directory it was in, and nothing between them was
    # wrong. ⚠ A short-lived token for a loopback port on a host that is about to
    # be destroyed is a weak secret, and rule 12 does not grade them.
    #
    # ⚠ 077 SO IT IS NEVER WORLD-READABLE EVEN WHILE IT EXISTS, and the umask is
    # restored straight after: a subshell would not do, because the file has to
    # outlive it.
    TOKEN=$(od -An -tx1 -N16 /dev/urandom 2>/dev/null | tr -d ' \n')
    [ -n "$TOKEN" ] || cannot "cannot generate an RPC token"
    _oldmask=$(umask)
    umask 077
    printf '%s\n' "$TOKEN" >"$WORKDIR/rpc-token" ||
      cannot "cannot record the RPC token"
    umask "$_oldmask"
    printf '%s\n' "$RPC_PORT" >"$WORKDIR/rpc-port" ||
      cannot "cannot record the RPC port"

    # ⛔ FOUR DISCOVERY SURFACES OFF, NOT THE THREE THE CONTRACT NAMES.
    # adapters/README.md says an adapter disables DHT, peer exchange and local
    # peer discovery; this product has a FOURTH. `bt-port-mapping` is
    # "Enable UPnP and NAT-PMP port mapping" and it defaults to TRUE, which was
    # not read off the help text but found by driving the build on 2026-09-09 and
    # reading its own sockets out of `/proc`: with the three documented switches
    # off, the process was still bound to **UDP 1900**, the SSDP multicast port.
    # ⚠ A contained host has no default route, so a tracker or a DHT bootstrap
    # fails - and a multicast to 239.255.255.250 is link-local and does not. This
    # is the one-gated-door shape the README calls the most recurring hole there
    # is, found in the file that quotes it.
    #
    # ⛔ AND `--enable-dht6` IS DELIBERATELY ABSENT. Measured: this build does not
    # have that option, accepts it from the command line anyway and skips it with
    # a warning, because libtorrent uses one DHT switch for both families.
    # Passing it would put a flag in the record that set nothing.
    # `--enable-dht=false` is what covers both, and the verification below is
    # what establishes that it did rather than assuming it.
    #
    # ⛔ THE PEER PORT IS PASSED HERE, ON THE COMMAND LINE, and zero still means
    # "leave it": aria2 refuses a listen port of zero, so an unset peer port
    # drops the flag rather than sending a value the build would reject.
    #
    # ⛔ RPC IS BOUND TO LOOPBACK. `--rpc-listen-all=false` is the difference
    # between a control channel and a service on the capture host's network.
    set -- \
      --enable-rpc=true \
      --rpc-listen-port="$RPC_PORT" \
      --rpc-listen-all=false \
      --rpc-secret="$TOKEN" \
      --enable-dht=false \
      --bt-enable-lpd=false \
      --enable-peer-exchange=false \
      --bt-port-mapping=false \
      --bt-require-crypto=false \
      --dir="$WORKDIR/downloads" \
      --seed-time=0 \
      --allow-overwrite=true \
      --console-log-level=info \
      --summary-interval=0
    [ "$PORT" = "0" ] || set -- "$@" --listen-port="$PORT"

    "$BINARY" "$@" >"$WORKDIR/client.out" 2>&1 &
    printf '%s\n' "$!" >"$WORKDIR/pid"
    PID=$(cat "$WORKDIR/pid")

    # ⚠ A BOUNDED WAIT ON THE WORK, NOT ON THE CLOCK. The daemon is up when it
    # answers, so this asks it; a fixed sleep would be too short on a loaded
    # runner and wasted everywhere else. conventions/shell.md section 10.
    WAITED=0
    until rpc aria2.getVersion "[\"token:$TOKEN\"]" "$WORKDIR/rpc-version.json" &&
      ! rpc_failed "$WORKDIR/rpc-version.json"; do
      # ⛔ A DEAD DAEMON IS REPORTED AS ONE, rather than waited out. A build that
      # refused its own options exits immediately, and without this the step
      # would spend the whole deadline discovering it.
      kill -0 "$PID" 2>/dev/null ||
        refuse "aria2-next exited before its RPC server answered; $WORKDIR/client.out says why"
      WAITED=$((WAITED + 1))
      [ "$WAITED" -lt 30 ] ||
        refuse "aria2-next did not answer RPC on port $RPC_PORT within 30 seconds"
      sleep 1
    done

    # ⭐ THE CONTAINMENT SWITCHES ARE READ BACK FROM THE BUILD, WHICH IS THE ONE
    # QUESTION RPC ANSWERS AND THE COMMAND LINE CANNOT. A command line reports
    # what was PASSED; `aria2.getGlobalOption` reports what took EFFECT. This
    # target accepts an option it does not have and skips it, so the difference
    # between those two is not theoretical here - it is the measured behaviour of
    # `--enable-dht6`.
    rpc aria2.getGlobalOption "[\"token:$TOKEN\"]" "$WORKDIR/rpc-options.json" ||
      refuse "aria2-next would not report its global options"
    ! rpc_failed "$WORKDIR/rpc-options.json" ||
      refuse "aria2-next refused to report its global options"
    for _switch in enable-dht enable-dht6 bt-enable-lpd enable-peer-exchange bt-port-mapping; do
      grep -q "\"$_switch\":\"false\"" "$WORKDIR/rpc-options.json" ||
        refuse "aria2-next reports $_switch is not false; a discovery surface is live"
    done

    # ⛔ AND THE PORT THE OBSERVER IS WATCHING IS READ BACK FROM THE BUILD. A
    # peer port that silently did not take is a capture watching a port nothing
    # is on, and it would look exactly like a client that never announced.
    if [ "$PORT" != "0" ]; then
      grep -q "\"listen-port\":\"$PORT\"" "$WORKDIR/rpc-options.json" ||
        refuse "aria2-next reports a listen port other than $PORT"
    fi

    # ⭐ THE TORRENT GOES IN OVER RPC, base64-ENCODED, WHICH IS THE CASE
    # CLIENT-14 IS ABOUT. `aria2.addTorrent` takes the torrent's bytes rather
    # than a path, so the build is handed the same object the observer wrote
    # instead of a filename it re-reads. ⚠ base64 is the one encoding no shell
    # interprets - conventions/shell.md section 1 - and it is what this method
    # already requires, so nothing is being encoded twice.
    B64=$(base64 -w0 <"$TORRENT" 2>/dev/null) || B64=$(base64 <"$TORRENT" | tr -d '\n')
    [ -n "$B64" ] || cannot "the torrent at $TORRENT could not be encoded"
    rpc aria2.addTorrent "[\"token:$TOKEN\",\"$B64\"]" "$WORKDIR/rpc-add.json" ||
      refuse "aria2-next would not accept the torrent over RPC"
    ! rpc_failed "$WORKDIR/rpc-add.json" ||
      refuse "aria2-next refused the torrent: $(cat "$WORKDIR/rpc-add.json")"
    GID=$(sed -n 's/.*"result":"\([^"]*\)".*/\1/p' "$WORKDIR/rpc-add.json")
    [ -n "$GID" ] || refuse "aria2-next accepted the torrent and named no download"
    printf '%s\n' "$GID" >"$WORKDIR/gid"
    printf 'started aria2-next pid %s on port %s, download %s, RPC %s\n' \
      "$PID" "$PORT" "$GID" "$RPC_PORT"
    ;;

  stop)
    [ $# -ge 1 ] || cannot "stop takes <workdir>"
    WORKDIR="$1"
    # ⚠ A stop over a build that already exited is done, not refused.
    [ -f "$WORKDIR/pid" ] || exit 0
    PID=$(cat "$WORKDIR/pid")
    case "$PID" in
      '' | *[!0-9]*) exit 0 ;;
    esac
    RPC_ERR="$WORKDIR/rpc.err"

    # ⭐ THE PRODUCT'S OWN SHUTDOWN FIRST. `aria2.shutdown` is the same question a
    # signal asks and a better-behaved answer to it: the build closes its
    # sockets, writes its session state and exits on its own terms. ⚠ The signal
    # path below is kept, because a build that ignores its own RPC is exactly the
    # case a capture must still be able to end.
    if [ -f "$WORKDIR/rpc-token" ]; then
      TOKEN=$(cat "$WORKDIR/rpc-token")
      [ ! -f "$WORKDIR/rpc-port" ] || RPC_PORT=$(cat "$WORKDIR/rpc-port")
      rpc aria2.shutdown "[\"token:$TOKEN\"]" "$WORKDIR/rpc-shutdown.json" || :
      # ⛔ GONE BEFORE ANYTHING BUNDLES THIS DIRECTORY, and gone whether or not
      # the shutdown was accepted. `capture-client` calls `stop` and then packs
      # the workdir, so this unlink is the whole of what keeps the token out of
      # the artifact. ⚠ It is removed HERE rather than at the end of `stop`,
      # because every path below this line can exit.
      rm -f "$WORKDIR/rpc-token"
      # ⚠ TEN SECONDS, AND THE NUMBER IS MEASURED RATHER THAN ROUND. Driven on
      # 2026-09-09: `aria2.shutdown` answered `OK` and the build was still alive
      # five seconds later, so a five-second grace fell through to a signal every
      # time and the graceful path this branch exists for never once completed.
      WAITED=0
      while [ "$WAITED" -lt 10 ]; do
        kill -0 "$PID" 2>/dev/null || exit 0
        sleep 1
        WAITED=$((WAITED + 1))
      done
    fi

    kill "$PID" 2>/dev/null || exit 0
    WAITED=0
    while [ "$WAITED" -lt 5 ]; do
      kill -0 "$PID" 2>/dev/null || exit 0
      sleep 1
      WAITED=$((WAITED + 1))
    done
    kill -9 "$PID" 2>/dev/null
    exit 0
    ;;

  *) cannot "unknown subcommand: $COMMAND" ;;
esac
