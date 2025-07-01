// Icon Component
// Proper SVG implementation matching Vue Storybook version

use crate::tokens::*;
use zoon::*;
use futures_signals::signal::{Signal, SignalExt};
// Removed unused HashMap import since we use include_str! for inline SVG

// Typed icon names matching Vue Storybook exactly
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconName {
    ArrowLeft,
    UserRound,
    Search,
    Check,
    X,
    Plus,
    Minus,
    Eye,
    EyeOff,
    Pencil,
    Trash,
    Info,
    TriangleAlert,
    CircleAlert,
    CircleCheck,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    Menu,
    EllipsisVertical,
    Ellipsis,
    Calendar,
    Clock,
    Upload,
    Download,
    Funnel,
    Settings,
    Star,
    Heart,
    Lock,
    LockOpen,
    RefreshCcw,
    RefreshCw,
    ExternalLink,
    Copy,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    House,
    File,
    Folder,
    Image,
    CloudUpload,
    CloudDownload,
    Send,
    MessageCircle,
    Phone,
    Mail,
    ZoomIn,
    ZoomOut,
    User,
    Users,
    Settings2,
    LogIn,
    LogOut,
    Shield,
    ShieldOff,
    CircleHelp,
    OctagonAlert,
    Bookmark,
    Tag,
    Bell,
    BellOff,
    CalendarCheck,
    CalendarX,
    CalendarPlus,
    CalendarMinus,
    ChevronsUp,
    ChevronsDown,
    ChevronsLeft,
    ChevronsRight,
    Hash,
    Asterisk,
    Moon,
    Sun,
}

impl IconName {
    pub fn to_kebab_case(self) -> &'static str {
        match self {
            IconName::ArrowLeft => "arrow-left",
            IconName::UserRound => "user-round",
            IconName::Search => "search",
            IconName::Check => "check",
            IconName::X => "x",
            IconName::Plus => "plus",
            IconName::Minus => "minus",
            IconName::Eye => "eye",
            IconName::EyeOff => "eye-off",
            IconName::Pencil => "pencil",
            IconName::Trash => "trash",
            IconName::Info => "info",
            IconName::TriangleAlert => "triangle-alert",
            IconName::CircleAlert => "circle-alert",
            IconName::CircleCheck => "circle-check",
            IconName::ChevronDown => "chevron-down",
            IconName::ChevronUp => "chevron-up",
            IconName::ChevronLeft => "chevron-left",
            IconName::ChevronRight => "chevron-right",
            IconName::Menu => "menu",
            IconName::EllipsisVertical => "ellipsis-vertical",
            IconName::Ellipsis => "ellipsis",
            IconName::Calendar => "calendar",
            IconName::Clock => "clock",
            IconName::Upload => "upload",
            IconName::Download => "download",
            IconName::Funnel => "funnel",
            IconName::Settings => "settings",
            IconName::Star => "star",
            IconName::Heart => "heart",
            IconName::Lock => "lock",
            IconName::LockOpen => "lock-open",
            IconName::RefreshCcw => "refresh-ccw",
            IconName::RefreshCw => "refresh-cw",
            IconName::ExternalLink => "external-link",
            IconName::Copy => "copy",
            IconName::ArrowRight => "arrow-right",
            IconName::ArrowUp => "arrow-up",
            IconName::ArrowDown => "arrow-down",
            IconName::House => "house",
            IconName::File => "file",
            IconName::Folder => "folder",
            IconName::Image => "image",
            IconName::CloudUpload => "cloud-upload",
            IconName::CloudDownload => "cloud-download",
            IconName::Send => "send",
            IconName::MessageCircle => "message-circle",
            IconName::Phone => "phone",
            IconName::Mail => "mail",
            IconName::ZoomIn => "zoom-in",
            IconName::ZoomOut => "zoom-out",
            IconName::User => "user",
            IconName::Users => "users",
            IconName::Settings2 => "settings-2",
            IconName::LogIn => "log-in",
            IconName::LogOut => "log-out",
            IconName::Shield => "shield",
            IconName::ShieldOff => "shield-off",
            IconName::CircleHelp => "circle-help",
            IconName::OctagonAlert => "octagon-alert",
            IconName::Bookmark => "bookmark",
            IconName::Tag => "tag",
            IconName::Bell => "bell",
            IconName::BellOff => "bell-off",
            IconName::CalendarCheck => "calendar-check",
            IconName::CalendarX => "calendar-x",
            IconName::CalendarPlus => "calendar-plus",
            IconName::CalendarMinus => "calendar-minus",
            IconName::ChevronsUp => "chevrons-up",
            IconName::ChevronsDown => "chevrons-down",
            IconName::ChevronsLeft => "chevrons-left",
            IconName::ChevronsRight => "chevrons-right",
            IconName::Hash => "hash",
            IconName::Asterisk => "asterisk",
            IconName::Moon => "moon",
            IconName::Sun => "sun",
        }
    }

    pub fn get_url(self) -> String {
        format!("/icons/{}.svg", self.to_kebab_case())
    }
}



