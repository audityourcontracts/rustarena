# rustarena

Test with;

`RUST_LOG=info cargo run -- -t`

If you want to bypass the Website Parsers and just test a repository. e.g.;

`RUST_LOG=debug cargo run -- -t -g https://github.com/smartcontractkit/truffle-starter-kit`

Get command line arguments with

`RUST_LOG=info cargo run -- -t`

Set a larger builder pool with `-m`

`RUST_LOG=info cargo run -- -t -m 30`