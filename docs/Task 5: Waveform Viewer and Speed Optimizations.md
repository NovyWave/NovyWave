# Task 5: Waveform Viewer and Speed Optimizations

## Screenshot 

> _Note:_ Included also in README.md

![Dark Theme Interface](Task%205%20-%20media/novywave_dark_linux.png)

## Demo

https://github.com/user-attachments/assets/a54f447c-11a4-479f-97c4-2a78dd0feaf1

## Milestones

- **5a.** Efficiently transfer signal data from the Tauri backend	
- **5b.** Integrate and use the Fast2D library's API	
- **5c.** Develop algorithms for timeline navigation and zoom	
- **5d.** Implement various signal value formatters (binary, hexadecimal, text, etc.)	
- **5e.** Integrate keyboard navigation controls

## Keys & Functionality

- **Z** - moves the zoom center to the start of the timeline. The zoom center is marked by the dashed vertical line that follows mouse cursor.
- **W** - zooms in, respecting the zoom center. It zooms faster while the Shift key is pressed.
- **S** - the same as W but zooms out.
- **R** - resets waveform viewer to show entire timeline (fit-all view), moves timeline cursor (solid vertical line) to the center of that fit-all view and parks the zoom center to 0 (as the Z key does).
- **T** - toggle waveform tooltip visibility. It's a semi-transparent tooltip following mouse cursor and showing variable name, time and value currently under the mouse cursor.
- **A** - pans left, moves faster while the Shift key is pressed.
- **Q** - moves the timeline cursor to the left. It makes the cursor jumps to value transitions while the Shift key is pressed (it means it jumps to block edges that represent contant values in time for indiviual variables).
- **E** - the same as Q but moves/jumps to the right.
- **D** - pans right, moves faster while the Shift key is pressed.

## Value Formatters

1. ASCII ("ASCII")
    - Converts 8-bit binary chunks to ASCII characters
    - Useful for displaying text data in waveforms
2. Binary ("Bin")
    - Raw binary display (1s and 0s)
    - Direct representation of the signal value
3. BinaryWithGroups ("Bins")
    - Binary with 4-bit grouping separated by spaces
    - Example: 1101 0011 1010
    - Improves readability for long binary values
4. Hexadecimal ("Hex") - Default format
    - Converts binary to base-16 representation
5. Octal ("Oct")
    - Converts binary to base-8 representation
    - Less common but useful for certain applications
6. Signed ("Int")
    - Interprets binary as signed
7. Unsigned ("UInt")
    - Interprets binary as unsigned integer

## Implementation Notes

- Dataflow primitives Actor, ActorVec, ActorMap and Relay were introduced to the project (`frontend/src/dataflow`) and they will be moved to MoonZoon repository later. Actor is slim wrapper of Mutable and TaskHandle, it cannot be locked and its dependencies/inputs are set only during creation. Relay is basically a channel used to indirectly mutate Actors.
- Code was refactored to use Actor+Relay architecture and remove globals to prevent locking issues and make data flow easier to follow.
- Fast2D library was optimized. Also we use mostly _feature_ `canvas` to use browser canvas API as a renderer because WebGL/WebGPU through `wgpu` library is too slow in debug mode and there are no noticeable performance differences in release builds.
- Some other minor changes were made - e.g. User can press Escape to remove focus from the variable search input to quickly activate global shortcuts (W, A, S, D.. keys).
