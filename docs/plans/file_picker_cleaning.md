# File Picker Cleaning Plan

## 🔴 CRITICAL CONTEXT FOR FUTURE SESSIONS

**Problem**: The file picker implementation in `frontend/src/config.rs` and `frontend/src/file_picker.rs` contains experimental patterns, workarounds, and legacy bridge code from an incomplete Actor+Relay architecture migration. The codebase has dual state management (Actor + Mutable), placeholder functions, and excessive use of zoon::Task instead of proper Actor patterns.

**Goal**: Clean up all TODOs, remove experimental dual patterns, eliminate legacy bridge code, and implement proper Actor+Relay architecture throughout.

**Files to Clean**:
- `frontend/src/config.rs` - Contains FilePickerDomain with experimental patterns
- `frontend/src/file_picker.rs` - Contains placeholder functions and deprecated code

---

## 📋 TODO List

### 📁 **config.rs** - FilePickerDomain Experimental Pattern Cleanup

#### TODO-001: Remove Dual Actor+Mutable Pattern for directory_cache
**Location**: Lines 80-81, 163-183, and all references
**Problem**: `directory_cache_mutable` maintains duplicate state alongside `directory_cache_actor`, violating single source of truth
**Current Code Pattern**:
```rust
// Line 81: Experimental dual state
pub directory_cache_mutable: zoon::Mutable<std::collections::HashMap<String, Vec<shared::FileSystemItem>>>,

// Lines 178-180: Dual update antipattern
state.set(cache.clone());
cache_mutable_for_sync.set_neq(cache);
```
**Solution**:
1. Remove `directory_cache_mutable` field from struct
2. Remove line 164: `let directory_cache_mutable = zoon::Mutable::new(...)`
3. Remove line 167: `let cache_mutable_for_sync = directory_cache_mutable.clone()`
4. Remove lines 179-180: dual update code
5. Update any TreeView components to use `directory_cache_actor.signal()` directly

**Verification**: TreeView should still show directory contents using only Actor signals

---

#### TODO-002: Remove Debug Signal Test Code
**Location**: Lines 192-200
**Problem**: Test code that monitors Actor signals for debugging, creates unnecessary Task
**Solution**: Delete entire block from `{` at line 192 to `}` at line 200
**Verification**: No functional impact, just removes debug logging

---

#### TODO-003: Remove Legacy Backward Compatibility Mutables
**Location**: Lines 323-325, 750-752, 823-824
**Problem**: Duplicate state for backward compatibility during transition
```rust
// Lines 323-325
pub file_picker_expanded_directories: Mutable<indexmap::IndexSet<String>>,
pub file_picker_scroll_position: Mutable<i32>,
```
**Solution**:
1. Remove fields from AppConfig struct (lines 324-325)
2. Remove initialization code (lines 750-752)
3. Remove from struct construction (lines 823-824)
4. Update all references to use `file_picker_domain.expanded_directories_actor` instead

**Verification**: File picker dialog still maintains expanded state and scroll position

---

#### TODO-004: Fix Platform Layer Workaround
**Location**: Lines 356-358
**Problem**: Platform layer is broken, using hardcoded defaults
```rust
zoon::println!("🚨 CONFIG: Platform layer broken, using defaults for now");
Ok(SharedAppConfig::default())
```
**Solution**:
1. Investigate why platform layer is broken
2. Either fix the platform layer OR
3. Document this as permanent and remove the "TEMPORARY" comment
4. Consider using connection-based config loading instead

**Verification**: Config loads properly from backend or uses documented fallback

---

