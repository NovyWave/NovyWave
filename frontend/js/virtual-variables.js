// Virtual Variables List Implementation - MoonZoon Module Pattern (Fresh)
console.log('=== FRESH VIRTUAL LIST MODULE LOADING ===');

// Virtual Variables List Implementation
let virtualizer = null;
let container = null;
let isInitialized = false;
let getVariablesCountCallback = null;
let getVariableAtIndexCallback = null;

// Simple virtual core implementation
const VirtualCore = {
  Virtualizer: function(options) {
    console.log('Creating virtualizer with options:', options);
    this.options = options;
    this.scrollTop = 0;
    this.itemHeight = options.estimateSize || 24;
    
    this.measure = function() {
      console.log('Virtualizer measure called');
      this.updateVisibleRange();
    };
    
    this.updateVisibleRange = function() {
      const scrollElement = this.options.getScrollElement();
      let containerHeight = scrollElement.clientHeight;
      
      // Fallback if clientHeight is 0 or NaN
      if (!containerHeight || isNaN(containerHeight)) {
        containerHeight = 400; // Use our fixed height as fallback
        console.log('Using fallback height:', containerHeight);
      }
      
      // Ensure scrollTop is initialized
      if (typeof this.scrollTop === 'undefined' || isNaN(this.scrollTop)) {
        this.scrollTop = scrollElement.scrollTop || 0;
      }
      
      // Ensure itemHeight is valid
      if (typeof this.itemHeight === 'undefined' || isNaN(this.itemHeight) || this.itemHeight <= 0) {
        this.itemHeight = 24;
      }
      
      const startIndex = Math.floor(this.scrollTop / this.itemHeight);
      const endIndex = Math.min(
        this.options.count,
        Math.ceil((this.scrollTop + containerHeight) / this.itemHeight) + 5
      );
      
      console.log('ScrollTop:', this.scrollTop, 'ItemHeight:', this.itemHeight, 'Container height:', containerHeight, 'Range:', startIndex, 'to', endIndex);
      
      if (this.options.renderRange) {
        this.options.renderRange({ start: startIndex, end: endIndex });
      }
    };
    
    this.getTotalSize = function() {
      return this.options.count * this.itemHeight;
    };
    
    this.setOptions = function(newOptions) {
      this.options = { ...this.options, ...newOptions };
    };
    
    // Set up scroll listener - let browser handle native wheel scrolling
    if (this.options.getScrollElement) {
      const scrollElement = this.options.getScrollElement();
      
      // Handle scroll events only
      scrollElement.addEventListener('scroll', () => {
        this.scrollTop = scrollElement.scrollTop;
        this.updateVisibleRange();
      });
    }
  }
};

// Initialize the virtual list - called from WASM
export function initializeVirtualList(getCountCallback, getVariableCallback) {
  console.log('=== INITIALIZING FRESH VIRTUAL LIST ===');
  
  try {
    // Store callbacks for data access
    getVariablesCountCallback = getCountCallback;
    getVariableAtIndexCallback = getVariableCallback;
    
    container = document.getElementById('virtual-variables-container');
    if (!container) {
      console.error('Virtual variables container not found!');
      return false;
    }
    
    console.log('Container found:', container);
    
    // Reset initialization state
    isInitialized = false;
    virtualizer = null;

    // Set up container styles with proper height and native scrolling
    container.style.height = '400px'; // Fixed height for testing
    container.style.width = '100%';
    container.style.overflow = 'auto';
    container.style.position = 'relative';
    container.style.webkitOverflowScrolling = 'touch'; // Better mobile scrolling
    
    console.log('Container styled, height:', container.style.height);

    // Create virtualizer instance
    const count = getVariablesCount();
    console.log('Creating virtualizer with count:', count);
    
    virtualizer = new VirtualCore.Virtualizer({
      count: count,
      getScrollElement: () => container,
      estimateSize: () => 24,
      renderRange: (range) => {
        console.log('Rendering range:', range.start, 'to', range.end);
        renderVariables(range.start, range.end);
      }
    });

    // Start the virtualizer
    virtualizer.measure();
    isInitialized = true;
    console.log('=== VIRTUAL LIST INITIALIZED SUCCESSFULLY ===');
    return true;
    
  } catch (error) {
    console.error('Failed to initialize virtualizer:', error);
    return false;
  }
}

