# Task Execution Guide

> **Reference domain_map.md for domain details**

## Quick Protocol

1. **Domain ID** → Which of 5 domains? (Files, Variables, Timeline, Canvas, Platform)
2. **Pattern** → Actor+Relay, event-source naming, Cache Current Values
3. **Strategy** → Single domain = Direct tools, Multi-domain = Subagent

## Templates

### Actor+Relay Creation
```rust
let (event_relay, mut event_stream) = relay();
let actor = Actor::new(state, async move |handle| {
    let mut cached = initial;
    loop {
        select! {
            Some(v) = stream.next() => cached = v,
            Some(e) = event_stream.next() => { use_cached(cached); handle.set_neq(new); }
        }
    }
});
```

### Signal Patterns
```rust
// ✅ Direct signal
domain.actor.signal().map(render)

// ✅ Collections
.items_signal_vec(items.signal_vec().map(render_item))

// ❌ NEVER: SignalVec conversion
items.signal_vec().to_signal_cloned()  // 20+ renders per change
```

## Checklists

**Before:** Identify domain, select pattern, check integration points

**During:** Event-source naming, Cache Current Values in loops only, no raw Mutables

**After:** Compilation check, browser MCP test, no signal cascades

## Debugging

- **Performance:** Check SignalVec→Signal antipatterns, use `.dedupe()`
- **Compilation:** Verify Actor+Relay patterns, event-source naming
- **Functionality:** Trace data flow, check Actor loops, browser MCP
