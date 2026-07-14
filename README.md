# ops

CLI for the Opsd API.

## Authentication

Sign in before calling the public API:

```sh
ops auth login
```

The CLI starts an OAuth device authorization, attempts to open the Opsd
website, and also prints the verification URL and user code for terminals that
cannot open a browser. The command waits until the website approval completes.

Inspect or remove the saved login with:

```sh
ops auth status
ops auth logout
```

Logout revokes the OAuth access token on the server before removing the local
credential. If revocation fails, the credential is retained so logout can be
retried.

The opaque access token is stored in `~/.config/opsd/credentials.json`. The
directory and file are restricted to the current user with Unix permissions
`0700` and `0600`. Set `OPSD_CONFIG_DIR` to use a different directory.

Credentials are tied to the server that issued them. For local development,
pass the same server URL when logging in and making later requests:

```sh
ops --base-url http://localhost:8080 auth login
ops --base-url http://localhost:8080 hello world
```

## Installation

On macOS or Linux, install the latest release with:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://downloads.opsd.sh/ops/install.sh | sh
```

The installer places `ops` in `~/.local/bin` and explains how to add that
directory to `PATH` if needed.

### Shell completions

Add the command for your shell to its startup file:

```sh
# Bash: ~/.bashrc
eval "$(ops completions bash)"

# Zsh: ~/.zshrc
source <(ops completions zsh)

# Fish: ~/.config/fish/config.fish
ops completions fish | source
```

Open a new terminal after updating the startup file.

## Releasing

Releases are built from the private `ops` and `opsd-rust` repositories and
published as public binaries at `downloads.opsd.sh`.

### Release inputs

- `Cargo.toml` contains the CLI version.
- `dist-workspace.toml` defines release targets, archives, and the installer.
- `opsd-rust.rev` pins the exact `opsd-rust` revision.
- The `OPSD_RUST_TOKEN` Actions secret grants read-only access to the private
  `opsd-rust` repository.

Update the pinned SDK revision whenever a release needs newer SDK code. Release
builds intentionally do not follow the SDK's default branch.

### Validate without publishing

Run the `Release` workflow manually from GitHub Actions. Manual runs test and
package all configured targets. This exercises the private `opsd-rust`
checkout, each native GitHub runner, the Linux musl toolchains, and `dist`
installer generation before creating an immutable release tag.

The resulting archives, checksums, and installer are saved as a temporary
GitHub Actions artifact named `release`, where they can be inspected or
downloaded from the workflow run. They are not uploaded to
`downloads.opsd.sh`.

AWS publication steps run only when the workflow was triggered by a `v*` tag.
The AWS role independently enforces the same restriction through its GitHub
OIDC trust policy, so a manual workflow run cannot publish even though it uses
the same packaging jobs.

Locally, inspect the release plan with:

```sh
dist plan
```

### Publish a release

1. Update the version in `Cargo.toml`, commit the release changes, and push
   them to `main`.
2. Optionally open the repository's GitHub Actions page and run the `Release`
   workflow manually as a preflight check. Confirm that every platform build
   and the `Package and publish` job succeed. The resulting `release` artifact
   can be inspected, but is not published to AWS.
3. Create and push a matching version tag:

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

The tag-triggered workflow publishes immutable artifacts below:

```text
https://downloads.opsd.sh/ops/releases/v0.1.0/
```

After all versioned artifacts are uploaded, it updates:

```text
https://downloads.opsd.sh/ops/install.sh
https://downloads.opsd.sh/ops/latest.json
```

The AWS release role trusts only `v*` tag workflows, so manual workflow runs
cannot publish artifacts.
