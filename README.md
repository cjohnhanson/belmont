# belmont

Belmont is a secrets manager for LLM coding agents. It resolves credentials
from pluggable backends. It injects the values into commands at run time. It
scrubs the values out of the command output before an agent reads the output.
An agent can use a secret, and the secret does not enter the agent context
window.

## Install

```sh
cargo install --git https://github.com/cjohnhanson/belmont
```

## Usage

```sh
belmont init              # make belmont.yml
belmont list              # show the declared secret references, never the values
belmont check             # check that Belmont can resolve every secret
belmont set <name> [val]  # store a secret in its backend
belmont run <command>     # run a command with the secrets injected and the output scrubbed
```

A typical run. Use single quotes. The shell then does not expand the variable
before Belmont injects it.

```sh
belmont run -- 'curl -H "Authorization: Bearer $API_KEY" https://api.example.com'
```

## How it works

Declare the secrets in `belmont.yml` as backend URIs:

```yaml
secrets:
  DATABASE_URL: "ref+env://DATABASE_URL"
  API_KEY: "ref+keyring://belmont/API_KEY"
```

`belmont run` resolves each reference. It sets the values as environment
variables on the child process. It runs the command in a PTY. It replaces each
secret value in the PTY output with `belmont://NAME` before the output reaches
the agent. The secret strings never appear in the output that goes to the
inference API.

## Backends

Belmont has two backends:

- **Environment** (`ref+env://VAR`) — reads from the host environment.
  Read-only.
- **Keyring** (`ref+keyring://SERVICE/ACCOUNT`) — reads from the OS credential
  store: the macOS Keychain, the Windows Credential Manager, or the Linux
  secret-service. Read and write.

## Threat model

Belmont covers the most common exfiltration patterns of an LLM agent. An agent
runs `cat` on a `.env` file. An agent echoes an environment variable while it
troubleshoots API authentication. An agent reads credentials through other
ordinary shell operations. For these cases, Belmont scrubs the value out of the
PTY output, and that is enough.

An agent can also attack a side channel. It can inspect `/proc` for the
environment of a subshell. It can run a localhost echo server and send the
value to that server with `curl`. It can encode a value as base64 before it
prints the value. Such an agent will probably succeed. Belmont does not defend
against these attacks.

One person wrote this codebase. The author is not a security researcher. Do not
use Belmont for anything security-critical.

## Related

These tools share the same shape. They use plain text. Git tracks them. Agents
read them. They need no external service.

- [tisket](https://github.com/cjohnhanson/tisket) — file-based issue tracker
- [zettel](https://github.com/cjohnhanson/zettel) — zettelkasten knowledge base
- [almanac](https://github.com/cjohnhanson/almanac) — agent skill aggregator
- [mdstore](https://github.com/cjohnhanson/mdstore) — frontmattered markdown library
- [codelikecody](https://github.com/cjohnhanson/codelikecody) — workflow engine that bundles these

## License

MIT.