// Icon size variants matching design system
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconSize {
    Small,   // 16px
    Medium,  // 20px
    Large,   // 24px
    XLarge,  // 32px
}

impl IconSize {
    pub fn to_px(self) -> u32 {
        match self {
            IconSize::Small => 16,
            IconSize::Medium => 20,
            IconSize::Large => 24,
            IconSize::XLarge => 32,
        }
    }
}

// Icon color variants with theme support
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconColor {
    Current,    // Inherit from parent (default)
    Primary,    // Primary theme color
    Secondary,  // Secondary theme color
    Muted,      // Muted text color
    Success,    // Success color
    Error,      // Error color
    Custom(&'static str), // Custom color value
}

// Icon builder for fluent API
pub struct IconBuilder {
    name: IconName,
    size: IconSize,
    color: IconColor,
    aria_label: Option<String>,
}

impl IconBuilder {
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: IconSize::Medium,
            color: IconColor::Secondary,  // Use Secondary instead of Current for better visibility
            aria_label: None,
        }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: IconColor) -> Self {
        self.color = color;
        self
    }

    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    pub fn build(self) -> impl Element {
        let size_px = self.size.to_px();

        // Get color signal based on theme and color variant - improved contrast for dark theme
        let color = self.color;
        let color_signal = theme().map(move |t| match (color, t) {
            (IconColor::Current, _) => "currentColor",
            (IconColor::Primary, Theme::Light) => "oklch(55% 0.16 250)",
            (IconColor::Primary, Theme::Dark) => "oklch(75% 0.16 250)",  // Brighter for dark theme
            (IconColor::Secondary, Theme::Light) => "oklch(45% 0.05 255)",
            (IconColor::Secondary, Theme::Dark) => "oklch(75% 0.05 255)",  // Much brighter for dark theme
            (IconColor::Muted, Theme::Light) => "oklch(60% 0.02 255)",
            (IconColor::Muted, Theme::Dark) => "oklch(65% 0.02 255)",  // Brighter for dark theme
            (IconColor::Success, Theme::Light) => "oklch(55% 0.16 140)",
            (IconColor::Success, Theme::Dark) => "oklch(70% 0.16 140)",  // Brighter for dark theme
            (IconColor::Error, Theme::Light) => "oklch(55% 0.16 15)",
            (IconColor::Error, Theme::Dark) => "oklch(70% 0.16 15)",  // Brighter for dark theme
            (IconColor::Custom(color), _) => color,
        });

        // Create SVG icon element using proper SVG loading
        let svg_element = create_svg_icon(self.name, color_signal, size_px);

        // Wrap in container with proper accessibility and sizing
        El::new()
            .s(Width::exact(size_px))
            .s(Height::exact(size_px))
            .s(Align::center())
            .child(svg_element)
    }
}

// SVG icon creation function with proper inline SVG using include_str! macro
fn create_svg_icon(name: IconName, color_signal: impl Signal<Item = &'static str> + Unpin + 'static, size_px: u32) -> impl Element {
    // For stroke-based SVG icons, we need to use inline SVG with currentColor
    El::new()
        .s(Width::exact(size_px))
        .s(Height::exact(size_px))
        .s(Align::center())
        .child_signal(
            color_signal.map(move |color| {
                // Get inline SVG content with proper stroke color
                let svg_content = get_svg_content(name, size_px);

                RawHtmlEl::new("div")
                    .style("color", color)
                    .inner_markup(&svg_content)
                    .into_element()
            })
        )
}

