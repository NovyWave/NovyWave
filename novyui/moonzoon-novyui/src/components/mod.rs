// Component Library for NovyUI MoonZoon Migration
// Research-validated component patterns

pub mod accordion;
pub mod alert;
pub mod avatar;
pub mod badge;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod fileinput;
pub mod icon;
pub mod input;
pub mod kbd;
pub mod list;
pub mod pattern;
pub mod select;
pub mod switch;
pub mod textarea;
pub mod treeview;
pub mod typography;

// Re-export components for easy access
pub use accordion::*;
pub use alert::*;
pub use avatar::*;
pub use badge::*;
pub use button::*;
pub use card::*;
pub use checkbox::*;
pub use fileinput::*;
pub use icon::{
    IconBuilder,
    IconColor,
    IconName,
    IconSize,
    check,
    chevron_down,
    chevron_left,
    chevron_right,
    chevron_up,
    eye,
    eye_off,
    heart,
    icon,
    icon_name_from_str,
    icon_str,
    minus,
    plus,
    refresh_cw, // Note: arrow_right is excluded to avoid conflict with kbd::arrow_right
    search,
    settings,
    star,
    user,
    x,
};
pub use input::*;
pub use kbd::*;
pub use list::*;
pub use pattern::*;
pub use select::*;
pub use switch::*;
pub use textarea::*;
pub use treeview::*;
pub use typography::*;
