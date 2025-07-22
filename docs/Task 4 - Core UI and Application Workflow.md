# Task 4: Core UI and Application Workflow

## Demo

MP4 videos will be uploaded directly to GitHub

## Main Screenshot

![Dark Theme Interface](Task%204%20-%20media/app_dark.png)

## Milestones

### 4a. Implement file opening and loading functionality
![File Selection Dialog](Task%204%20-%20media/select_waveform_files_dialog.png)
- Directory browser with file filtering
- Multi-select with selected file badges at bottom
- Error handling for inaccessible folders
- Special case for empty folder (folders without subfolders and without any supported files)
- Expanded directories and scroll position is remembered

### 4b. Develop a file & scope browser
- See Files & Scopes panel in Main Screenshot above
- Error handling - corrupted/incomplete files, files not found
- Parallel loading/parsing
- Multi-file support
- Only one scope can be selected (may change in the future)
- Opened files, expanded directories, and selected scope is remembered 

### 4c. Create a variables browser
- See Variables panel in Main Screenshot above
- Variables panel shows number of (filtered) variables in the selected scope
- Simple text filter
- Variable labels are visually grouped by making shared prefix contrast
- Type displayed for each variable
- Search term is remembered

### 4d. Build interactive control panels 
- See all three panels in Main Screenshot above
- All panel headers are fully implemented, and all of them are resizable and responsive

### 4e. Implement support for both vertical and horizontal layouts
![Docked to Bottom Layout](Task%204%20-%20media/docked_to_bottom.png)
- See Main Screenshot above to see _Docked to Right_ layout
- Layout switch is on the Selected Variables panel header
- Panels are resized by dragging bars that divide them
- Panel sizes are remembered for both layout modes

### 4f. Enable basic state saving and theme switching
![Light Theme Interface](Task%204%20-%20media/app_light.png)
- Theme switch is on the Selected Variables panel header
- Theme is remembered
- All things noted as _remembered_ are stored in a configuration file (currently in the `.novywave` file in the project root)

### 4g. Basic error handling
![Error Handling System](Task%204%20-%20media/errors.png)
- Errors displayed both in app as toasts and in a console
- Toast auto-dismiss timeout may be paused by clicking on the given toast


## Notes
- Selected Variables panel content is a placeholder, it will be fully implemented in another Task
- Tested mostly in Linux browsers, will be prepared for other platforms in another Task

