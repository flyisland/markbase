export MARKBASE_BASE_DIR := "../md-notes"

set positional-arguments := true
set shell := ["bash", "-c"]

default:
    @just -l

# 执行 markbase 命令
markbase *args:
    cargo run -- "$@"

index *args:
    cargo run -- index "$@"

query *args:
    cargo run -- query "$@"

# 检查问题
verify:
    cargo clippy
    cargo test
