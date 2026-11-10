//! Regression test: choosing a UI language must not replace `LANG` with a
//! locale the C library does not have (e.g. `en.UTF-8`), which made GTK warn
//! "Locale not supported by C library" at start-up.
#![allow(unsafe_code)]

use gcodekit5_ui::i18n;

#[test]
fn test_i18n_language_choice_keeps_system_lang() {
    let _env = crate::global_env_lock();
    // SAFETY: the binary-wide env lock is held and this test starts no threads.
    unsafe {
        std::env::set_var("LANG", "en_US.UTF-8");
        std::env::remove_var("LANGUAGE");
    }

    i18n::init(Some("en".to_string()));

    assert_eq!(std::env::var("LANG").as_deref(), Ok("en_US.UTF-8"));
    assert_eq!(std::env::var("LANGUAGE").as_deref(), Ok("en"));
}
