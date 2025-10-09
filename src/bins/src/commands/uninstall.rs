use crate::shared::{self};
use velopack::{bundle::load_bundle_from_file, constants, locator::{find_latest_full_package, VelopackLocator}};

use crate::windows;
use anyhow::Result;
use std::fs::File;

pub fn uninstall(locator: &VelopackLocator, delete_self: bool) -> Result<()> {
    info!("Command: Uninstall");

    // Initialize localization strings from the installed package if available
    let packages_dir = locator.get_packages_dir();
    if let Some((package_path, _)) = find_latest_full_package(&packages_dir) {
        if let Ok(bundle) = load_bundle_from_file(&package_path) {
            shared::localization::initialize_from_bundle(&bundle);
            info!("Localization initialized from package: {}", package_path.to_string_lossy());
        } else {
            warn!("Failed to load bundle from package for localization");
        }
    } else {
        warn!("No package found for localization, using defaults");
    }

    let root_path = locator.get_root_dir();

    // the real app could be running at the moment
    let _ = shared::force_stop_package(&root_path);

    // run uninstall hook
    windows::run_hook(&locator, constants::HOOK_CLI_UNINSTALL, 60);

    // remove all shortcuts pointing to the app
    windows::remove_all_shortcuts_for_root_dir(&root_path);

    info!("Removing directory '{}'", root_path.to_string_lossy());
    let _ = remove_dir_all::remove_dir_contents(&root_path);

    if let Err(e) = windows::registry::remove_uninstall_entry(&locator) {
        error!("Unable to remove uninstall registry entry ({}).", e);
    }

    // if it returns true, it was a success.
    // if it returns false, it was completed with errors which the user should be notified of.
    let app_title = locator.get_manifest_title();

    info!("Finished successfully.");
    let title = shared::localization::text_with_default(
        "templates.uninstall_title",
        &[("app_title", &app_title)],
        "{app_title} Uninstall",
    );
    let content = shared::localization::text_with_default(
        "dialogs.uninstall_success.content",
        &[("app_title", &app_title)],
        "{app_title} was successfully uninstalled.",
    );
    shared::dialogs::show_info(title.as_str(), None, &content);

    let dead_path = root_path.join(".dead");
    let _ = File::create(dead_path);

    if delete_self {
        if let Err(e) = windows::register_intent_to_delete_self(3, &root_path) {
            warn!("Unable to schedule self delete ({}).", e);
        }
    }

    Ok(())
}
