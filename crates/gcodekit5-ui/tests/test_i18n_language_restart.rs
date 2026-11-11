//! Changing the UI language in Preferences offers a restart: these cover when
//! a restart is needed and how the relaunch command is built.

use gcodekit5_ui::i18n::{needs_restart, relaunch_command};
use std::ffi::OsStr;

#[test]
fn test_i18n_language_restart_needed_only_when_language_changes() {
    assert!(needs_restart("en", "de"));
    assert!(needs_restart("de", "system"));
    assert!(needs_restart("system", "fr"));
    assert!(!needs_restart("de", "de"));
    assert!(!needs_restart("system", "system"));
    // An empty preference is the system language.
    assert!(!needs_restart("system", ""));
    assert!(!needs_restart("", "system"));
}

#[test]
fn test_i18n_language_restart_relaunches_this_executable() -> std::io::Result<()> {
    let command = relaunch_command(None)?;
    assert_eq!(command.get_program(), std::env::current_exe()?.as_os_str());
    let args: Vec<_> = command.get_args().collect();
    let expected: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(
        args,
        expected.iter().map(|a| a.as_os_str()).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_i18n_language_restart_restores_launch_language() -> std::io::Result<()> {
    let language_env = |original: Option<&OsStr>| -> std::io::Result<Option<Option<String>>> {
        let command = relaunch_command(original)?;
        Ok(command
            .get_envs()
            .find(|(k, _)| *k == OsStr::new("LANGUAGE"))
            .map(|(_, v)| v.map(|v| v.to_string_lossy().into_owned())))
    };
    // Launched with LANGUAGE=es: the new instance gets es back, not the
    // preference this run set.
    assert_eq!(
        language_env(Some(OsStr::new("es")))?,
        Some(Some("es".to_string()))
    );
    // Launched without LANGUAGE: it is removed, so "system" really is the
    // system language after a switch back.
    assert_eq!(language_env(None)?, Some(None));
    Ok(())
}
