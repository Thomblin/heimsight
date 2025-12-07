# Claude Project Instructions

## Project Overview
This is an API-first web application built with Rust, Tokio, and Axum. The project follows a monorepo structure and may include CLI tools for API interaction.

---

## Development Workflow

### Test-Driven Development (TDD)
**CRITICAL: Always follow this workflow:**

1. **Write tests first** - Create failing tests for new features/fixes
2. **Run tests** - Verify tests fail for the right reasons
3. **Implement code** - Write minimal code to pass tests
4. **Validate build** - Ensure `cargo build` succeeds
5. **Run tests again** - Verify tests now pass
6. **Repeat** - Continue until all tests pass and build succeeds

**Example workflow:**
```bash
# 1. Write test in tests/ or module test section
# 2. Run tests (expect failure)
cargo test
# 3. Implement feature
# 4. Validate build
cargo build
# 5. Run tests again
cargo test
```

### Git Workflow
- **Always ask before committing** - Never create commits without explicit confirmation
- Follow conventional commit format: `type(scope): description`
  - Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`
- Batch related changes into logical commits
- Update CHANGELOG.md with each commit

---

## Code Quality Standards

### 1. Strict Linting
- Fix all `clippy` warnings: `cargo clippy -- -D warnings`
- Use `cargo fmt` to format all code
- Never suppress linting warnings without explicit justification
- Run linting before asking to commit

### 2. Type Safety
- Use strong typing throughout
- Avoid `unwrap()` and `expect()` in production code - handle errors properly
- Prefer `Result<T, E>` and `Option<T>` with proper error handling
- Use `#![deny(unsafe_code)]` unless unsafe is explicitly required
- Leverage Rust's type system for compile-time guarantees

### 3. Security First
- Follow OWASP best practices
- Validate and sanitize all input
- Use prepared statements/parameterized queries
- Implement proper authentication and authorization
- Never log sensitive data
- Use security-focused crates (e.g., `secrecy` for secrets)
- Check dependencies for vulnerabilities: `cargo audit`

### 4. Performance Focus
- Use async/await patterns with Tokio runtime
- Profile code when performance matters
- Avoid unnecessary allocations
- Use appropriate data structures
- Consider memory usage and efficiency
- Benchmark critical paths

---

## Documentation Requirements

### File-Level Documentation
Every Rust file should have a module-level doc comment:
```rust
//! Brief description of the module.
//!
//! Longer explanation of purpose, key concepts, and usage examples.
```

### Struct/Enum Documentation
```rust
/// Brief description of the type.
///
/// # Examples
///
/// ```
/// // Usage example
/// ```
pub struct MyStruct {
    /// Description of field
    field: Type,
}
```

### Function Documentation
```rust
/// Brief description of what the function does.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function returns an error and why
///
/// # Examples
///
/// ```
/// // Usage example
/// ```
pub fn my_function(param: Type) -> Result<ReturnType, Error> {
    // implementation
}
```

### README.md Updates
- Keep README current with project changes
- Update API endpoints when added/modified
- Document setup and installation steps
- Include example usage
- Maintain dependency list

### CHANGELOG.md
- Update with every significant change
- Follow [Keep a Changelog](https://keepachangelog.com/) format
- Categories: Added, Changed, Deprecated, Removed, Fixed, Security
- Include version numbers and dates

---

## Project Structure (Monorepo)

```
project-root/
├── api/                 # Axum API server
│   ├── src/
│   ├── tests/
│   └── Cargo.toml
├── cli/                 # CLI tools
│   ├── src/
│   ├── tests/
│   └── Cargo.toml
├── shared/              # Shared libraries
│   ├── models/
│   ├── utils/
│   └── Cargo.toml
├── Cargo.toml           # Workspace definition
├── README.md
├── CHANGELOG.md
└── CLAUDE_INSTRUCTIONS.md
```

---

## Decision Making

### When to Ask vs. Decide
- **Make informed decisions** based on:
  - Existing codebase patterns
  - Rust best practices
  - Project requirements above

- **Ask questions** when:
  - Multiple approaches have significant trade-offs
  - Architectural decisions impact project structure
  - Requirements are unclear or ambiguous
  - Breaking changes are needed

### What "Informed Decisions" Means
1. Read existing code to understand patterns
2. Follow established conventions in the codebase
3. Apply Rust idioms and best practices
4. Consider security, performance, and type safety
5. Document the rationale in code comments when needed

---

## Technology Stack

### Core Technologies
- **Language:** Rust (latest stable)
- **Async Runtime:** Tokio
- **Web Framework:** Axum
- **Testing:** Built-in `#[test]` + `cargo test`

### Recommended Crates
- **Error Handling:** `anyhow`, `thiserror`
- **Serialization:** `serde`, `serde_json`
- **Database:** `sqlx` (if needed)
- **Validation:** `validator`
- **Logging:** `tracing`, `tracing-subscriber`
- **Configuration:** `config`, `dotenvy`
- **Security:** `secrecy`, `argon2`

---

## Common Commands

```bash
# Development
cargo build                    # Build project
cargo run                      # Run project
cargo test                     # Run tests
cargo clippy -- -D warnings    # Lint with strict mode
cargo fmt                      # Format code
cargo audit                    # Check for vulnerabilities

# Before requesting commit
cargo fmt && cargo clippy -- -D warnings && cargo test
```

---

## Acceptance Criteria for Tasks

Before marking any task as complete, ensure:

1. ✅ Tests written and passing
2. ✅ Build succeeds (`cargo build`)
3. ✅ No clippy warnings (`cargo clippy -- -D warnings`)
4. ✅ Code formatted (`cargo fmt`)
5. ✅ Documentation added/updated
6. ✅ CHANGELOG.md updated
7. ✅ Security considerations addressed
8. ✅ Error handling implemented properly

---

## Anti-Patterns to Avoid

❌ **Don't:**
- Use `.unwrap()` or `.expect()` in production code
- Suppress clippy warnings without justification
- Skip writing tests
- Commit without explicit approval
- Make breaking changes without discussion
- Add dependencies without considering security/maintenance
- Use `unsafe` without clear necessity and documentation
- Leave TODO comments without tracking issues

✅ **Do:**
- Handle errors with `Result` and proper error types
- Write tests before implementation
- Document public APIs thoroughly
- Ask before making architectural changes
- Consider security implications
- Use type system to prevent bugs at compile time
- Profile before optimizing

---

## Example Task Workflow

**User Request:** "Add a health check endpoint to the API"

**Claude's Workflow:**

1. **Write test first:**
   ```rust
   #[tokio::test]
   async fn test_health_check_returns_ok() {
       // Test implementation
   }
   ```

2. **Run tests** (expect failure): `cargo test`

3. **Implement endpoint:**
   ```rust
   /// Health check endpoint.
   ///
   /// Returns 200 OK if the service is running.
   pub async fn health_check() -> impl IntoResponse {
       StatusCode::OK
   }
   ```

4. **Validate build:** `cargo build`

5. **Run tests again:** `cargo test`

6. **Lint and format:**
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   ```

7. **Update documentation:**
   - Add endpoint to README.md
   - Update CHANGELOG.md

8. **Ask to commit:**
   "Ready to commit the health check endpoint. Would you like me to create a commit?"

---

## Questions or Clarifications

If you need to deviate from these instructions or have questions about how to handle a specific situation, ask before proceeding.

**Last Updated:** 2025-12-07
