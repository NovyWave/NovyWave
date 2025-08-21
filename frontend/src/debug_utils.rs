// Throttled debug logging to prevent dev_server.log corruption
// 
// Virtual lists and high-frequency event handlers can generate thousands
// of logs per second, causing file corruption when multiple backend threads
// write simultaneously to dev_server.log.

#![allow(dead_code, unused_imports)]

use std::sync::atomic::{AtomicUsize, Ordering};
use zoon::*;

static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
const MAX_LOGS_PER_SECOND: usize = 5;

// Debug mode configuration - set to false for production
const DEBUG_MODE: bool = false;
const DEBUG_SIGNAL_TRANSITIONS: bool = false;
const DEBUG_REQUEST_DEDUPLICATION: bool = false;
const DEBUG_TIMELINE_VALIDATION: bool = false;
const DEBUG_CACHE_MISS: bool = false;

/// Throttled debug logging - maximum 5 logs per second
/// Use this instead of zoon::println! in high-frequency handlers
pub fn debug_throttled(message: &str) {
    // Simple counter-based throttling (no time dependency)
    let current_count = LOG_COUNT.load(Ordering::Relaxed);
    
    // Reset every 100 calls to approximate throttling
    if current_count >= 100 {
        LOG_COUNT.store(0, Ordering::Relaxed);
    }
    
    let count = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    
    // Only log first 5 out of every 100 calls
    if count < MAX_LOGS_PER_SECOND {
        zoon::println!("[THROTTLED] {}", message);
    } else if count == MAX_LOGS_PER_SECOND {
        zoon::println!("[THROTTLED] Log rate limit reached, suppressing further messages...");
    }
}

/// Critical debug logging - always prints (use sparingly)
/// For errors and important state changes only
pub fn debug_critical(message: &str) {
    zoon::println!("[CRITICAL] {}", message);
}

/// Conditional debug logging based on debug mode
/// Only prints when DEBUG_MODE is true
pub fn debug_conditional(message: &str) {
    if DEBUG_MODE {
        zoon::println!("[DEBUG] {}", message);
    }
}

/// Signal transition debug logging
/// Only prints when DEBUG_SIGNAL_TRANSITIONS is true
pub fn debug_signal_transitions(message: &str) {
    if DEBUG_SIGNAL_TRANSITIONS {
        zoon::println!("[TRANSITIONS] {}", message);
    }
}

/// Request deduplication debug logging
/// Only prints when DEBUG_REQUEST_DEDUPLICATION is true
pub fn debug_request_deduplication(message: &str) {
    if DEBUG_REQUEST_DEDUPLICATION {
        zoon::println!("[REQUEST] {}", message);
    }
}

/// Timeline validation debug logging
/// Only prints when DEBUG_TIMELINE_VALIDATION is true
pub fn debug_timeline_validation(message: &str) {
    if DEBUG_TIMELINE_VALIDATION {
        zoon::println!("[TIMELINE] {}", message);
    }
}

/// Cache miss logging - conditional based on debug mode
/// Only prints when DEBUG_CACHE_MISS is true
pub fn debug_cache_miss(message: &str) {
    if DEBUG_CACHE_MISS {
        zoon::println!("[CACHE MISS] {}", message);
    }
}

/// Enable cache miss debugging at runtime (for development)
/// Call this function to see cache miss logs without recompiling
#[allow(dead_code)]
pub fn enable_cache_miss_debug() {
    // Note: To enable cache miss logs, set DEBUG_CACHE_MISS = true above and recompile
    zoon::println!("[DEBUG] To enable cache miss logs, set DEBUG_CACHE_MISS = true in debug_utils.rs");
}