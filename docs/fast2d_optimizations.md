# Fast2D Performance Optimization Guide

## Executive Summary

Fast2D currently exhibits significant performance degradation when using WebGL/WebGPU backends compared to Canvas 2D, particularly on Linux systems with NVIDIA drivers. This document provides a comprehensive analysis of performance bottlenecks and detailed optimization strategies to achieve smooth 60fps rendering.

### Critical Performance Issues Identified

1. **Buffer Recreation Overhead**: Creating new vertex/index buffers every frame instead of reusing
2. **CPU Tessellation Bottleneck**: Lyon tessellation happens on CPU for all shapes every frame
3. **Linux NVIDIA Driver Issues**: WebGL hardware acceleration failures and software fallback
4. **Signal Cascade Overload**: Multiple reactive handlers triggering redundant full redraws
5. **Missing Frame Pacing**: No coordination between animation systems causing stuttering

## Table of Contents

1. [WebGPU Performance Analysis](#webgpu-performance-analysis)
2. [WebGL Performance Analysis](#webgl-performance-analysis)
3. [Linux GPU Driver Issues](#linux-gpu-driver-issues)
4. [Frame Timing Architecture](#frame-timing-architecture)
5. [NovyWave Integration Issues](#novywave-integration-issues)
6. [Optimization Roadmap](#optimization-roadmap)
7. [Testing & Verification](#testing--verification)

## WebGPU Performance Analysis

### Current Implementation Problems

#### 1. Buffer Management Anti-Patterns

**Problem**: Fast2D creates new buffers on every frame
```rust
// Current problematic code pattern
fn render(&mut self) {
    // BAD: Creates new buffer every frame
    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: BufferUsages::VERTEX,
    });
}
```

**Impact**: 
- GPU memory allocation overhead (10-20ms per frame)
- Driver synchronization stalls
- Memory fragmentation over time

**TODO - CRITICAL**:
- [ ] Implement persistent buffer pool with pre-allocated buffers
- [ ] Use `queue.write_buffer()` for updates instead of recreation
- [ ] Add buffer size prediction based on typical scene complexity
- [ ] Implement growth strategy for buffer pool (2x growth when needed)

#### 2. CPU Tessellation Every Frame

**Problem**: Lyon tessellation runs on CPU for all shapes every frame
```rust
// Current inefficient pattern
for shape in shapes {
    // BAD: Tessellates every shape every frame
    let tessellated = tessellate_shape(shape);
    vertices.extend(tessellated.vertices);
}
```

**Impact**:
- 30-50ms CPU time for complex scenes
- Blocks GPU from starting work early
- No caching of unchanged geometry

**TODO - CRITICAL**:
- [ ] Implement geometry caching with dirty tracking
- [ ] Cache tessellated vertices keyed by shape parameters
- [ ] Only re-tessellate when shape properties change
- [ ] Consider GPU-based tessellation for dynamic shapes

#### 3. Present Mode Selection

**Problem**: Using first available present mode without performance consideration
```rust
// Current suboptimal selection
let present_mode = surface_caps.present_modes[0];
```

**Impact**:
- May select Fifo (VSync) when Immediate/Mailbox available
- Adds 1-2 frames of input latency
- Causes stuttering during animations

**TODO - HIGH**:
- [ ] Implement intelligent present mode selection:
  ```rust
  fn select_present_mode(caps: &SurfaceCapabilities) -> PresentMode {
      // Priority: Immediate > Mailbox > Fifo > AutoVsync
      if caps.present_modes.contains(&PresentMode::Immediate) {
          PresentMode::Immediate  // Lowest latency, may tear
      } else if caps.present_modes.contains(&PresentMode::Mailbox) {
          PresentMode::Mailbox    // Low latency, no tearing
      } else if caps.present_modes.contains(&PresentMode::Fifo) {
          PresentMode::Fifo       // VSync, higher latency
      } else {
          caps.present_modes[0]  // Fallback
      }
  }
  ```
- [ ] Add user preference for VSync vs performance
- [ ] Different modes for resize vs normal rendering

#### 4. Missing Render Optimizations

**TODO - HIGH**:
- [ ] Implement instanced rendering for repeated elements
- [ ] Add frustum culling to skip off-screen objects
- [ ] Implement level-of-detail (LOD) for complex waveforms
- [ ] Add dirty region tracking to avoid full redraws
- [ ] Batch draw calls by render state (same pipeline/texture)
- [ ] Use indirect drawing for dynamic object counts

#### 5. Pipeline State Management

**TODO - MEDIUM**:
- [ ] Cache pipeline states to avoid recreation
- [ ] Use pipeline derivatives for faster switching
- [ ] Implement specialized pipelines for different shape types
- [ ] Add async shader compilation for smooth startup

## WebGL Performance Analysis

### WebGL-Specific Limitations

#### 1. Downlevel Limits

**Problem**: WebGL uses restrictive limits compared to WebGPU
```rust
#[cfg(feature = "webgl")]
required_limits: wgpu::Limits::downlevel_webgl2_defaults()
```

**Impact**:
- Smaller uniform buffer sizes (64KB vs 128KB)
- Limited texture dimensions (8192 vs 32768)
- Fewer bind groups (4 vs 8)

**TODO - HIGH**:
- [ ] Implement uniform buffer paging for large data
- [ ] Add texture atlasing to work within size limits
- [ ] Optimize bind group usage for WebGL constraints

#### 2. Missing WebGL Extensions

**Problem**: Not utilizing performance-critical WebGL extensions

**TODO - HIGH**:
- [ ] Query and enable these extensions:
  - [ ] `ANGLE_instanced_arrays` - For instanced rendering
  - [ ] `OES_vertex_array_object` - VAO support
  - [ ] `EXT_disjoint_timer_query` - GPU profiling
  - [ ] `WEBGL_lose_context` - Better context recovery
  - [ ] `EXT_texture_filter_anisotropic` - Better texture quality
  - [ ] `OES_element_index_uint` - 32-bit indices

#### 3. Shader Compatibility

**Problem**: Manual sRGB conversion adds overhead
```rust
// WebGL requires manual conversion
let srgb = pow(linear_rgb, vec3(1.0/2.2));
```

**TODO - MEDIUM**:
- [ ] Optimize sRGB conversion with lookup tables
- [ ] Use approximations where quality permits
- [ ] Consider pre-converted color values

#### 4. Buffer Upload Strategies

**TODO - HIGH**:
- [ ] Use `bufferSubData` for partial updates
- [ ] Implement double buffering to avoid stalls
- [ ] Add buffer orphaning for better pipelining
- [ ] Use pixel buffer objects for texture uploads

## Linux GPU Driver Issues

### NVIDIA-Specific Problems

#### 1. Hardware Acceleration Failures

**Problem**: WebGL falls back to software rendering on Linux+NVIDIA
- Particularly severe with WebKitGTK backend
- Affects both X11 and Wayland sessions
- Driver versions 390.25+ show regressions

**Symptoms**:
```javascript
// Chrome://gpu shows:
WebGL: Software only, hardware acceleration unavailable
WebGL2: Software only, hardware acceleration unavailable
WebGPU: Disabled
```

**TODO - CRITICAL**:
- [ ] Implement GPU capability detection:
  ```rust
  fn detect_gpu_capabilities() -> GpuCapabilities {
      // Check for software rendering
      let renderer = gl.get_parameter(gl.RENDERER);
      let is_software = renderer.includes("SwiftShader") || 
                        renderer.includes("llvmpipe") ||
                        renderer.includes("Software");
      
      // Check for ANGLE/Mesa
      let is_angle = renderer.includes("ANGLE");
      let is_mesa = renderer.includes("Mesa");
      
      // Detect NVIDIA driver version
      let vendor = gl.get_parameter(gl.VENDOR);
      let is_nvidia = vendor.includes("NVIDIA");
      
      GpuCapabilities {
          hardware_accelerated: !is_software,
          vendor_type: detect_vendor_type(vendor),
          supports_webgpu: check_webgpu_support(),
          max_texture_size: gl.get_parameter(gl.MAX_TEXTURE_SIZE),
      }
  }
  ```

- [ ] Add automatic fallback strategies:
  - [ ] Reduce MSAA samples (4x → 2x → disabled)
  - [ ] Disable complex shaders
  - [ ] Switch to Canvas 2D for software rendering
  - [ ] Reduce resolution for better performance

#### 2. WebKitGTK Backend Issues

**Problem**: WebKitGTK breaks WebGL on Linux+NVIDIA systems

**TODO - HIGH**:
- [ ] Detect WebKitGTK environment
- [ ] Implement CEF backend as alternative
- [ ] Add Servo experimental backend support
- [ ] Document Tauri limitations and workarounds

#### 3. Wayland vs X11 Differences

**TODO - MEDIUM**:
- [ ] Detect display server (Wayland/X11)
- [ ] Apply server-specific optimizations:
  - [ ] X11: Enable `__GL_YIELD="USLEEP"` for better scheduling
  - [ ] Wayland: Use EGL instead of GLX
  - [ ] Both: Set `__GL_THREADED_OPTIMIZATIONS=1`

#### 4. Power Management Issues

**Problem**: Laptop GPUs throttle or switch to integrated graphics

**TODO - MEDIUM**:
- [ ] Detect Optimus/PRIME configurations
- [ ] Force discrete GPU usage:
  ```bash
  export __NV_PRIME_RENDER_OFFLOAD=1
  export __GLX_VENDOR_LIBRARY_NAME=nvidia
  ```
- [ ] Monitor GPU clock speeds and throttling
- [ ] Adjust quality based on power state

### Mesa Driver Considerations

**TODO - MEDIUM**:
- [ ] Detect Mesa version and capabilities
- [ ] Enable Mesa-specific optimizations:
  - [ ] `MESA_GL_VERSION_OVERRIDE=4.5` for newer features
  - [ ] `MESA_GLSL_VERSION_OVERRIDE=450` for better shaders
  - [ ] `mesa_glthread=true` for multithreaded GL

## Frame Timing Architecture

### Current Event-Driven Problems

#### 1. Signal Cascade Overload

**Problem**: 8+ reactive signals all trigger full canvas redraws
```rust
// Current problematic pattern in NovyWave
Task::start(SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(|_| {
    canvas.clear();
    canvas.update_objects(create_all_objects()); // Full recreation
}));

Task::start(TIMELINE_ZOOM.signal().for_each_sync(|_| {
    canvas.clear();
    canvas.update_objects(create_all_objects()); // Duplicate work
}));
// ... 6 more similar handlers
```

**Impact**:
- Multiple redraws per frame (3-5x overdraw)
- Signal race conditions
- Unpredictable frame timing

**TODO - CRITICAL**:
- [ ] Implement unified render coordinator:
  ```rust
  struct RenderCoordinator {
      pending_updates: HashSet<UpdateType>,
      last_frame_time: Instant,
      target_fps: f32,
  }
  
  impl RenderCoordinator {
      fn request_update(&mut self, update_type: UpdateType) {
          self.pending_updates.insert(update_type);
          self.schedule_frame();
      }
      
      fn schedule_frame(&mut self) {
          // Batch updates within frame budget
          if self.pending_updates.is_empty() { return; }
          
          let elapsed = self.last_frame_time.elapsed();
          let frame_budget = Duration::from_secs_f32(1.0 / self.target_fps);
          
          if elapsed >= frame_budget {
              self.render_frame();
          } else {
              // Schedule for next frame
              Timer::sleep(frame_budget - elapsed);
          }
      }
  }
  ```

- [ ] Consolidate signal handlers into categories:
  - [ ] Data updates (variables, scopes)
  - [ ] View updates (zoom, pan, timeline)
  - [ ] UI updates (theme, resize)

#### 2. Animation Conflicts

**Problem**: Multiple 60fps timers without coordination
```rust
// Current pattern causing conflicts
Timer::interval(16, || update_zoom());    // 60fps zoom
Timer::interval(16, || update_pan());     // 60fps pan
Timer::interval(16, || update_cursor());  // 60fps cursor
```

**TODO - HIGH**:
- [ ] Implement single animation loop:
  ```rust
  struct AnimationScheduler {
      animations: Vec<Box<dyn Animation>>,
      frame_time: Duration,
  }
  
  impl AnimationScheduler {
      fn run_frame(&mut self, dt: Duration) {
          for animation in &mut self.animations {
              animation.update(dt);
          }
          self.render_once(); // Single render per frame
      }
  }
  ```

- [ ] Add interpolation for smooth animations
- [ ] Implement easing functions
- [ ] Add animation priorities and cancellation

#### 3. Object Recreation Overhead

**Problem**: Full object list recreation on every change
```rust
// Current inefficient pattern
*objects = create_waveform_objects_with_dimensions_and_theme(
    &selected_vars, 
    canvas_width, 
    canvas_height, 
    &novyui_theme, 
    cursor_pos
); // Recreates everything
```

**TODO - CRITICAL**:
- [ ] Implement incremental updates:
  ```rust
  enum ObjectUpdate {
      Add(ObjectId, Object),
      Remove(ObjectId),
      Modify(ObjectId, ObjectPatch),
      Move(ObjectId, Position),
  }
  
  fn apply_updates(objects: &mut Vec<Object>, updates: Vec<ObjectUpdate>) {
      for update in updates {
          match update {
              Add(id, obj) => objects.push(obj),
              Remove(id) => objects.retain(|o| o.id != id),
              Modify(id, patch) => {
                  if let Some(obj) = objects.find(id) {
                      obj.apply_patch(patch);
                  }
              }
              Move(id, pos) => {
                  if let Some(obj) = objects.find(id) {
                      obj.position = pos;
                  }
              }
          }
      }
  }
  ```

- [ ] Cache static objects (grid, labels)
- [ ] Only update dynamic objects (cursor, selection)
- [ ] Use object pooling for frequent create/destroy

## NovyWave Integration Issues

### 1. Canvas Wrapper Inefficiencies

**TODO - HIGH**:
- [ ] Remove intermediate object conversion layers
- [ ] Direct Fast2D API usage without wrappers
- [ ] Batch object updates instead of individual calls
- [ ] Use retained mode for static content

### 2. Zoom/Pan Implementation

**TODO - HIGH**:
- [ ] Implement viewport culling before tessellation
- [ ] Use transform matrices instead of recalculating vertices
- [ ] Cache tessellated geometry at multiple LODs
- [ ] Smooth zoom with exponential interpolation

### 3. Waveform Rendering

**TODO - CRITICAL**:
- [ ] Implement waveform decimation for zoom levels:
  ```rust
  struct WaveformLOD {
      zoom_range: (f32, f32),
      decimation_factor: usize,
      cached_points: Vec<Point>,
  }
  
  fn select_lod(zoom: f32) -> &WaveformLOD {
      // Return appropriate LOD for current zoom
      match zoom {
          0.0..=0.1 => &lod_coarse,    // 1:1000 decimation
          0.1..=1.0 => &lod_medium,    // 1:100 decimation  
          1.0..=10.0 => &lod_fine,     // 1:10 decimation
          _ => &lod_full,              // No decimation
      }
  }
  ```

- [ ] Use line strips instead of individual segments
- [ ] Implement binary search for visible range
- [ ] Add adaptive quality based on frame rate

## Optimization Roadmap

### Phase 1: Critical Fixes (Week 1)
**Goal**: Stop frame drops and stuttering

1. **Buffer Pool Implementation**
   - [ ] Design buffer pool architecture
   - [ ] Implement allocation strategy
   - [ ] Convert to `write_buffer` updates
   - [ ] Add metrics and monitoring
   - **Expected Impact**: 20-30% performance gain

2. **Geometry Caching**
   - [ ] Design cache key system
   - [ ] Implement LRU cache for tessellation
   - [ ] Add dirty tracking
   - [ ] Measure cache hit rates
   - **Expected Impact**: 40-50% CPU reduction

3. **Signal Consolidation**
   - [ ] Audit all signal handlers
   - [ ] Design render coordinator
   - [ ] Implement batching system
   - [ ] Remove redundant updates
   - **Expected Impact**: 3-5x reduction in redraws

### Phase 2: Performance Optimizations (Week 2)

4. **Frame Pacing System**
   - [ ] Implement animation scheduler
   - [ ] Add frame budget management
   - [ ] Integrate with browser RAF
   - [ ] Add frame drop detection
   - **Expected Impact**: Smooth 60fps

5. **Incremental Updates**
   - [ ] Design update protocol
   - [ ] Implement object diffing
   - [ ] Add update batching
   - [ ] Optimize change detection
   - **Expected Impact**: 60-70% reduction in work

6. **GPU Feature Detection**
   - [ ] Implement capability detection
   - [ ] Add automatic quality adjustment
   - [ ] Create fallback render paths
   - [ ] Add performance monitoring
   - **Expected Impact**: Eliminate software rendering

### Phase 3: Advanced Optimizations (Week 3)

7. **Instanced Rendering**
   - [ ] Identify repeated elements
   - [ ] Implement instance buffers
   - [ ] Convert to instanced draw calls
   - [ ] Measure draw call reduction
   - **Expected Impact**: 10x reduction in draw calls

8. **Waveform LOD System**
   - [ ] Design decimation algorithm
   - [ ] Implement LOD generation
   - [ ] Add seamless LOD switching
   - [ ] Cache LOD data
   - **Expected Impact**: 100x reduction for zoomed out

9. **WebGL Extension Usage**
   - [ ] Query available extensions
   - [ ] Implement VAO usage
   - [ ] Add instancing support
   - [ ] Enable timer queries
   - **Expected Impact**: 20-30% WebGL improvement

### Phase 4: Platform-Specific (Week 4)

10. **Linux Driver Workarounds**
    - [ ] Detect driver/GPU configuration
    - [ ] Apply NVIDIA-specific fixes
    - [ ] Add Mesa optimizations
    - [ ] Implement Wayland/X11 paths
    - **Expected Impact**: Fix Linux stuttering

11. **Present Mode Optimization**
    - [ ] Implement smart selection
    - [ ] Add user preferences
    - [ ] Different modes for resize
    - [ ] Measure latency improvement
    - **Expected Impact**: 1-2 frame latency reduction

12. **Memory Management**
    - [ ] Implement resource pooling
    - [ ] Add memory pressure handling
    - [ ] Optimize allocation patterns
    - [ ] Add memory profiling
    - **Expected Impact**: Eliminate memory spikes

## Testing & Verification

### Performance Metrics

#### 1. Frame Time Analysis
```rust
struct FrameMetrics {
    frame_times: VecDeque<Duration>,
    dropped_frames: usize,
    target_fps: f32,
}

impl FrameMetrics {
    fn record_frame(&mut self, duration: Duration) {
        self.frame_times.push_back(duration);
        if self.frame_times.len() > 100 {
            self.frame_times.pop_front();
        }
        
        let target = Duration::from_secs_f32(1.0 / self.target_fps);
        if duration > target * 1.5 {
            self.dropped_frames += 1;
        }
    }
    
    fn percentile(&self, p: f32) -> Duration {
        // Calculate p50, p95, p99 frame times
    }
}
```

#### 2. GPU Profiling
```javascript
// WebGL timer queries
const ext = gl.getExtension('EXT_disjoint_timer_query');
const query = ext.createQueryEXT();
ext.beginQueryEXT(ext.TIME_ELAPSED_EXT, query);
// ... render ...
ext.endQueryEXT(ext.TIME_ELAPSED_EXT);

// Check result
const available = ext.getQueryObjectEXT(query, ext.QUERY_RESULT_AVAILABLE_EXT);
if (available) {
    const timeElapsed = ext.getQueryObjectEXT(query, ext.QUERY_RESULT_EXT);
    console.log(`GPU time: ${timeElapsed / 1000000}ms`);
}
```

#### 3. Memory Profiling
```rust
fn measure_memory_usage() -> MemoryStats {
    MemoryStats {
        heap_allocated: ALLOCATOR.allocated(),
        gpu_memory: estimate_gpu_memory(),
        buffer_pool_size: BUFFER_POOL.total_size(),
        cache_size: GEOMETRY_CACHE.size_bytes(),
    }
}
```

### Test Scenarios

1. **Stress Test**
   - Load 10,000+ signals
   - Rapid zoom in/out
   - Continuous panning
   - Measure sustained frame rate

2. **Memory Test**
   - Long running session (1+ hours)
   - Monitor memory growth
   - Check for leaks
   - Verify cleanup

3. **Platform Test**
   - Test on NVIDIA + Linux
   - Test on AMD + Linux
   - Test on Intel integrated
   - Test on macOS Metal
   - Test on Windows D3D12

4. **Quality Levels**
   - Force software rendering
   - Test each quality preset
   - Verify automatic adjustment
   - Measure performance delta

### Success Criteria

- **Frame Rate**: Consistent 60fps with <1% dropped frames
- **Frame Time**: p95 < 16.67ms, p99 < 20ms
- **Input Latency**: < 1 frame (16.67ms)
- **Memory**: < 500MB for typical session
- **Startup**: < 500ms to first paint
- **Resize**: Smooth without white flashes

## Implementation Notes

### Build Configuration

```toml
# Cargo.toml optimization flags
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"

[profile.release.package.fast2d]
opt-level = 3
debug = false

# Feature flags for testing
[features]
profile = ["fast2d/profile"]
webgl-opt = ["fast2d/webgl", "fast2d/instancing"]
webgpu-opt = ["fast2d/webgpu", "fast2d/indirect-draw"]
```

### Debug Helpers

```rust
// Performance overlay
#[cfg(feature = "profile")]
fn render_debug_overlay(metrics: &FrameMetrics) {
    draw_text(format!("FPS: {:.1}", metrics.current_fps()));
    draw_text(format!("Frame: {:.2}ms", metrics.last_frame_ms()));
    draw_text(format!("Dropped: {}", metrics.dropped_frames));
    draw_text(format!("Draw calls: {}", metrics.draw_call_count));
    draw_text(format!("Triangles: {}", metrics.triangle_count));
}
```

## Conclusion

Fast2D's performance issues stem from fundamental architectural problems that compound when used with reactive frameworks like MoonZoon/Zoon. The combination of buffer recreation overhead, CPU tessellation bottlenecks, missing frame coordination, and Linux driver issues creates severe stuttering.

By implementing the optimizations in this document systematically, we can achieve:
- **10-50x performance improvement** in complex scenes
- **Smooth 60fps** on all platforms including Linux+NVIDIA
- **< 16ms frame times** consistently
- **Minimal memory usage** with proper pooling
- **Low input latency** with optimized present modes

The roadmap prioritizes fixes by impact, with critical buffer/caching improvements first, followed by architectural changes to frame timing, and finally platform-specific optimizations. Each phase builds on the previous, with measurable success criteria to verify improvements.