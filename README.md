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

Rust developers can instead build and install the CLI from crates.io using
Rust 1.99 or newer:

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

## Practices and payroll

Practice commands follow the existing business command structure:

```sh
opsctl practices create --name "Example Accountants"
opsctl practices list
opsctl practices get PRACTICE_ID
opsctl practices businesses create PRACTICE_ID --name "Client Ltd"
opsctl practices businesses list PRACTICE_ID
opsctl practices members list PRACTICE_ID
opsctl practices members update PRACTICE_ID USER_ID --role admin
opsctl practices members remove PRACTICE_ID USER_ID
opsctl practices invitations create PRACTICE_ID --email user@example.com --role member
opsctl practices invitations list PRACTICE_ID
opsctl practices invitations cancel PRACTICE_ID INVITATION_ID
opsctl practice-invitations list
opsctl practice-invitations accept INVITATION_ID
opsctl practice-invitations decline INVITATION_ID
```

Replace the uppercase placeholders with public UUIDs returned by the API.
Practice roles are `admin` and `member`. Creating a practice-owned business
creates a new client with no direct users; it does not link an existing business.
Use the existing employee, PAYE and payroll-run commands with that client's
business ID for operational work.

Standalone payroll agreements use subscription vocabulary:

```sh
opsctl businesses payroll subscribe BUSINESS_ID
opsctl businesses payroll status BUSINESS_ID
opsctl businesses payroll cancel BUSINESS_ID
```

Practice admins enable or disable payroll for individual clients:

```sh
opsctl practices payroll enable PRACTICE_ID BUSINESS_ID
opsctl practices payroll status PRACTICE_ID BUSINESS_ID
opsctl practices payroll disable PRACTICE_ID BUSINESS_ID
```

Subscribing or enabling does not itself start billing or clear suspension.
Cancellation and disabling do not clear outstanding charges. Status includes
any independent suspension; suspension applies practice-wide for practice clients.

## Billing

Both payer types support these administrator commands:

```sh
opsctl businesses billing status BUSINESS_ID
opsctl businesses billing payment-methods BUSINESS_ID --limit 25
opsctl businesses billing set-default-payment-method BUSINESS_ID pm_EXAMPLE
opsctl businesses billing invoices BUSINESS_ID --before 2026-10 --limit 25
opsctl businesses billing invoice-lines BUSINESS_ID INVOICE_ID --offset 0 --limit 25
opsctl businesses billing invoice-pdf BUSINESS_ID INVOICE_ID --output invoice.pdf

opsctl practices billing status PRACTICE_ID
opsctl practices billing payment-methods PRACTICE_ID
opsctl practices billing set-default-payment-method PRACTICE_ID pm_EXAMPLE
opsctl practices billing invoices PRACTICE_ID
opsctl practices billing invoice-lines PRACTICE_ID INVOICE_ID
opsctl practices billing invoice-pdf PRACTICE_ID INVOICE_ID --output practice-invoice.pdf
```

Successful API responses are printed as JSON; operations returning no content
are silent. List commands return one page. Pass `next_before` as `--before`,
`next_offset` as `--offset`, or `next_starting_after` as `--starting-after` on
the next request. A null cursor means there are no more pages. Limits accept
1–100; omitting pagination options uses the server defaults.

Billing status and payment methods are read live from Stripe. Invoice history
and detailed charges come from Opsd; subtotals are not outstanding balances.
Practice invoice lines identify each client business, while PDFs contain
summary lines. Selecting a default payment method does not charge it, retry
invoices or clear suspension.

PDF downloads require `--output`, preserve binary contents, and refuse to
overwrite existing files. New PDF files are owner-only on Unix. Download
failures do not create an output file. Files are written to a temporary file
in the destination directory first, so a local write failure leaves no partial
PDF at the requested path. Confirmation is printed to stderr,
leaving stdout empty.

The existing `businesses billing setup BUSINESS_ID` command opens the website
without passing CLI credentials to the browser. Practice payment setup is
handled through the website; the new practice billing commands use API
credentials and do not create browser portal sessions.

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
   git tag -a v0.3.2 -m "Release opsctl 0.3.2"
   git push origin refs/tags/v0.3.2
   ```

The tag-triggered workflow publishes immutable artifacts below:

```text
https://downloads.opsd.sh/opsctl/releases/v0.3.2/
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
