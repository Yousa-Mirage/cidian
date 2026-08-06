alias fmt := format
alias doc := document

default:
    @just --list

format:
    @cargo fmt --all
    @echo "✅ All files formatted!"

lint:
    @cargo clippy --all-targets --all-features -- -D warnings
    @echo "✅ Clippy lint passed!"

test:
    @cargo test --all-targets --all-features
    @cargo test --doc --all-features
    @echo "🎉 All tests passed!"

document:
    @cargo doc --no-deps --all-features
    @echo "✅ Documentation generated successfully!"
