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