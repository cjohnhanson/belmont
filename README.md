# belmont

Belmont is a secrets manager for LLM coding agents. It resolves
credentials from pluggable backends, injects them into commands at
runtime, and scrubs the resolved values out of command output before an
agent sees them — so an agent can use a secret without the secret
landing in its context window.

## Install

```sh
cargo install --git https://github.com/cjohnhanson/belmont
```

## Usage

```sh
belmont init              # create belmont.yml
belmont list              # show declared secret references (never values)
belmont check             # verify all secrets are resolvable
belmont set <name> [val]  # store a secret in its backend
belmont run <command>     # execute with secrets injected, output scrubbed
```

A typical run (use single quotes so your shell doesn't expand the
variable before belmont injects it):

```sh
belmont run -- 'curl -H "Authorization: Bearer $API_KEY" https://api.example.com'
```

## How it works

Declare secrets in `belmont.yml` as backend URIs:

```yaml
secrets:
  DATABASE_URL: "ref+env://DATABASE_URL"
  API_KEY: "ref+keyring://belmont/API_KEY"
```

`belmont run` resolves each reference, sets the values as environment
variables on the child process, executes the command in a PTY, and
replaces any occurrence of a secret value in the PTY output with
`belmont://NAME` before the output reaches the agent. The actual secret
strings never appear in output sent to the inference API.

## Backends

Two backends today:

- **Environment** (`ref+env://VAR`) — reads from the host environment.
  Read-only.
- **Keyring** (`ref+keyring://SERVICE/ACCOUNT`) — reads from the OS
  credential store (macOS Keychain, Windows Credential Manager, Linux
  secret-service). Read and write.

## Threat model

Belmont targets the most common LLM agent exfiltration patterns: an
agent cat'ing a `.env` file, echoing an environment variable while
troubleshooting API auth, or otherwise reading credentials through
ordinary shell operations. For those cases, scrubbing the value out of
PTY output is enough.

An agent that actively tries to extract secrets through side channels —
inspecting `/proc` for a subshell's environment, running a localhost
echo server and curling to it, base64-encoding values before printing
them — can probably succeed. Belmont does not defend against that.

This is a solo-developed codebase. The author is not a security
researcher. Do not use it for anything security-critical.

## Related

A loose ecosystem of tools sharing the same shape: plaintext,
git-tracked, agent-readable, no external services.

- [tisket](https://github.com/cjohnhanson/tisket) — file-based issue tracker
- [zettel](https://github.com/cjohnhanson/zettel) — zettelkasten knowledge base
- [almanac](https://github.com/cjohnhanson/almanac) — agent skill aggregator
- [mdstore](https://github.com/cjohnhanson/mdstore) — frontmattered markdown library
- [codelikecody](https://github.com/cjohnhanson/codelikecody) — workflow engine that bundles these

## License

MIT.