// Get the total number of variables using callback
function getVariablesCount() {
  try {
    if (getVariablesCountCallback) {
      const count = getVariablesCountCallback();
      console.log('getVariablesCount returned:', count);
      return count;
    } else {
      console.warn('getVariablesCount callback not available');
      return 0;
    }
  } catch (error) {
    console.error('Error getting variables count:', error);
    return 0;
  }
}

// Get variable data at specific index using callback
function getVariableAtIndex(index) {
  try {
    if (getVariableAtIndexCallback) {
      return getVariableAtIndexCallback(index);
    } else {
      console.warn('getVariableAtIndex callback not available');
      return null;
    }
  } catch (error) {
    console.error('Error getting variable at index:', error);
    return null;
  }
}

// Render variables in the given range
function renderVariables(startIndex, endIndex) {
  if (!container || !virtualizer) return;

  console.log('Rendering variables', startIndex, 'to', endIndex);

  // Clear existing content
  container.innerHTML = '';
  
  // Create wrapper for proper scrolling with correct total height
  const wrapper = document.createElement('div');
  const totalHeight = virtualizer.getTotalSize();
  wrapper.style.position = 'relative';
  wrapper.style.height = totalHeight + 'px';
  wrapper.style.width = '100%';
  wrapper.style.minHeight = totalHeight + 'px'; // Ensure minimum height
  
  console.log('Wrapper height:', wrapper.style.height, 'Total items:', virtualizer.options.count);

  // Create virtual items with proper positioning
  for (let i = startIndex; i < endIndex; i++) {
    const variable = getVariableAtIndex(i);
    if (!variable) {
      console.warn('No variable data at index', i);
      continue;
    }

    const itemElement = createVariableElement(variable, i);
    wrapper.appendChild(itemElement);
  }
  
  container.appendChild(wrapper);
  console.log('Rendered', (endIndex - startIndex), 'variables in wrapper, total height:', totalHeight);
}

// Create a DOM element for a single variable
function createVariableElement(variable, index) {
  const element = document.createElement('div');
  element.className = 'variable-item';
  element.style.cssText = 
    'position: absolute;' +
    'top: ' + (index * 24) + 'px;' +
    'left: 0;' +
    'right: 0;' +
    'height: 24px;' +
    'padding: 2px 8px;' +
    'display: flex;' +
    'align-items: center;' +
    'border-bottom: 1px solid rgba(255, 255, 255, 0.1);' +
    'cursor: pointer;' +
    'transition: background-color 0.1s;';

  // Add hover effect
  element.addEventListener('mouseenter', function() {
    element.style.backgroundColor = 'rgba(255, 255, 255, 0.05)';
  });
  element.addEventListener('mouseleave', function() {
    element.style.backgroundColor = 'transparent';
  });

  // Create variable name element
  const nameElement = document.createElement('span');
  nameElement.textContent = variable.name || ('Variable ' + index);
  nameElement.style.cssText = 
    'color: #e0e0e0;' +
    'font-family: "SF Pro Text", -apple-system, BlinkMacSystemFont, sans-serif;' +
    'font-size: 13px;' +
    'font-weight: 400;' +
    'white-space: nowrap;' +
    'overflow: hidden;' +
    'text-overflow: ellipsis;' +
    'flex: 1;';

  element.appendChild(nameElement);

  // Add click handler for variable selection
  element.addEventListener('click', function() {
    console.log('Selected variable:', variable.name || index);
  });

  return element;
}

// Refresh the virtual list when data changes
export function refreshVirtualList() {
  console.log('=== REFRESHING VIRTUAL LIST ===');
  
  if (!isInitialized || !virtualizer) {
    console.log('Virtual list not initialized, cannot refresh');
    return false;
  }

  const newCount = getVariablesCount();
  console.log('Refreshing virtual list with', newCount, 'variables');
  
  virtualizer.setOptions({
    count: newCount
  });
  
  virtualizer.measure();
  return true;
}

console.log('=== FRESH VIRTUAL VARIABLES MODULE LOADED ===');