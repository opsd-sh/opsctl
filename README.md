# opsctl

CLI for the Opsd API.

## Authentication

Sign in before calling the public API:

```sh
opsctl auth login
```

The CLI starts an OAuth device authorization, attempts to open the Opsd
website, and also prints the verification URL and user code for terminals that
cannot open a browser. The command waits until the website approval completes.

Inspect or remove the saved login with:

```sh
opsctl auth status
opsctl auth logout
```

Logout revokes the OAuth access token on the server before removing the local
credential. If revocation fails, the credential is retained so logout can be
retried.

The opaque access token is stored in the platform's configuration directory
under `opsctl/credentials.json`. On Linux this defaults to
`~/.config/opsctl/credentials.json`; on macOS it defaults to
`~/Library/Application Support/opsctl/credentials.json`. The directory and
file are restricted to the current user with Unix permissions `0700` and
`0600`. Set `OPSCTL_CONFIG_DIR` to use a different directory.

Credentials are tied to the server that issued them. For local development,
pass the same server URL when logging in and making later requests:

```sh
opsctl --base-url http://localhost:8080 auth login
opsctl --base-url http://localhost:8080 hello world
```

## Installation

On macOS or Linux, install the latest release with:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://downloads.opsd.sh/opsctl/install.sh | sh
```

The installer places `opsctl` in `~/.local/bin` and explains how to add that
directory to `PATH` if needed.

Rust developers can instead build and install the CLI from crates.io:

```sh
cargo install --locked opsctl
```

### Shell completions

Add the command for your shell to its startup file:

```sh
# Bash: ~/.bashrc
eval "$(opsctl completions bash)"

# Zsh: ~/.zshrc
source <(opsctl completions zsh)

# Fish: ~/.config/fish/config.fish
opsctl completions fish | source
```

Open a new terminal after updating the startup file.

## Releasing

Releases are built from the `opsctl` repository and published as public
binaries at `downloads.opsd.sh`.

### Release inputs

- `Cargo.toml` contains the CLI version.
- `Cargo.lock` pins the exact versions of dependencies, including `opsd`.
- `dist-workspace.toml` defines release targets, archives, and the installer.

### Validate without publishing

Run the `Release` workflow manually from GitHub Actions. Manual runs test and
package all configured targets. This exercises each native GitHub runner, the
Linux musl toolchains, and `dist` installer generation before creating an
immutable release tag.

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
   git tag v0.3.0
   git push origin v0.3.0
   ```

The tag-triggered workflow publishes immutable artifacts below:

```text
https://downloads.opsd.sh/opsctl/releases/v0.3.0/
```

After all versioned artifacts are uploaded, it updates:

```text
https://downloads.opsd.sh/opsctl/install.sh
https://downloads.opsd.sh/opsctl/latest.json
```

The AWS release role trusts only `v*` tag workflows, so manual workflow runs
cannot publish artifacts.

## License

Licensed under either the Apache License, Version 2.0 or the MIT license, at
your option.
