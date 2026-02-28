set positional-arguments := true

default:
    @just -l

# 执行 markbase 命令
markbase *args:
    cargo run -- "$@"

# 检查问题
verify:
    cargo clippy
    cargo test
