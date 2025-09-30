use super::{dialogs_common::*, dialogs_const::*};
use crate::shared::localization;
use velopack::bundle::Manifest;
use anyhow::Result;
use std::path::PathBuf;
use winsafe::{self as w, co, prelude::*, WString};
use velopack::locator::{auto_locate_app_manifest, LocationContext};

pub fn show_restart_required(app: &Manifest) {
    let version = app.version.to_string();
    let title = localization::text_with_default(
        "templates.setup_title",
        &[("app_title", app.title.as_str()), ("app_version", version.as_str())],
        "{app_title} Setup {app_version}",
    );
    let instruction = localization::text_or_default("dialogs.restart_required.instruction", "Restart Required");
    let content = localization::text_or_default(
        "dialogs.restart_required.content",
        "A restart is required before Setup can continue. Please restart your computer and try again.",
    );
    show_warn(&title, Some(instruction.as_str()), &content);
}

pub fn show_update_missing_dependencies_dialog(
    app: &Manifest,
    depedency_string: &str,
    from: &semver::Version,
    to: &semver::Version,
) -> bool {
    if get_silent() {
        // this has different behavior to show_setup_missing_dependencies_dialog,
        // if silent is true then we will bail because the app is probably exiting
        // and installing dependencies may result in a UAC prompt.
        warn!("Cancelling pre-requisite installation because silent flag is true.");
        return false;
    }

    let from_version = from.to_string();
    let to_version = to.to_string();
    let title = localization::text_with_default(
        "templates.update_title_no_version",
        &[("app_title", app.title.as_str())],
        "{app_title} Update",
    );
    let header = localization::text_with_default(
        "dialogs.update_missing_dependencies.header",
        &[
            ("app_title", app.title.as_str()),
            ("from_version", from_version.as_str()),
            ("to_version", to_version.as_str()),
        ],
        "{app_title} would like to update from {from_version} to {to_version}",
    );
    let content = localization::text_with_default(
        "dialogs.update_missing_dependencies.content",
        &[
            ("app_title", app.title.as_str()),
            ("from_version", from_version.as_str()),
            ("to_version", to_version.as_str()),
            ("dependency_list", depedency_string),
        ],
        "{app_title} {to_version} has missing dependencies which need to be installed: {dependency_list}. Would you like to continue?",
    );
    let button = localization::text_or_default("buttons.install_and_update", "Install & Update");

    show_ok_cancel(&title, Some(header.as_str()), &content, Some(button.as_str()))
}

pub fn show_setup_missing_dependencies_dialog(app: &Manifest, depedency_string: &str) -> bool {
    if get_silent() {
        return true;
    }

    let version = app.version.to_string();
    let title = localization::text_with_default(
        "templates.setup_title",
        &[("app_title", app.title.as_str()), ("app_version", version.as_str())],
        "{app_title} Setup {app_version}",
    );
    let header = localization::text_with_default(
        "dialogs.setup_missing_dependencies.header",
        &[("app_title", app.title.as_str())],
        "{app_title} has missing system dependencies.",
    );
    let content = localization::text_with_default(
        "dialogs.setup_missing_dependencies.content",
        &[("app_title", app.title.as_str()), ("dependency_list", depedency_string)],
        "{app_title} requires the following packages to be installed: {dependency_list}. Would you like to continue?",
    );
    let button = localization::text_or_default("buttons.install", "Install");

    show_ok_cancel(&title, Some(header.as_str()), &content, Some(button.as_str()))
}

