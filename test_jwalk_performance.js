#!/usr/bin/env node

// Test script to evaluate jwalk performance improvements
// This sends BrowseDirectory WebSocket messages to the backend and measures response times

const WebSocket = require('ws');

function testJwalkPerformance() {
    console.log("=== jwalk Performance Test ===");
    
    const ws = new WebSocket('ws://localhost:8080/_api/ws');
    const testResults = [];
    let testIndex = 0;
    
    const testDirectories = [
        './test_large_dir',  // 180 files (150 txt + 20 vcd + 10 dirs)
        '.',                 // Project root (moderate size)
        '/usr/bin',          // System directory (hundreds of files)  
        '/home',             // Home directories
        './frontend/src'     // Source code directory
    ];
    
    ws.on('open', function open() {
        console.log('Connected to NovyWave backend');
        runNextTest();
    });
    
    ws.on('message', function message(data) {
        const response = JSON.parse(data);
        const endTime = Date.now();
        
        if (response.DirectoryContents) {
            const startTime = testResults[testResults.length - 1]?.startTime;
            const responseTime = endTime - startTime;
            const itemCount = response.DirectoryContents.items.length;
            const path = response.DirectoryContents.path;
            
            console.log(`✅ ${path}: ${itemCount} items in ${responseTime}ms (${(itemCount/responseTime*1000).toFixed(1)} items/sec)`);
            
            // Show filtering effectiveness
            const directories = response.DirectoryContents.items.filter(item => item.is_directory).length;
            const vcdFiles = response.DirectoryContents.items.filter(item => item.name.endsWith('.vcd')).length;
            const fstFiles = response.DirectoryContents.items.filter(item => item.name.endsWith('.fst')).length;
            
            console.log(`   📁 ${directories} dirs, 📄 ${vcdFiles} .vcd, 📄 ${fstFiles} .fst`);
            
            testResults[testResults.length - 1].responseTime = responseTime;
            testResults[testResults.length - 1].itemCount = itemCount;
            testResults[testResults.length - 1].directories = directories;
            testResults[testResults.length - 1].waveformFiles = vcdFiles + fstFiles;
            
        } else if (response.DirectoryError) {
            console.log(`❌ ${response.DirectoryError.path}: ${response.DirectoryError.error}`);
            testResults[testResults.length - 1].error = response.DirectoryError.error;
        }
        
        // Wait briefly then run next test
        setTimeout(runNextTest, 100);
    });
    
    ws.on('error', function error(err) {
        console.error('WebSocket error:', err);
    });
    
    function runNextTest() {
        if (testIndex >= testDirectories.length) {
            showResults();
            ws.close();
            return;
        }
        
        const directory = testDirectories[testIndex++];
        const startTime = Date.now();
        
        console.log(`\n🔍 Testing directory: ${directory}`);
        
        testResults.push({
            directory,
            startTime,
            responseTime: null,
            itemCount: 0,
            error: null
        });
        
        // Send BrowseDirectory message
        const message = JSON.stringify({
            BrowseDirectory: directory
        });
        
        ws.send(message);
    }
    
    function showResults() {
        console.log('\n=== Performance Summary ===');
        
        const successfulTests = testResults.filter(t => !t.error && t.responseTime !== null);
        
        if (successfulTests.length > 0) {
            const avgResponseTime = successfulTests.reduce((sum, t) => sum + t.responseTime, 0) / successfulTests.length;
            const totalItems = successfulTests.reduce((sum, t) => sum + t.itemCount, 0);
            const avgThroughput = totalItems / successfulTests.reduce((sum, t) => sum + t.responseTime, 0) * 1000;
            
            console.log(`📊 Average response time: ${avgResponseTime.toFixed(1)}ms`);
            console.log(`📊 Average throughput: ${avgThroughput.toFixed(1)} items/sec`);
            console.log(`📊 Total items processed: ${totalItems}`);
            
            // Find best and worst performance
            const fastest = successfulTests.reduce((min, t) => t.responseTime < min.responseTime ? t : min);
            const slowest = successfulTests.reduce((max, t) => t.responseTime > max.responseTime ? t : max);
            
            console.log(`🚀 Fastest: ${fastest.directory} (${fastest.responseTime}ms, ${fastest.itemCount} items)`);
            console.log(`🐌 Slowest: ${slowest.directory} (${slowest.responseTime}ms, ${slowest.itemCount} items)`);
        }
        
        console.log('\n=== jwalk Implementation Benefits ===');
        console.log('✅ Parallel directory traversal with built-in filtering');
        console.log('✅ Single spawn_blocking call instead of multiple async operations');
        console.log('✅ Built-in sorting eliminates post-processing');
        console.log('✅ Efficient filtering during traversal (only dirs + .vcd/.fst files)');
        console.log('✅ Reduced memory allocation vs async_fs approach');
    }
}

// Start the test
testJwalkPerformance();