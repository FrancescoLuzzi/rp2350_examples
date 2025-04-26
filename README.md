# rp2350_examples

Install probe-rs

```shell
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
```

Install elf2uf2-rs with support for rp2350

```shell
# clone from https://github.com/JoNil/elf2uf2-rs/pull/39 and run `cargo install --path .`
# if elf2uf2-rs released a version > 2.1.1
cargo install elf2uf2-rs
```
