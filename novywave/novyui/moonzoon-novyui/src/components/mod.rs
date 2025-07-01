// Component Library for NovyUI MoonZoon Migration
// Research-validated component patterns

pub mod button;
pub mod icon;
pub mod input;
pub mod typography;
pub mod badge;
pub mod card;
pub mod list;
pub mod avatar;
pub mod switch;
pub mod checkbox;
pub mod textarea;
pub mod kbd;
pub mod accordion;
pub mod select;
pub mod alert;
pub mod treeview;
pub mod fileinput;
pub mod pattern;

// Re-export components for easy access
pub use button::*;
pub use icon::{
    IconName, IconSize, IconColor, IconBuilder,
    icon, icon_str, icon_name_from_str,
    chevron_down, chevron_up, chevron_left, chevron_right,
    search, check, x, plus, minus, eye, eye_off,
    user, star, heart, settings, refresh_cw
    // Note: arrow_right is excluded to avoid conflict with kbd::arrow_right
};
pub use input::*;
pub use typography::*;
pub use badge::*;
pub use card::*;
pub use list::*;
pub use avatar::*;
pub use switch::*;
pub use checkbox::*;
pub use textarea::*;
pub use kbd::*;
pub use accordion::*;
pub use select::*;
pub use alert::*;
pub use treeview::*;
pub use fileinput::*;
pub use pattern::*;