pub fn show_uninstall_complete_with_errors_dialog(app_title: &str, log_path: Option<&PathBuf>) {
    if get_silent() {
        return;
    }

    let mut setup_name = WString::from_str(localization::text_with_default(
        "templates.uninstall_title",
        &[("app_title", app_title)],
        "{app_title} Uninstall",
    ));
    let mut instruction = WString::from_str(localization::text_with_default(
        "dialogs.uninstall_complete_with_errors.instruction",
        &[("app_title", app_title)],
        "{app_title} uninstall has completed with errors.",
    ));
    let mut content = WString::from_str(localization::text_or_default(
        "dialogs.uninstall_complete_with_errors.content",
        "There may be left-over files or directories on your system. You can attempt to remove these manually or re-install the application and try again.",
    ));

    let mut config: w::TASKDIALOGCONFIG = Default::default();
    config.dwFlags = co::TDF::ENABLE_HYPERLINKS | co::TDF::SIZE_TO_CONTENT;
    config.dwCommonButtons = co::TDCBF::OK;
    config.set_pszMainIcon(w::IconIdTdicon::Tdicon(co::TD_ICON::WARNING));
    config.set_pszWindowTitle(Some(&mut setup_name));
    config.set_pszMainInstruction(Some(&mut instruction));
    config.set_pszContent(Some(&mut content));

    let footer_path = log_path.map(|p| p.to_string_lossy().to_string()).unwrap_or("".to_string());
    let mut footer = WString::from_str(localization::text_with_default(
        "dialogs.uninstall_complete_with_errors.footer",
        &[("log_path", footer_path.as_str())],
        "Log file: '<A HREF=\"na\">{log_path}</A>'",
    ));
    if let Some(log_path) = log_path {
        if log_path.exists() {
            config.set_pszFooterIcon(w::IconId::Id(co::TD_ICON::INFORMATION.into()));
            config.set_pszFooter(Some(&mut footer));
            config.lpCallbackData = log_path as *const PathBuf as usize;
            config.pfCallback = Some(task_dialog_callback);
        }
    }

    let _ = w::TaskDialogIndirect(&config, None);
}

pub fn show_processes_locking_folder_dialog(app_title: &str, app_version: &str, process_names: &str) -> DialogResult {
    if get_silent() {
        return DialogResult::Cancel;
    }

    let mut config: w::TASKDIALOGCONFIG = Default::default();
    config.set_pszMainIcon(w::IconIdTdicon::Tdicon(co::TD_ICON::INFORMATION));

    let mut update_name = WString::from_str(localization::text_with_default(
        "templates.update_title",
        &[("app_title", app_title), ("app_version", app_version)],
        "{app_title} Update {app_version}",
    ));
    let mut instruction = WString::from_str(localization::text_with_default(
        "templates.update_title_no_version",
        &[("app_title", app_title)],
        "{app_title} Update",
    ));

    let mut content = WString::from_str(localization::text_with_default(
        "dialogs.processes_locking_folder.content",
        &[("process_names", process_names), ("app_title", app_title)],
        "There are programs ({process_names}) preventing the {app_title} update from proceeding.\n\nYou can press Continue to have this updater attempt to close them automatically, or if you've closed them yourself press Retry for the updater to check again.",
    ));

    let mut btn_retry_txt = WString::from_str(localization::text_or_default(
        "buttons.retry_close_programs",
        "Retry\nTry again if you've closed the program(s)",
    ));
    let mut btn_continue_txt = WString::from_str(localization::text_or_default(
        "buttons.continue_close_programs",
        "Continue\nAttempt to close the program(s) automatically",
    ));
    let mut btn_cancel_txt = WString::from_str(localization::text_or_default(
        "buttons.cancel_stop_update",
        "Cancel\nThe update will not continue",
    ));

    let mut btn_retry = w::TASKDIALOG_BUTTON::default();
    btn_retry.set_nButtonID(co::DLGID::RETRY.into());
    btn_retry.set_pszButtonText(Some(&mut btn_retry_txt));

    let mut btn_continue = w::TASKDIALOG_BUTTON::default();
    btn_continue.set_nButtonID(co::DLGID::CONTINUE.into());
    btn_continue.set_pszButtonText(Some(&mut btn_continue_txt));

    let mut btn_cancel = w::TASKDIALOG_BUTTON::default();
    btn_cancel.set_nButtonID(co::DLGID::CANCEL.into());
    btn_cancel.set_pszButtonText(Some(&mut btn_cancel_txt));

    let mut custom_btns = vec![btn_retry, btn_continue, btn_cancel];
    config.dwFlags = co::TDF::USE_COMMAND_LINKS;
    config.set_pButtons(Some(&mut custom_btns));
    config.set_pszWindowTitle(Some(&mut update_name));
    config.set_pszMainInstruction(Some(&mut instruction));
    config.set_pszContent(Some(&mut content));

    let (btn, _) = w::TaskDialogIndirect(&config, None).ok().unwrap_or((co::DLGID::CANCEL, 0));
    DialogResult::from_win(btn)
}

