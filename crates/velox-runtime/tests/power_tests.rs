use velox_runtime::{PowerClass, PowerPolicy};

#[test]
fn performance_runs_all_classes() {
    let policy = PowerPolicy::Performance;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(policy.should_run(PowerClass::Decorative));
    assert!(policy.should_run(PowerClass::Background));
}

#[test]
fn adaptive_allows_all() {
    let policy = PowerPolicy::Adaptive;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(policy.should_run(PowerClass::Decorative));
    assert!(policy.should_run(PowerClass::Background));
}

#[test]
fn saving_only_essential_and_interactive() {
    let policy = PowerPolicy::Saving;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(!policy.should_run(PowerClass::Decorative));
    assert!(!policy.should_run(PowerClass::Background));
}

#[test]
fn default_policy_is_adaptive() {
    assert_eq!(PowerPolicy::default(), PowerPolicy::Adaptive);
}
