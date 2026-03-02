# Templates component

Greentic templating node powered by Handlebars. The only exposed operation is `text`.

- Debug strings: `{{payload}}` renders compact JSON (use `{{{payload}}}` for unescaped).
- Strict scoping: rendering fails if scope identifiers are missing.

## Requirements

- Rust 1.91+
- `wasm32-wasip2` target (`rustup target add wasm32-wasip2`)

## Usage

Node authoring shape:

```yaml
nodes:
  my_template:
    templates:
      text: "My name is {{name}}"
      routing: out   # optional, defaults to out
```

Context model:
- `payload`: current input payload
- `msg`: channel message envelope
- `{{payload}}`: compact JSON strings for debugging (triple-stash to avoid HTML escaping)

Examples:
- `Payload: {{payload.name}}` → pulls from payload
- `Debug: {{{payload}}}` → raw JSON of payload
- Control flow helpers work as usual: `{{#if payload.active}}Hi{{/if}}`, `{{#each payload.items}}{{this}}{{/each}}`

## Develop

```bash
cargo build --target wasm32-wasip2
cargo test
greentic-component build --manifest ./component.manifest.json --no-flow --no-write-schema
```