// Get inline SVG content using include_str! macro for all 87 icons
fn get_svg_content(name: IconName, size_px: u32) -> String {
    let svg_template = match name {
        IconName::ArrowLeft => include_str!("../../assets/icons/arrow-left.svg"),
        IconName::UserRound => include_str!("../../assets/icons/user-round.svg"),
        IconName::Search => include_str!("../../assets/icons/search.svg"),
        IconName::Check => include_str!("../../assets/icons/check.svg"),
        IconName::X => include_str!("../../assets/icons/x.svg"),
        IconName::Plus => include_str!("../../assets/icons/plus.svg"),
        IconName::Minus => include_str!("../../assets/icons/minus.svg"),
        IconName::Eye => include_str!("../../assets/icons/eye.svg"),
        IconName::EyeOff => include_str!("../../assets/icons/eye-off.svg"),
        IconName::Pencil => include_str!("../../assets/icons/pencil.svg"),
        IconName::Trash => include_str!("../../assets/icons/trash.svg"),
        IconName::Info => include_str!("../../assets/icons/info.svg"),
        IconName::TriangleAlert => include_str!("../../assets/icons/triangle-alert.svg"),
        IconName::CircleAlert => include_str!("../../assets/icons/circle-alert.svg"),
        IconName::CircleCheck => include_str!("../../assets/icons/circle-check.svg"),
        IconName::ChevronDown => include_str!("../../assets/icons/chevron-down.svg"),
        IconName::ChevronUp => include_str!("../../assets/icons/chevron-up.svg"),
        IconName::ChevronLeft => include_str!("../../assets/icons/chevron-left.svg"),
        IconName::ChevronRight => include_str!("../../assets/icons/chevron-right.svg"),
        IconName::Menu => include_str!("../../assets/icons/menu.svg"),
        IconName::EllipsisVertical => include_str!("../../assets/icons/ellipsis-vertical.svg"),
        IconName::Ellipsis => include_str!("../../assets/icons/ellipsis.svg"),
        IconName::Calendar => include_str!("../../assets/icons/calendar.svg"),
        IconName::Clock => include_str!("../../assets/icons/clock.svg"),
        IconName::Upload => include_str!("../../assets/icons/upload.svg"),
        IconName::Download => include_str!("../../assets/icons/download.svg"),
        IconName::Funnel => include_str!("../../assets/icons/funnel.svg"),
        IconName::Settings => include_str!("../../assets/icons/settings.svg"),
        IconName::Star => include_str!("../../assets/icons/star.svg"),
        IconName::Heart => include_str!("../../assets/icons/heart.svg"),
        IconName::Lock => include_str!("../../assets/icons/lock.svg"),
        IconName::LockOpen => include_str!("../../assets/icons/lock-open.svg"),
        IconName::RefreshCcw => include_str!("../../assets/icons/refresh-ccw.svg"),
        IconName::RefreshCw => include_str!("../../assets/icons/refresh-cw.svg"),
        IconName::ExternalLink => include_str!("../../assets/icons/external-link.svg"),
        IconName::Copy => include_str!("../../assets/icons/copy.svg"),
        IconName::ArrowRight => include_str!("../../assets/icons/arrow-right.svg"),
        IconName::ArrowUp => include_str!("../../assets/icons/arrow-up.svg"),
        IconName::ArrowDown => include_str!("../../assets/icons/arrow-down.svg"),
        IconName::House => include_str!("../../assets/icons/house.svg"),
        IconName::File => include_str!("../../assets/icons/file.svg"),
        IconName::Folder => include_str!("../../assets/icons/folder.svg"),
        IconName::Image => include_str!("../../assets/icons/image.svg"),
        IconName::CloudUpload => include_str!("../../assets/icons/cloud-upload.svg"),
        IconName::CloudDownload => include_str!("../../assets/icons/cloud-download.svg"),
        IconName::Send => include_str!("../../assets/icons/send.svg"),
        IconName::MessageCircle => include_str!("../../assets/icons/message-circle.svg"),
        IconName::Phone => include_str!("../../assets/icons/phone.svg"),
        IconName::Mail => include_str!("../../assets/icons/mail.svg"),
        IconName::ZoomIn => include_str!("../../assets/icons/zoom-in.svg"),
        IconName::ZoomOut => include_str!("../../assets/icons/zoom-out.svg"),
        IconName::User => include_str!("../../assets/icons/user.svg"),
        IconName::Users => include_str!("../../assets/icons/users.svg"),
        IconName::Settings2 => include_str!("../../assets/icons/settings-2.svg"),
        IconName::LogIn => include_str!("../../assets/icons/log-in.svg"),
        IconName::LogOut => include_str!("../../assets/icons/log-out.svg"),
        IconName::Shield => include_str!("../../assets/icons/shield.svg"),
        IconName::ShieldOff => include_str!("../../assets/icons/shield-off.svg"),
        IconName::CircleHelp => include_str!("../../assets/icons/circle-help.svg"),
        IconName::OctagonAlert => include_str!("../../assets/icons/octagon-alert.svg"),
        IconName::Bookmark => include_str!("../../assets/icons/bookmark.svg"),
        IconName::Tag => include_str!("../../assets/icons/tag.svg"),
        IconName::Bell => include_str!("../../assets/icons/bell.svg"),
        IconName::BellOff => include_str!("../../assets/icons/bell-off.svg"),
        IconName::CalendarCheck => include_str!("../../assets/icons/calendar-check.svg"),
        IconName::CalendarX => include_str!("../../assets/icons/calendar-x.svg"),
        IconName::CalendarPlus => include_str!("../../assets/icons/calendar-plus.svg"),
        IconName::CalendarMinus => include_str!("../../assets/icons/calendar-minus.svg"),
        IconName::ChevronsUp => include_str!("../../assets/icons/chevrons-up.svg"),
        IconName::ChevronsDown => include_str!("../../assets/icons/chevrons-down.svg"),
        IconName::ChevronsLeft => include_str!("../../assets/icons/chevrons-left.svg"),
        IconName::ChevronsRight => include_str!("../../assets/icons/chevrons-right.svg"),
        IconName::Hash => include_str!("../../assets/icons/hash.svg"),
        IconName::Asterisk => include_str!("../../assets/icons/asterisk.svg"),
        IconName::Moon => include_str!("../../assets/icons/moon.svg"),
        IconName::Sun => include_str!("../../assets/icons/sun.svg"),
    };

    // Process the SVG to set proper size and ensure currentColor works
    process_svg_for_size_and_color(svg_template, size_px)
}

