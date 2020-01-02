RUST_BACKTRACE=1 cargo run "$@" 2> errors.txt ; reset; cat errors.txt
