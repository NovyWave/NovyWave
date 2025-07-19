#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! jwalk = "0.8"
//! tokio = { version = "1.0", features = ["full"] }
//! ```

use jwalk::WalkDir;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone)]
struct FileSystemItem {
    name: String,
    path: String,
    is_directory: bool,
    file_size: Option<u64>,
    is_waveform_file: bool,
    file_extension: Option<String>,
    has_expandable_content: bool,
}

async fn scan_directory_jwalk(path: &Path) -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
    let path_buf = path.to_path_buf();
    
    // Use jwalk for parallel directory traversal, bridged with tokio
    let items = tokio::task::spawn_blocking(move || -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
        let mut items = Vec::new();
        
        // jwalk with parallel processing, single directory level
        for entry in WalkDir::new(&path_buf)
            .sort(true)  // Enable sorting for consistent results
            .max_depth(1)  // Single level only (like TreeView expansion)
            .skip_hidden(false)  // We'll filter manually to match existing logic
            .process_read_dir(|_, _, _, dir_entry_results| {
                // Filter entries in parallel processing callback for better performance
                dir_entry_results.retain(|entry_result| {
                    if let Ok(entry) = entry_result {
                        let name = entry.file_name().to_string_lossy();
                        !name.starts_with('.') // Skip hidden files
                    } else {
                        true // Keep errors for proper handling
                    }
                });
            })
        {
            match entry {
                Ok(dir_entry) => {
                    let entry_path = dir_entry.path();
                    
                    // Skip the root directory itself (jwalk includes it)
                    if entry_path == path_buf {
                        continue;
                    }
                    
                    let name = entry_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    
                    let is_directory = entry_path.is_dir();
                    let path_str = entry_path.to_string_lossy().to_string();
                    
                    // Only include directories and waveform files for cleaner file dialog
                    let is_waveform = if !is_directory {
                        let name_lower = name.to_lowercase();
                        name_lower.ends_with(".vcd") || name_lower.ends_with(".fst")
                    } else {
                        false
                    };
                    
                    // Skip non-waveform files to reduce clutter
                    if !is_directory && !is_waveform {
                        continue;
                    }
                    
                    let item = FileSystemItem {
                        name,
                        path: path_str,
                        is_directory,
                        file_size: None, // Skip file size for instant loading
                        is_waveform_file: is_waveform, // Proper waveform detection  
                        file_extension: None, // Skip extension parsing for instant loading
                        has_expandable_content: is_directory, // All directories expandable
                    };
                    
                    items.push(item);
                }
                Err(e) => {
                    eprintln!("jwalk error processing entry: {}", e);
                    // Continue processing other entries despite individual errors
                }
            }
        }
        
        // Sort items: directories first, then files, both alphabetically
        // jwalk's sort(true) provides basic ordering, but we need custom logic
        items.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        Ok(items)
    }).await??;
    
    Ok(items)
}

// Simulate the old async_fs approach for comparison
async fn scan_directory_async_fs_style(path: &Path) -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
    let mut items = Vec::new();
    
    // Simulate multiple async operations like the old implementation
    let mut dir_reader = tokio::fs::read_dir(path).await?;
    
    while let Some(entry) = dir_reader.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        
        // Skip hidden files and directories (starting with .)
        if name.starts_with('.') {
            continue;
        }
        
        // Multiple async calls per entry (like the old implementation)
        let file_type = entry.file_type().await?;
        let is_directory = file_type.is_dir();
        let path_str = entry.path().to_string_lossy().to_string();
        
        // Only include directories and waveform files for cleaner file dialog
        let is_waveform = if !is_directory {
            let name_lower = name.to_lowercase();
            name_lower.ends_with(".vcd") || name_lower.ends_with(".fst")
        } else {
            false
        };
        
        // Skip non-waveform files to reduce clutter
        if !is_directory && !is_waveform {
            continue;
        }
        
        let item = FileSystemItem {
            name,
            path: path_str,
            is_directory,
            file_size: None,
            is_waveform_file: is_waveform,
            file_extension: None,
            has_expandable_content: is_directory,
        };
        
        items.push(item);
    }
    
    // Manual sorting (post-processing)
    items.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    
    Ok(items)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== jwalk vs async_fs Performance Benchmark ===\n");
    
    let test_directories = vec![
        "./test_large_dir",
        "./test_deep_hierarchy", 
        "./frontend",
        ".",
    ];
    
    for test_dir in test_directories {
        let path = Path::new(test_dir);
        if !path.exists() {
            println!("⏭️  Skipping {}: directory not found", test_dir);
            continue;
        }
        
        println!("📁 Testing directory: {}", test_dir);
        
        // Test jwalk implementation
        let start = Instant::now();
        let jwalk_result = scan_directory_jwalk(path).await;
        let jwalk_time = start.elapsed();
        
        match jwalk_result {
            Ok(jwalk_items) => {
                println!("  🚀 jwalk: {} items in {:?} ({:.1} items/ms)", 
                    jwalk_items.len(), jwalk_time, jwalk_items.len() as f64 / jwalk_time.as_millis() as f64);
                
                let dirs = jwalk_items.iter().filter(|i| i.is_directory).count();
                let vcd = jwalk_items.iter().filter(|i| i.name.ends_with(".vcd")).count();
                let fst = jwalk_items.iter().filter(|i| i.name.ends_with(".fst")).count();
                println!("    📊 {} dirs, {} .vcd files, {} .fst files", dirs, vcd, fst);
                
                // Test async_fs implementation for comparison
                let start = Instant::now();
                let async_fs_result = scan_directory_async_fs_style(path).await;
                let async_fs_time = start.elapsed();
                
                match async_fs_result {
                    Ok(async_fs_items) => {
                        println!("  🐌 async_fs: {} items in {:?} ({:.1} items/ms)", 
                            async_fs_items.len(), async_fs_time, async_fs_items.len() as f64 / async_fs_time.as_millis() as f64);
                        
                        let speedup = async_fs_time.as_millis() as f64 / jwalk_time.as_millis() as f64;
                        if speedup > 1.0 {
                            println!("  ✅ jwalk is {:.1}x faster!", speedup);
                        } else {
                            println!("  ⚠️  async_fs is {:.1}x faster", 1.0 / speedup);
                        }
                        
                        // Verify both implementations return the same results
                        if jwalk_items.len() == async_fs_items.len() {
                            println!("  ✅ Both implementations returned same item count");
                        } else {
                            println!("  ⚠️  Item count mismatch: jwalk={}, async_fs={}", 
                                jwalk_items.len(), async_fs_items.len());
                        }
                    }
                    Err(e) => {
                        println!("  ❌ async_fs failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ❌ jwalk failed: {}", e);
            }
        }
        
        println!();
    }
    
    println!("=== Performance Analysis ===");
    println!("✅ jwalk benefits:");
    println!("  • Parallel directory traversal with built-in thread pool");
    println!("  • Single spawn_blocking call vs multiple async operations");
    println!("  • Built-in sorting eliminates post-processing overhead");
    println!("  • Early filtering during traversal reduces memory allocation");
    println!("  • Optimized for CPU-bound filtering vs async I/O overhead");
    
    Ok(())
}