// Process SVG content to set proper size while preserving currentColor
fn process_svg_for_size_and_color(svg_content: &str, size_px: u32) -> String {
    let mut processed = svg_content.to_string();

    // Replace width and height attributes with the desired size
    // Lucide icons typically have width="24" height="24"
    processed = processed.replace("width=\"24\"", &format!("width=\"{}\"", size_px));
    processed = processed.replace("height=\"24\"", &format!("height=\"{}\"", size_px));

    // Ensure stroke="currentColor" is preserved (it should already be in the SVG files)
    // This allows the icons to inherit the color from the parent element

    processed
}









// String to IconName conversion for backward compatibility
pub fn icon_name_from_str(name: &str) -> IconName {
    match name {
        "arrow-left" => IconName::ArrowLeft,
        "user-round" => IconName::UserRound,
        "search" => IconName::Search,
        "check" => IconName::Check,
        "x" => IconName::X,
        "plus" => IconName::Plus,
        "minus" => IconName::Minus,
        "eye" => IconName::Eye,
        "eye-off" => IconName::EyeOff,
        "pencil" => IconName::Pencil,
        "trash" => IconName::Trash,
        "info" => IconName::Info,
        "triangle-alert" => IconName::TriangleAlert,
        "circle-alert" => IconName::CircleAlert,
        "circle-check" => IconName::CircleCheck,
        "chevron-down" => IconName::ChevronDown,
        "chevron-up" => IconName::ChevronUp,
        "chevron-left" => IconName::ChevronLeft,
        "chevron-right" => IconName::ChevronRight,
        "menu" => IconName::Menu,
        "ellipsis-vertical" => IconName::EllipsisVertical,
        "ellipsis" => IconName::Ellipsis,
        "calendar" => IconName::Calendar,
        "clock" => IconName::Clock,
        "upload" => IconName::Upload,
        "download" => IconName::Download,
        "funnel" => IconName::Funnel,
        "settings" => IconName::Settings,
        "star" => IconName::Star,
        "heart" => IconName::Heart,
        "lock" => IconName::Lock,
        "lock-open" => IconName::LockOpen,
        "refresh-ccw" => IconName::RefreshCcw,
        "refresh-cw" => IconName::RefreshCw,
        "external-link" => IconName::ExternalLink,
        "copy" => IconName::Copy,
        "arrow-right" => IconName::ArrowRight,
        "arrow-up" => IconName::ArrowUp,
        "arrow-down" => IconName::ArrowDown,
        "house" => IconName::House,
        "file" => IconName::File,
        "folder" => IconName::Folder,
        "image" => IconName::Image,
        "cloud-upload" => IconName::CloudUpload,
        "cloud-download" => IconName::CloudDownload,
        "send" => IconName::Send,
        "message-circle" => IconName::MessageCircle,
        "phone" => IconName::Phone,
        "mail" => IconName::Mail,
        "zoom-in" => IconName::ZoomIn,
        "zoom-out" => IconName::ZoomOut,
        "user" => IconName::User,
        "users" => IconName::Users,
        "settings-2" => IconName::Settings2,
        "log-in" => IconName::LogIn,
        "log-out" => IconName::LogOut,
        "shield" => IconName::Shield,
        "shield-off" => IconName::ShieldOff,
        "circle-help" => IconName::CircleHelp,
        "octagon-alert" => IconName::OctagonAlert,
        "bookmark" => IconName::Bookmark,
        "tag" => IconName::Tag,
        "bell" => IconName::Bell,
        "bell-off" => IconName::BellOff,
        "calendar-check" => IconName::CalendarCheck,
        "calendar-x" => IconName::CalendarX,
        "calendar-plus" => IconName::CalendarPlus,
        "calendar-minus" => IconName::CalendarMinus,
        "chevrons-up" => IconName::ChevronsUp,
        "chevrons-down" => IconName::ChevronsDown,
        "chevrons-left" => IconName::ChevronsLeft,
        "chevrons-right" => IconName::ChevronsRight,
        "hash" => IconName::Hash,
        "asterisk" => IconName::Asterisk,
        "moon" => IconName::Moon,
        "sun" => IconName::Sun,
        _ => IconName::CircleHelp, // Default fallback
    }
}