pub fn show_overwrite_repair_dialog(app: &Manifest, root_path: &PathBuf, root_is_default: bool) -> bool {
    if get_silent() {
        return true;
    }

    // these are the defaults, if we can't detect the current app version - we call it "Repair"
    let mut config: w::TASKDIALOGCONFIG = Default::default();
    let mut icon = co::TD_ICON::WARNING;

    let app_version = app.version.to_string();
    let mut setup_name = WString::from_str(localization::text_with_default(
        "templates.setup_title",
        &[("app_title", app.title.as_str()), ("app_version", app_version.as_str())],
        "{app_title} Setup {app_version}",
    ));

    let mut instruction_text = localization::text_with_default(
        "dialogs.overwrite_repair.default_instruction",
        &[("app_title", app.title.as_str())],
        "{app_title} is already installed.",
    );
    let mut content_text = localization::text_with_default(
        "dialogs.overwrite_repair.default_content",
        &[("app_title", app.title.as_str())],
        "This application is installed on your computer. If it is not functioning correctly, you can attempt to repair it.",
    );
    let mut yes_button_text = localization::text_with_default(
        "buttons.repair",
        &[("app_version", app_version.as_str())],
        "Repair\nErase the application and re-install version {app_version}.",
    );
    let cancel_button_text = localization::text_or_default(
        "buttons.cancel_backup_first",
        "Cancel\nBackup or save your work first",
    );

    // if we can detect the current app version, we call it "Update" or "Downgrade"
    let old_app = auto_locate_app_manifest(LocationContext::FromSpecifiedRootDir(root_path.to_owned()));
    if let Ok(old) = old_app {
        let old_version = old.get_manifest_version();
        if old_version < app.version {
            let old_version_str = old_version.to_string();
            instruction_text = localization::text_with_default(
                "dialogs.overwrite_repair.update_instruction",
                &[("app_title", app.title.as_str())],
                "An older version of {app_title} is installed.",
            );
            content_text = localization::text_with_default(
                "dialogs.overwrite_repair.update_content",
                &[
                    ("old_version", old_version_str.as_str()),
                    ("new_version", app_version.as_str()),
                ],
                "Would you like to update from {old_version} to {new_version}?",
            );
            yes_button_text = localization::text_with_default(
                "buttons.update",
                &[("target_version", app_version.as_str())],
                "Update\nTo version {target_version}",
            );
            icon = co::TD_ICON::INFORMATION;
        } else if old_version > app.version {
            let old_version_str = old_version.to_string();
            instruction_text = localization::text_with_default(
                "dialogs.overwrite_repair.downgrade_instruction",
                &[("app_title", app.title.as_str())],
                "A newer version of {app_title} is installed.",
            );
            content_text = localization::text_with_default(
                "dialogs.overwrite_repair.downgrade_content",
                &[("old_version", old_version_str.as_str())],
                "You already have {old_version} installed. Would you like to downgrade this application to an older version?",
            );
            yes_button_text = localization::text_with_default(
                "buttons.downgrade",
                &[("target_version", app_version.as_str())],
                "Downgrade\nTo version {target_version}",
            );
        }
    }

    config.set_pszMainIcon(w::IconIdTdicon::Tdicon(icon));

    let mut instruction = WString::from_str(instruction_text);
    let mut content = WString::from_str(content_text);
    let mut btn_yes_txt = WString::from_str(yes_button_text);
    let mut btn_cancel_txt = WString::from_str(cancel_button_text);

    let custom_path = root_path.display().to_string();
    let footer_string = if root_is_default {
        localization::text_with_default(
            "templates.install_directory_default",
            &[("app_id", app.id.as_str())],
            "The install directory is '<A HREF=\"na\">%LocalAppData%\\{app_id}</A>'",
        )
    } else {
        localization::text_with_default(
            "templates.install_directory_custom",
            &[("path", custom_path.as_str())],
            "The install directory is '<A HREF=\"na\">{path}</A>'",
        )
    };
    let mut footer = WString::from_str(footer_string);

    let mut btn_yes = w::TASKDIALOG_BUTTON::default();
    btn_yes.set_nButtonID(co::DLGID::YES.into());
    btn_yes.set_pszButtonText(Some(&mut btn_yes_txt));

    let mut btn_cancel = w::TASKDIALOG_BUTTON::default();
    btn_cancel.set_nButtonID(co::DLGID::CANCEL.into());
    btn_cancel.set_pszButtonText(Some(&mut btn_cancel_txt));

    let mut custom_btns = Vec::with_capacity(2);
    custom_btns.push(btn_yes);
    custom_btns.push(btn_cancel);

    config.dwFlags = co::TDF::ENABLE_HYPERLINKS | co::TDF::USE_COMMAND_LINKS;
    config.set_pButtons(Some(&mut custom_btns));
    config.set_pszWindowTitle(Some(&mut setup_name));
    config.set_pszMainInstruction(Some(&mut instruction));
    config.set_pszContent(Some(&mut content));
    config.set_pszFooterIcon(w::IconId::Id(co::TD_ICON::INFORMATION.into()));
    config.set_pszFooter(Some(&mut footer));

    config.lpCallbackData = root_path as *const PathBuf as usize;
    config.pfCallback = Some(task_dialog_callback);

    let (btn, _) = w::TaskDialogIndirect(&config, None).ok().unwrap_or_else(|| (co::DLGID::YES, 0));
    return btn == co::DLGID::YES;
}

