# MoonZoon NovyUI Component Library

A comprehensive, type-safe UI component library for MoonZoon applications. This library provides a complete design system with tokens and components that match modern UI/UX patterns.

## Features

- **Complete Design System**: Comprehensive token system for colors, spacing, typography, etc.
- **Type Safety**: Full Rust type safety with compile-time guarantees
- **Accessibility**: Built-in accessibility features and ARIA support
- **Theming**: Light/dark theme support with smooth transitions
- **Responsive**: Mobile-first responsive design patterns
- **Performance**: Optimized for fast rendering and minimal bundle size

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
moonzoon-novyui = { path = "../path/to/moonzoon-novyui" }
zoon = { git = "https://github.com/MoonZoon/MoonZoon", rev = "4428421b26fbb8b62670c44d911c9ba4c7f0c11b" }
```

Basic usage:

```rust
use moonzoon_novyui::*;
use zoon::*;

fn my_component() -> impl Element {
    Column::new()
        .item(
            button("Click me!")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Medium)
                .on_press(|| {
                    // Handle click
                })
                .build()
        )
        .item(
            badge("New")
                .variant(BadgeVariant::Success)
                .build()
        )
}
```

## Components

### Button
Interactive buttons with multiple variants, sizes, states, and icon support.

```rust
button("Save")
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Medium)
    .left_icon("check")
    .on_press(|| { /* handle click */ })
    .build()
```

### Badge
Status indicators with multiple variants and removable functionality.

```rust
badge("New Feature")
    .variant(BadgeVariant::Success)
    .left_icon("star")
    .removable()
    .on_remove(|| { /* handle remove */ })
    .build()
```

### Input
Text inputs with validation, icons, and error states.

```rust
input()
    .placeholder("Enter your email")
    .input_type(InputType::Email)
    .left_icon("mail")
    .error_message("Please enter a valid email")
    .build()
```

### Switch
Toggle switches with icons and accessibility support.

```rust
switch()
    .checked(true)
    .size(SwitchSize::Medium)
    .checked_icon("check")
    .unchecked_icon("x")
    .on_change(|checked| { /* handle change */ })
    .build()
```

### And More...
- **Checkbox**: Checkboxes with indeterminate state support
- **Select**: Dropdown selects with search and multi-selection
- **Typography**: Headings, paragraphs, and text utilities
- **Icon**: Comprehensive icon system with multiple sizes and colors
- **TreeView**: Hierarchical data display with expand/collapse
- **Kbd**: Keyboard shortcut display components
- **Card, List, Avatar, Textarea, Accordion, Alert, FileInput**

## Design Tokens

The library includes a complete design token system:

- **Colors**: Semantic color scales with light/dark theme support
- **Spacing**: Consistent spacing scale from 4px to 150px
- **Typography**: Font families, sizes, weights, and line heights
- **Borders**: Border widths and styles
- **Corner Radius**: Consistent border radius values
- **Shadows**: Elevation system with multiple shadow levels
- **Animation**: Smooth transitions and timing functions

## Theming

```rust
// Toggle between light and dark themes
toggle_theme();

// Get current theme
let current_theme = theme().get();

// Use theme-aware colors
let background_color = theme().map(|t| match t {
    Theme::Light => "oklch(99% 0.025 255)",
    Theme::Dark => "oklch(18% 0.035 255)",
});
```

## License

MIT
