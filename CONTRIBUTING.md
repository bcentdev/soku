# Contributing to Ultra ⚡

Thanks for your interest in contributing to Ultra! This document outlines the process for contributing to make it as easy as possible.

## 🚀 Quick Start

### Development Setup

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/your-username/ultra
   cd ultra
   ```

2. **Install Rust** (if not already installed)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update
   ```

3. **Build and test**
   ```bash
   cargo build
   cargo test
   cargo run -- --help
   ```

4. **Run the example**
   ```bash
   cd examples/basic
   cargo run -- build --root . --outdir ./dist
   ```

## 📋 Development Workflow

### Branch Strategy

- `main` - Production ready code
- `develop` - Integration branch for features
- `feature/description` - Feature branches
- `fix/description` - Bug fix branches

### Making Changes

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Write tests for new functionality
   - Update documentation if needed
   - Follow Rust coding conventions

3. **Test your changes**
   ```bash
   cargo test
   cargo fmt
   cargo clippy
   ```

4. **Commit with conventional commits**
   ```bash
   git commit -m "feat: add tree shaking support"
   git commit -m "fix: resolve CSS import resolution"
   git commit -m "docs: update installation instructions"
   ```

5. **Push and create PR**
   ```bash
   git push origin feature/your-feature-name
   ```

## 🧪 Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

### Writing Tests

- **Unit tests**: Place in the same file as the code they test
- **Integration tests**: Place in `tests/` directory
- **Examples**: Place in `examples/` directory

Example unit test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_processing() {
        let css = "body { color: red; }";
        let result = process_css(css);
        assert!(result.is_ok());
    }
}
```

## 📝 Code Style

### Rust Conventions

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Document public APIs with `///` comments

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `test:` - Test changes
- `chore:` - Maintenance tasks

Examples:
```
feat: implement tree shaking with oxc AST
fix: resolve CSS @import paths correctly
perf: optimize parallel file processing
docs: add examples for plugin development
```

## 🚀 Areas for Contribution

### High Priority
- **Performance optimizations** - Always welcome!
- **Tree shaking** - Remove unused code from bundles
- **Source maps** - Better debugging experience
- **Error handling** - Better error messages and recovery

### Medium Priority
- **Plugin system** - Extensibility for the community
- **Watch mode** - File watching for development
- **Code splitting** - Automatic chunking strategies
- **CSS improvements** - Better CSS Modules support

### Nice to Have
- **Documentation** - More examples and guides
- **Benchmarks** - More comprehensive performance tests
- **Platform support** - Windows, macOS, Linux optimizations

## 🐛 Bug Reports

### Before Reporting

1. Check if the issue already exists
2. Try with the latest version
3. Minimize the reproduction case

### Bug Report Template

```markdown
**Describe the bug**
A clear description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Run `ultra build`
2. See error

**Expected behavior**
What you expected to happen.

**Environment**
- OS: [e.g. macOS 14.0]
- Rust version: [e.g. 1.70.0]
- Ultra version: [e.g. 0.1.0]

**Additional context**
Any other context about the problem.
```

## 💡 Feature Requests

### Feature Request Template

```markdown
**Is your feature request related to a problem?**
A clear description of what the problem is.

**Describe the solution you'd like**
A clear description of what you want to happen.

**Describe alternatives you've considered**
Alternative solutions you've considered.

**Additional context**
Any other context about the feature request.
```

## 📚 Documentation

- Update README.md for user-facing changes
- Update inline documentation for code changes
- Add examples for new features
- Update CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/)

## 🔄 Release Process

### Versioning

We use [Semantic Versioning](https://semver.org/):
- `MAJOR.MINOR.PATCH`
- Breaking changes increment MAJOR
- New features increment MINOR
- Bug fixes increment PATCH

### Release Checklist

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Create release PR
4. Tag release after merge
5. Publish to crates.io
6. Create GitHub release

## 🤝 Community Guidelines

- **Be respectful** - Treat everyone with respect
- **Be helpful** - Help others learn and contribute
- **Be patient** - Reviews take time, be understanding
- **Be collaborative** - Work together towards common goals

## ❓ Questions?

- Open a [Discussion](https://github.com/yourusername/ultra/discussions) for questions
- Join our [Discord](https://discord.gg/ultra) for real-time chat
- Check existing [Issues](https://github.com/yourusername/ultra/issues) and [PRs](https://github.com/yourusername/ultra/pulls)

---

**Thank you for contributing to Ultra! 🚀**