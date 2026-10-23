### Program to config fee sharing

## Deployments

- Program ID: `dfsdo2UqvwfN8DuUVrMRNfQe11VaiNoKcMqLHVvDPzh`


### Development

### Toolchain

| Tool           | Version                                                     |
| -------------- | ----------------------------------------------------------- |
| Rust           | `1.95.0` (pinned via `rust-toolchain.toml`)                 |
| Solana (Agave) | `4.1.2` (declared in `Anchor.toml`)                         |
| Anchor         | `1.2.1` (declared in `Anchor.toml`), `solana-v4` crates     |
| platform-tools | `v1.57`, building for SBPF `v3` (set in `package.json`)     |
| Node.js        | `>= 20.18` (required by `@solana/kit`, used by LiteSVM 1.x) |
| Bun            | latest stable                                               |

### Build

Program

```
anchor build --ignore-keys --tools-version v1.57 --arch v3
```

Solana verifiable build

```
solana-verify build --library-name dynamic_fee_sharing --arch v3
```

### Test

```
bun install
bun run build-test
```

`bun run test` runs the tests against the program already built in `target/deploy`.
When switching from a branch that uses LiteSVM 0.x, remove `node_modules` before `bun install`,
otherwise a leftover `litesvm-darwin-universal@0.1.0` can be loaded instead of LiteSVM 1.x.