#### TODO-005: Replace Task-based Config Saving with Actor Pattern
**Location**: Lines 632-654, 677-747
**Problem**: Using zoon::Task for config saving instead of Actor+Relay pattern
**Solution**:
1. Create a ConfigSaver actor that handles save requests
2. Use debouncing Actor pattern from documentation:
```rust
let debounce_actor = Actor::new((), async move |_state| {
    loop {
        select! {
            Some(()) = save_requested_stream.next() => {
                loop {
                    select! {
                        Some(()) = save_requested_stream.next() => continue,
                        _ = Timer::sleep(300) => {
                            perform_save().await;
                            break;
                        }
                    }
                }
            }
        }
    }
});
```
3. Remove all Task::start calls for config saving

**Verification**: Config still saves with debouncing, no zoon::Task usage

---

#### TODO-006: Remove Backend Sender Monitoring Task
**Location**: Lines 238-263
**Problem**: Uses Task to monitor pending loads instead of Actor pattern
**Solution**:
1. Integrate backend sending directly into the loading Actor
2. OR create a dedicated BackendSender actor
3. Remove the entire Task::start block

**Verification**: Directory loading still works without Task

---

#### TODO-007: Remove Dummy Connection Workaround
**Location**: Lines 881-892
**Problem**: Creates dummy connection for initialization without real connection
**Solution**:
1. Either remove this function entirely OR
2. Document why it's needed and make it clear it's not temporary
3. Consider if this is actually needed or if all initialization should require connection

**Verification**: App initialization works correctly

---

#### TODO-008: Clean up Scroll Sync Actor
**Location**: Lines 756-775
**Problem**: Complex bridge pattern for scroll position sync
**Solution**:
1. Analyze if this is still needed after removing legacy Mutables
2. If needed, simplify to direct relay connections
3. If not needed, remove entirely

**Verification**: Scroll position still persists correctly

---

### 📁 **file_picker.rs** - Placeholder and Deprecated Function Cleanup

#### TODO-009: Remove Placeholder Cache Update Functions
**Location**: Lines 20-29
**Problem**: Temporary placeholder functions that should use FilePickerDomain
```rust
pub fn update_global_directory_cache(_path: String, _items: Vec<shared::FileSystemItem>) {
    // TODO: Replace with proper FilePickerDomain.directory_contents_received_relay.send()
}
```
**Solution**:
1. Find all callers of these functions (likely in connection.rs)
2. Update callers to use `app_config.file_picker_domain.directory_contents_received_relay.send()`
3. Delete both placeholder functions

**Verification**: Directory loading still works after removal

---

#### TODO-010: Remove Deprecated load_directories_on_demand
**Location**: Lines 509-516
**Problem**: Deprecated function kept as stub
**Solution**: Delete entire function
**Verification**: No compilation errors after removal

---

#### TODO-011: Fix TreeView State Synchronization
**Location**: Line 373
**Problem**: TODO comment about replacing Task::start pattern
**Solution**:
1. Implement proper one-way sync from Atom to MutableVec without Task
2. OR remove the sync entirely if TreeView can use Atom directly

**Verification**: Selected files sync correctly between Atom and TreeView

---

#### TODO-012: Implement Proper Scroll Position Persistence
**Location**: Line 378
**Problem**: TODO about implementing scroll position persistence
**Solution**:
1. Add scroll position tracking to FilePickerDomain
2. Connect scroll events to domain relay
3. Persist scroll position through config

**Verification**: Scroll position restored when dialog reopens

---

#### TODO-013: Remove Keep-Alive Timer Antipattern
**Location**: Line 67 in initialize_default_directories
**Problem**: Uses Timer::sleep(10) to keep actor alive
```rust
loop {
    zoon::Timer::sleep(10).await; // Keep actor alive
}
```
**Solution**:
1. Replace with proper Actor lifecycle management
2. OR use `std::future::pending().await` if actor needs to stay alive
3. OR redesign to not need a keep-alive pattern

**Verification**: Directory initialization still works

---

#### TODO-014: Clean up Actor Creation in Initialization
**Location**: Lines 36-69
**Problem**: Creates Actor inline for initialization logic
**Solution**:
1. Move this logic to FilePickerDomain initialization
2. OR create a proper named initialization actor
3. Remove inline Actor creation pattern

