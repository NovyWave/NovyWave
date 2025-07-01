//! # MoonZoon NovyUI Component Library
//!
//! A comprehensive, type-safe UI component library for MoonZoon applications.
//! This library provides a complete design system with tokens and components
//! that match modern UI/UX patterns.
//!
//! ## Features
//!
//! - **Complete Design System**: Comprehensive token system for colors, spacing, typography, etc.
//! - **Type Safety**: Full Rust type safety with compile-time guarantees
//! - **Accessibility**: Built-in accessibility features and ARIA support
//! - **Theming**: Light/dark theme support with smooth transitions
//! - **Responsive**: Mobile-first responsive design patterns
//! - **Performance**: Optimized for fast rendering and minimal bundle size
//!
//! ## Quick Start
//!
//! ```rust
//! use moonzoon_novyui::*;
//! use zoon::*;
//!
//! fn my_component() -> impl Element {
//!     Column::new()
//!         .item(
//!             button("Click me!")
//!                 .variant(ButtonVariant::Primary)
//!                 .size(ButtonSize::Medium)
//!                 .on_press(|| {
//!                     // Handle click
//!                 })
//!                 .build()
//!         )
//!         .item(
//!             badge("New")
//!                 .variant(BadgeVariant::Success)
//!                 .build()
//!         )
//! }
//! ```
//!
//! ## Components
//!
//! - **Button**: Primary, secondary, outline, ghost, link, and destructive variants
//! - **Badge**: Status indicators with multiple variants and removable functionality
//! - **Input**: Text inputs with validation, icons, and error states
//! - **Switch**: Toggle switches with icons and accessibility support
//! - **Checkbox**: Checkboxes with indeterminate state support
//! - **Select**: Dropdown selects with search and multi-selection
//! - **Typography**: Headings, paragraphs, and text utilities
//! - **Icon**: Comprehensive icon system with multiple sizes and colors
//! - **TreeView**: Hierarchical data display with expand/collapse
//! - **Kbd**: Keyboard shortcut display components
//! - **And more**: Card, List, Avatar, Textarea, Accordion, Alert, FileInput
//!
//! ## Design Tokens
//!
//! The library includes a complete design token system:
//!
//! - **Colors**: Semantic color scales with light/dark theme support
//! - **Spacing**: Consistent spacing scale from 4px to 150px
//! - **Typography**: Font families, sizes, weights, and line heights
//! - **Borders**: Border widths and styles
//! - **Corner Radius**: Consistent border radius values
//! - **Shadows**: Elevation system with multiple shadow levels
//! - **Animation**: Smooth transitions and timing functions

pub mod components;
pub mod tokens;

// Re-export all components for easy access
pub use components::*;

// Re-export all tokens for easy access
pub use tokens::*;

// Re-export zoon for convenience
pub use zoon;
