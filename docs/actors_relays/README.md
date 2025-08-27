# Actor+Relay Architecture Documentation

Complete documentation for Actor+Relay architecture - a reactive state management pattern that eliminates common problems in large MoonZoon applications through controlled state mutations and typed message passing.

## 📚 Documentation Structure

### **[moonzoon/](moonzoon/)** - Generic MoonZoon Patterns
Ready-to-use Actor+Relay patterns for any MoonZoon application:
- **[Architecture](moonzoon/architecture.md)** - Core concepts and API design
- **[Patterns](moonzoon/patterns.md)** - Migration strategies and modern patterns  
- **[Examples](moonzoon/examples.md)** - Generic examples (Counter, Todo, Chat, Resource Manager)
- **[Testing Guide](moonzoon/testing.md)** - Testing and debugging strategies
- **[External API Bridging](moonzoon/bridging.md)** - ConnectionAdapter patterns for SSE+Fetch, HTTP, etc.
- **[Refactoring Guide](moonzoon/refactoring.md)** - Step-by-step migration from Mutables

### **[novywave/](novywave/)** - NovyWave Implementation Examples
Real-world implementation experience from NovyWave's migration:
- **[Migration Lessons](novywave/migration_lessons.md)** - Lessons learned from migrating 69+ global Mutables
- **[File Manager](novywave/file_manager.md)** - Waveform file management patterns
- **[Variable Selection](novywave/variable_selection.md)** - Signal selection and timeline integration

### **[verified_examples/](verified_examples/)** - Carefully Verified Examples
Original, thoroughly tested non-global Actor+Relay examples:
- **[Counter Example](verified_examples/counter_example.md)** - Local counter implementation
- **[Chat Example](verified_examples/chat_example.md)** - Local chat with SSE+Fetch connection
- **[Counters Example](verified_examples/counters_example.md)** - Dynamic collection patterns

### **[legacy_examples/](legacy_examples/)** - Global Patterns (Bridge Documentation)
Global state patterns for migration from traditional MoonZoon:
- **[Counter Global](legacy_examples/counter_global.md)** - Global counter patterns
- **[Chat Global](legacy_examples/chat_global.md)** - Global chat patterns
- **[Counters Global](legacy_examples/counters_global.md)** - Global collection patterns

### **[archive/](archive/)** - Historical Documentation
Previous versions and development history for reference.

## Quick Start

```rust
use futures::select;

// Create event relays
let (increment, mut increment_stream) = relay();
let (decrement, mut decrement_stream) = relay();

// Create Actor for sequential state processing
let counter = Actor::new(0, async move |state| {
    loop {
        select! {
            Some(()) = increment_stream.next() => state.update(|n| n + 1),
            Some(()) = decrement_stream.next() => state.update(|n| n - 1),
        }
    }
});

// Emit events from UI
increment.send(());

// Bind to reactive signals
counter.signal()  // Always current state
```

## Problems Solved

❌ **Unclear mutation sources** - Multiple files modifying global state  
❌ **Recursive lock panics** - Signal handlers triggering more mutations  
❌ **Over-rendering issues** - Signal cascades causing 30+ UI updates  
❌ **Race conditions** - Concurrent access to shared state  
❌ **Testing difficulties** - Global state hard to isolate

## Benefits Achieved

✅ **No recursive locks** - Sequential message processing  
✅ **Full traceability** - Every state change logged with source  
✅ **Excellent testability** - Actors tested in isolation  
✅ **Type safety** - Compile-time message validation  
✅ **Clean architecture** - Clear separation of concerns

## Real-World Success

This architecture has been battle-tested in NovyWave, a complex waveform viewer that successfully migrated from 69+ global Mutables to Actor+Relay patterns, achieving:

- **Zero recursive lock panics**
- **85% reduction in UI over-rendering**
- **Complete state mutation traceability**
- **80%+ test coverage improvement**

See `novywave/` for detailed real-world implementation examples.

## Technology Stack

- **MoonZoon framework** - Full-stack Rust web development
- **SSE (Server-Sent Events) + Fetch** - Client-server communication
- **Actor+Relay** - State management and message passing
- **Signal-based reactivity** - UI updates and data flow

## Integration with MoonZoon

Actor+Relay is designed as a standalone module that can be:
- **Used in any MoonZoon application** - Copy patterns from `moonzoon/` directory
- **Extracted as `zoon-actors` crate** - Independent library for broader ecosystem
- **Integrated directly into MoonZoon framework** - When patterns are proven stable

The patterns work seamlessly with MoonZoon's reactive UI system while solving the state management challenges that emerge in complex applications.

## Navigation Guide

### 🚀 **Getting Started**
→ Start with `moonzoon/architecture.md` for core concepts  
→ Follow `moonzoon/examples.md` for practical patterns  
→ Use `moonzoon/refactoring.md` for migration guidance

### 🏗️ **Building Applications**  
→ Reference `verified_examples/` for tested implementations  
→ Use `moonzoon/bridging.md` for external API integration  
→ Follow `moonzoon/testing.md` for testing strategies

### 📚 **Real-World Learning**
→ Read `novywave/migration_lessons.md` for migration experience  
→ Study `novywave/` examples for domain-specific patterns  
→ Reference `legacy_examples/` for global state migration

### 🔧 **Legacy Migration**
→ Use `legacy_examples/` as stepping stones from global Mutables  
→ Follow `moonzoon/refactoring.md` for systematic migration  
→ Learn from `novywave/migration_lessons.md` real-world experience

This documentation provides everything needed to successfully adopt Actor+Relay architecture in MoonZoon applications, from basic concepts to advanced real-world patterns.