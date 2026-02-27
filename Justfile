set positional-arguments := true

default:
    @just -l

# 执行 mdb 命令
mdb *args:
    cargo run -- "$@"

# 检查问题
verify:
    cargo clippy
    cargo test
