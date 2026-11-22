# Making a release

Everything is automatic once a version tag is pushed. Plan and background:
`docs/release-and-updates-plan.md`.

## Steps

1. Make sure `master` is green on GitHub (the **CI** workflow: Actions tab).
2. Bump `version` in `[workspace.package]` of the top-level `Cargo.toml`
   (e.g. `0.1.0` → `0.2.0`). It is the only place the version lives.
3. Add a `## v0.2.0` section at the top of `docs/RELEASE_NOTES.md`, in plain
   language: what is new, what is fixed. The newest section becomes the body of
   the GitHub release and what the app shows under "update available".
4. Commit, then tag and push:
   ```sh
   git tag v0.2.0
   git push origin master --tags
   ```
5. Watch the **Release** workflow (Actions tab). It first checks that the tag
   matches `Cargo.toml` and that the notes section exists, then builds Linux,
   Windows and macOS in parallel, then publishes the release with the three
   archives and a `SHA256SUMS` file. If any platform fails, nothing is published;
   fix it, delete the tag (`git tag -d v0.2.0 && git push origin :v0.2.0`) and
   tag again.

A release can also be re-run for an existing tag from the Actions tab
(**Release › Run workflow**).

## What comes out

| File | Contents |
|---|---|
| `portmanteau-linux-x86_64.tar.gz` | app, CLI, MCP server, `.desktop` file, icon, licence |
| `portmanteau-windows-x86_64.zip` | `portmanteau.exe`, CLI, MCP server, licence |
| `portmanteau-macos-universal.zip` | `Portmanteau.app` (Intel and Apple Silicon, ad hoc signed), CLI, MCP server |
| `SHA256SUMS` | checksums of the three archives |
| `SHA256SUMS.minisig` | the signature on `SHA256SUMS` that the app checks before installing |

## First launch warnings (no paid certificates)

- **macOS**: "cannot be opened because the developer cannot be verified" →
  right-click the app › Open › Open (once), or System Settings › Privacy &
  Security › Open Anyway.
- **Windows**: SmartScreen "Windows protected your PC" → More info › Run anyway.

An Apple Developer account or a Windows signing certificate would remove both;
the workflow can take them later without other changes.

## How updates are trusted (signing)

The app installs an update only if **both** hold: `SHA256SUMS.minisig` is a valid
[minisign](https://jedisct1.github.io/minisign/) signature of `SHA256SUMS` by the
owner's key, and the downloaded archive's SHA-256 is the one listed there. The
public key is built into the app (`crates/portmanteau-update/update-public-key.pub`,
key id `5546518CFF710996`); the private key is the repository secret
`UPDATE_SIGNING_KEY` (set 2026-11-22) and the owner's password-manager backup.
The release workflow refuses to publish if the secret is missing, and checks its
own signature against the public key in the repository.

- **If the private key is lost or leaks**: make a new pair
  (`minisign -G -W -p new.pub -s new.key`), replace the secret and
  `update-public-key.pub`, and release. Installed apps carry the old public key,
  so they will refuse the new updates: everyone must download that release by
  hand once. (A leaked key lets an attacker sign malicious updates, so rotate at once.)
- **What the app never does**: install from any address other than
  `github.com/davidroberthoare/portmanteau/releases/download/…`, install an
  unsigned release (the panel falls back to "Download"), or touch the
  installed program when any check fails.

## Trying an update end to end (once, on each system)

An update can only be tried for real between two published releases:

1. Release `v0.1.0` and install it (unpack the archive somewhere you can write to).
2. Bump to `0.1.1`, add notes, tag `v0.1.1`, and wait for the Release workflow.
3. Start the installed 0.1.0: the update icon should appear (or Help › Check for
   Updates…). Open it, choose **Install and restart**. The app should close and
   reopen as 0.1.1 (Help › About) with the same project open.
4. Check that `*.old` files were cleaned up at that start, and that
   `portmanteau-cli --help` beside it still runs.
5. Try the failure paths: install a copy in a read-only folder (the panel should
   offer the release page), and cancel a download part-way.

On macOS the first launch after a *manual* download triggers Gatekeeper; an
update installed by the app does not.

## Keeping the workflows safe

The release workflow holds the signing key, so it is locked down: every
third-party action is pinned to a full commit SHA (the trailing comment names the
version), only the publishing job may write to the repository, and release builds
use no cache. Pins don't update themselves; now and then refresh them with
`gh api repos/<owner>/<repo>/commits/<tag> --jq .sha` (for example
`actions/checkout` and `v4`), after checking the action's release notes.
