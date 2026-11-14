# Release checklist

A synthetic paste for tests and local screenshots. It holds one of each block the reading page draws: headings at three levels, a long paragraph, a table wider than a phone, fenced code in several languages, lists and a quote.

## Before the tag

The release waits until every gate on main is green and the changelog names each user-visible change. A long paragraph like this one checks the reading column: its line length, its line height and how it wraps around inline `code`, **strong text**, *emphasis* and [a link](https://example.org/release). Nothing in it may push the page sideways, even when a token such as `a-very-long-identifier-that-does-not-break-at-a-hyphen-boundary` is wider than a phone.

1. Merge the approved pull requests.
2. Run the merge gate on the merged commit.
3. Write the changelog entry.

- Unordered item
  - Nested item with `inline code`
- Another item

> A quote: the release notes say what changed for the reader, not how the work went.

### Gate summary

| Gate | Command | Runs where | Wall-time limit | Baseline | Owner | Note |
| --- | --- | --- | --- | --- | --- | --- |
| Lint | `pnpm lint` | pull requests | 30 s | 12 s | web | fails on a raw colour or pixel value |
| Type check | `pnpm typecheck` | pull requests | 30 s | 9 s | web | strict mode, no unchecked index access |
| Unit tests | `pnpm test` | pull requests | 30 s | 4 s | web | shell contract and outline |
| Build | `pnpm build` | pull requests | 60 s | 15 s | web | reports the entry chunk size |

## Code samples

### Rust

```rust
fn escape_script(json: &str) -> String {
    json.replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026")
}
```

### Python

```python
def expires_in(days: int) -> int:
    """Seconds for an expiry given in days."""
    return days * 24 * 60 * 60
```

### TOML

```toml
[server]
address = "0.0.0.0:8088"
max_body_size = 67108864
```

### Shell

```sh
curl --data-binary @notes.md https://paste.example.org/
```

## After the tag

Check the image digest in the release notes, then deploy by digest.
