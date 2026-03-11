use tauri::utils::{config::BundleType, platform::bundle_type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateCapability {
    Hidden {
        reason: &'static str,
    },
    AutoUpdate {
        message_template: &'static str,
        action_label: &'static str,
    },
}

impl UpdateCapability {
    pub fn current(test_updater: bool) -> Self {
        let test_updater = test_updater || explicit_updater_test_mode();
        Self::for_bundle(
            tauri::is_dev() || cfg!(debug_assertions),
            test_updater,
            bundle_type(),
        )
    }

    fn for_bundle(
        is_local_build: bool,
        test_updater: bool,
        bundle_type: Option<BundleType>,
    ) -> Self {
        if test_updater {
            return Self::AutoUpdate {
                message_template:
                    "v{current} -> v{new}. Download and install the update, then restart NovyWave.",
                action_label: "Download and install",
            };
        }

        if is_local_build {
            return Self::Hidden {
                reason: "local/dev builds do not show production update banners",
            };
        }

        match bundle_type {
            Some(BundleType::AppImage) => Self::AutoUpdate {
                message_template:
                    "v{current} -> v{new}. Download and install the new AppImage, then restart NovyWave.",
                action_label: "Download and install",
            },
            Some(BundleType::App) => Self::AutoUpdate {
                message_template:
                    "v{current} -> v{new}. Download and install the update, then restart NovyWave.",
                action_label: "Download and install",
            },
            Some(BundleType::Nsis) => Self::AutoUpdate {
                message_template:
                    "v{current} -> v{new}. Download and install the update, then restart NovyWave.",
                action_label: "Download and install",
            },
            Some(BundleType::Deb) => Self::Hidden {
                reason: "deb installs are manual-update only",
            },
            Some(BundleType::Rpm) => Self::Hidden {
                reason: "rpm installs are manual-update only",
            },
            Some(BundleType::Msi) => Self::Hidden {
                reason: "MSI installs are manual-update only",
            },
            None => Self::Hidden {
                reason: "unsupported or unknown bundle type",
            },
        }
    }

    pub fn is_supported(self) -> bool {
        matches!(self, Self::AutoUpdate { .. })
    }

    pub fn hidden_reason(self) -> Option<&'static str> {
        match self {
            Self::Hidden { reason } => Some(reason),
            Self::AutoUpdate { .. } => None,
        }
    }

    pub fn action_label(self) -> Option<&'static str> {
        match self {
            Self::AutoUpdate { action_label, .. } => Some(action_label),
            Self::Hidden { .. } => None,
        }
    }

    pub fn banner_message(self, current_version: &str, new_version: &str) -> Option<String> {
        match self {
            Self::AutoUpdate {
                message_template, ..
            } => Some(
                message_template
                    .replace("{current}", current_version)
                    .replace("{new}", new_version),
            ),
            Self::Hidden { .. } => None,
        }
    }
}

fn explicit_updater_test_mode() -> bool {
    matches!(
        std::env::var("NOVYWAVE_UPDATER_TEST").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

#[cfg(test)]
mod tests {
    use super::UpdateCapability;
    use tauri::utils::config::BundleType;

    #[test]
    fn local_builds_hide_updates() {
        let capability = UpdateCapability::for_bundle(true, false, Some(BundleType::AppImage));
        assert_eq!(
            capability.hidden_reason(),
            Some("local/dev builds do not show production update banners")
        );
    }

    #[test]
    fn updater_test_mode_overrides_bundle_filter() {
        let capability = UpdateCapability::for_bundle(true, true, Some(BundleType::Deb));
        assert!(capability.is_supported());
    }

    #[test]
    fn supported_bundles_keep_auto_update() {
        assert!(
            UpdateCapability::for_bundle(false, false, Some(BundleType::AppImage)).is_supported()
        );
        assert!(UpdateCapability::for_bundle(false, false, Some(BundleType::App)).is_supported());
        assert!(UpdateCapability::for_bundle(false, false, Some(BundleType::Nsis)).is_supported());
    }

    #[test]
    fn manual_install_bundles_hide_updates() {
        assert_eq!(
            UpdateCapability::for_bundle(false, false, Some(BundleType::Deb)).hidden_reason(),
            Some("deb installs are manual-update only")
        );
        assert_eq!(
            UpdateCapability::for_bundle(false, false, Some(BundleType::Rpm)).hidden_reason(),
            Some("rpm installs are manual-update only")
        );
        assert_eq!(
            UpdateCapability::for_bundle(false, false, Some(BundleType::Msi)).hidden_reason(),
            Some("MSI installs are manual-update only")
        );
    }
}