extern "system" fn task_dialog_callback(_: w::HWND, msg: co::TDN, _: usize, _: isize, lp_ref_data: usize) -> co::HRESULT {
    if msg == co::TDN::HYPERLINK_CLICKED {
        let raw = lp_ref_data as *const PathBuf;
        let path: &PathBuf = unsafe { &*raw };
        let dir = path.to_str().unwrap();
        w::HWND::GetDesktopWindow().ShellExecute("open", &dir, None, None, co::SW::SHOWDEFAULT).ok();
        return co::HRESULT::S_FALSE; // do not close dialog
    }
    return co::HRESULT::S_OK; // close dialog on button press
}

pub fn generate_confirm(
    title: &str,
    header: Option<&str>,
    body: &str,
    ok_text: Option<&str>,
    btns: DialogButton,
    ico: DialogIcon,
) -> Result<DialogResult> {
    let hparent = w::HWND::GetDesktopWindow();
    let mut ok_text_buf = WString::from_opt_str(ok_text);
    let mut custom_btns = if ok_text.is_some() {
        let mut td_btn = w::TASKDIALOG_BUTTON::default();
        td_btn.set_nButtonID(co::DLGID::OK.into());
        td_btn.set_pszButtonText(Some(&mut ok_text_buf));
        let mut custom_btns = Vec::with_capacity(1);
        custom_btns.push(td_btn);
        custom_btns
    } else {
        Vec::<w::TASKDIALOG_BUTTON>::default()
    };

    let mut tdc = w::TASKDIALOGCONFIG::default();
    tdc.hwndParent = unsafe { hparent.raw_copy() };
    tdc.dwFlags = co::TDF::ALLOW_DIALOG_CANCELLATION | co::TDF::POSITION_RELATIVE_TO_WINDOW;
    tdc.dwCommonButtons = btns.to_win();
    tdc.set_pszMainIcon(w::IconIdTdicon::Tdicon(ico.to_win()));

    if ok_text.is_some() {
        tdc.set_pButtons(Some(&mut custom_btns));
    }

    let mut title_buf = WString::from_str(title);
    tdc.set_pszWindowTitle(Some(&mut title_buf));

    let mut header_buf = WString::from_opt_str(header);
    if header.is_some() {
        tdc.set_pszMainInstruction(Some(&mut header_buf));
    }

    let mut body_buf = WString::from_str(body);
    tdc.set_pszContent(Some(&mut body_buf));

    let result = w::TaskDialogIndirect(&tdc, None).map(|(dlg_id, _)| dlg_id)?;
    Ok(DialogResult::from_win(result))
}

pub fn generate_alert(
    title: &str,
    header: Option<&str>,
    body: &str,
    ok_text: Option<&str>,
    btns: DialogButton,
    ico: DialogIcon,
) -> Result<()> {
    let _ = generate_confirm(title, header, body, ok_text, btns, ico).map(|_| ())?;
    Ok(())
}
