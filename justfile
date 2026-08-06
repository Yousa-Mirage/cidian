alias fmt := format
alias doc := document

# List all available recipes.
default:
    @just --list

# Format all Rust source files with rustfmt.
format:
    @cargo fmt --all
    @echo "✅ All files formatted!"

# Run Clippy for all targets and treat warnings as errors.
lint:
    @cargo clippy --all-targets --all-features -- -D warnings
    @echo "✅ Clippy lint passed!"

# Run unit, integration, and documentation tests.
test:
    @cargo test --lib --tests --all-features
    @cargo test --doc --all-features
    @echo "🎉 All tests passed!"

# Run the parser benchmark against the largest real fixture for each format.
bench:
    @cargo bench --bench parse

# Generate API documentation. Pass `open` to open the generated index page.
document open="":
    @cargo doc --no-deps --all-features {{ if open == "open" { "--open" } else { "" } }}
    @echo "✅ Documentation generated successfully!"