**Verification**: Default directories still initialize correctly

---

### 🔍 **Investigation Tasks**

#### TODO-015: Audit All zoon::Task Usage
**Action**: Search for all `zoon::Task::start` in both files
**Goal**: Replace with Actor+Relay patterns or document why Task is necessary
**Command**: `rg "zoon::Task::start" frontend/src/config.rs frontend/src/file_picker.rs`

---

#### TODO-016: Find All TODO Comments
**Action**: Search for remaining TODO comments
**Goal**: Address or remove all TODO comments
**Command**: `rg "TODO" frontend/src/config.rs frontend/src/file_picker.rs`

---

#### TODO-017: Check for TEMPORARY Markers
**Action**: Search for "TEMPORARY" comments
**Goal**: Remove or make permanent
**Command**: `rg "TEMPORARY" frontend/src/config.rs frontend/src/file_picker.rs`

---

## ✅ **Verification Checklist After Cleanup**

1. [ ] No dual Actor+Mutable patterns remain
2. [ ] No placeholder functions exist
3. [ ] No deprecated functions remain
4. [ ] All TODO comments addressed or removed
5. [ ] No unnecessary zoon::Task usage
6. [ ] TreeView still shows directories correctly
7. [ ] File picker dialog works fully
8. [ ] Config saves and loads properly
9. [ ] Scroll position persists
10. [ ] Directory expansion state persists
11. [ ] No compilation warnings about unused code
12. [ ] No runtime errors in browser console

---

## 📊 **Success Metrics**

- **Line reduction**: Expect ~200-300 lines removed
- **Task reduction**: Remove at least 5 zoon::Task::start calls
- **Pattern compliance**: 100% Actor+Relay architecture
- **TODO elimination**: 0 TODO comments remaining
- **Code clarity**: Single source of truth for all state

---

## 🚨 **Critical Notes for Future Sessions**

1. **Do NOT add new workarounds** - fix the root cause
2. **Do NOT use Mutable** - use Actor for domain state, Atom for UI state
3. **Do NOT use zoon::Task** - use Actor patterns for async operations
4. **Test each TODO completion** individually before moving to next
5. **The FilePickerDomain should be the ONLY source of truth** for file picker state

---

## 📈 **Progress Tracking**

### Session 1: [Date]
- [ ] TODO-001: Remove Dual Actor+Mutable Pattern
- [ ] TODO-002: Remove Debug Signal Test Code
- [ ] TODO-003: Remove Legacy Backward Compatibility Mutables

### Session 2: [Date]
- [ ] TODO-004: Fix Platform Layer Workaround
- [ ] TODO-005: Replace Task-based Config Saving
- [ ] TODO-006: Remove Backend Sender Monitoring Task

### Session 3: [Date]
- [ ] TODO-007: Remove Dummy Connection Workaround
- [ ] TODO-008: Clean up Scroll Sync Actor
- [ ] TODO-009: Remove Placeholder Cache Update Functions

### Session 4: [Date]
- [ ] TODO-010: Remove Deprecated Function
- [ ] TODO-011: Fix TreeView State Sync
- [ ] TODO-012: Implement Scroll Position Persistence

### Session 5: [Date]
- [ ] TODO-013: Remove Keep-Alive Timer
- [ ] TODO-014: Clean up Actor Creation
- [ ] TODO-015-017: Investigation and Final Cleanup

---

## 🎯 **Execution Order**

**Recommended order for maximum safety**:
1. Start with simple removals (TODO-002, TODO-010)
2. Remove placeholder functions (TODO-009)
3. Clean up dual patterns (TODO-001)
4. Remove legacy compatibility (TODO-003)
5. Fix architectural issues (TODO-005, TODO-006)
6. Handle remaining workarounds
7. Final investigation and cleanup

Each TODO should be completed and tested before moving to the next.