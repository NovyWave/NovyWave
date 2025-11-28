# Contributing

Thank you for your interest in contributing to NovyWave! This guide will help you get started.

## Ways to Contribute

- **Bug Reports** - Found a problem? Open an issue
- **Feature Requests** - Have an idea? Start a discussion
- **Code Contributions** - Submit pull requests
- **Documentation** - Improve guides and examples
- **Testing** - Try NovyWave and report issues

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork:**
   ```bash
   git clone https://github.com/YOUR_USERNAME/NovyWave.git
   cd NovyWave
   ```
3. **Set up development environment** - See [Building from Source](./building.md)
4. **Create a branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Code Style

### Rust Conventions

- Follow standard Rust formatting (`rustfmt`)
- Use meaningful variable and function names
- Prefer explicit types over inference for public APIs
- Document public functions with doc comments

### Actor+Relay Architecture

NovyWave uses Actor+Relay for state management. Key rules:

- **No raw Mutables** - Use Actor+Relay or Atom
- **Event-source naming** - `button_clicked_relay`, not `add_item`
- **Domain-driven design** - `TrackedFiles`, not `FileManager`
- **No Manager/Service patterns** - Avoid enterprise abstractions

See [Actor+Relay Pattern](./actor-relay.md) for details.

### Code Organization

- Keep related code together
- Prefer small, focused functions
- Use the existing module structure

## Pull Request Process

### Before Submitting

1. **Test your changes:**
   ```bash
   makers start  # Test in browser
   makers tauri  # Test desktop
   ```

2. **Ensure code compiles:**
   ```bash
   makers build
   ```

3. **Check for warnings:**
   Review the build output for any warnings

### Submitting

1. **Push your branch:**
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Open a Pull Request** on GitHub

3. **Describe your changes:**
   - What does this PR do?
   - Why is it needed?
   - How was it tested?

### Review Process

- Maintainers will review your PR
- Address any feedback
- Once approved, your PR will be merged

## Reporting Bugs

### Good Bug Reports Include

- NovyWave version
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Error messages (from terminal if available)
- Waveform file details (if relevant)

### Where to Report

- **GitHub Issues** for bugs and feature requests
- **GitHub Discussions** for questions and ideas

## Development Tips

### Useful Commands

```bash
makers start       # Development server
makers tauri       # Desktop development
makers build       # Production build
makers clean       # Clean build artifacts
```

### Debugging

- Use `zoon::println!()` for console logging
- Check browser console for JavaScript errors
- Check terminal for Rust compilation errors

### Testing Changes

1. **Load a waveform file** (test files in `test_files/`)
2. **Try different features:**
   - File loading
   - Scope selection
   - Timeline navigation
   - Keyboard shortcuts
3. **Test both themes** (Ctrl+T)
4. **Test both dock modes** (Ctrl+D)

## Architecture Overview

- **frontend/** - Rust/WASM UI code
- **backend/** - MoonZoon backend (browser mode)
- **shared/** - Types shared between frontend/backend
- **src-tauri/** - Tauri desktop wrapper

See [Architecture Overview](./architecture.md) for details.

## Questions?

- Open a Discussion on GitHub
- Check existing issues for similar questions
- Review the documentation

Thank you for contributing to NovyWave!