// Convenience functions
pub fn icon(name: IconName) -> IconBuilder {
    IconBuilder::new(name)
}

// String-based icon function for backward compatibility
pub fn icon_str(name: &str) -> IconBuilder {
    IconBuilder::new(icon_name_from_str(name))
}

// Common icon shortcuts matching Vue Storybook exactly
pub fn chevron_down() -> IconBuilder {
    IconBuilder::new(IconName::ChevronDown)
}

pub fn chevron_up() -> IconBuilder {
    IconBuilder::new(IconName::ChevronUp)
}

pub fn chevron_left() -> IconBuilder {
    IconBuilder::new(IconName::ChevronLeft)
}

pub fn moon() -> IconBuilder {
    IconBuilder::new(IconName::Moon)
}

pub fn sun() -> IconBuilder {
    IconBuilder::new(IconName::Sun)
}

pub fn chevron_right() -> IconBuilder {
    IconBuilder::new(IconName::ChevronRight)
}

pub fn search() -> IconBuilder {
    IconBuilder::new(IconName::Search)
}

pub fn check() -> IconBuilder {
    IconBuilder::new(IconName::Check)
}

pub fn x() -> IconBuilder {
    IconBuilder::new(IconName::X)
}

pub fn plus() -> IconBuilder {
    IconBuilder::new(IconName::Plus)
}

pub fn minus() -> IconBuilder {
    IconBuilder::new(IconName::Minus)
}

pub fn eye() -> IconBuilder {
    IconBuilder::new(IconName::Eye)
}

pub fn eye_off() -> IconBuilder {
    IconBuilder::new(IconName::EyeOff)
}

pub fn user() -> IconBuilder {
    IconBuilder::new(IconName::User)
}

pub fn star() -> IconBuilder {
    IconBuilder::new(IconName::Star)
}

pub fn heart() -> IconBuilder {
    IconBuilder::new(IconName::Heart)
}

pub fn settings() -> IconBuilder {
    IconBuilder::new(IconName::Settings)
}

pub fn refresh_cw() -> IconBuilder {
    IconBuilder::new(IconName::RefreshCw)
}

pub fn arrow_right() -> IconBuilder {
    IconBuilder::new(IconName::ArrowRight)
}
