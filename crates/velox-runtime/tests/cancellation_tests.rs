use velox_runtime::CancellationToken;

#[test]
fn token_starts_active() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
}

#[test]
fn token_cancel_sets_flag() {
    let token = CancellationToken::new();
    let token2 = token.clone();
    token.cancel();
    assert!(token.is_cancelled());
    assert!(token2.is_cancelled());
}

#[test]
fn token_drop_does_not_cancel() {
    let token = CancellationToken::new();
    let token2 = token.clone();
    drop(token);
    assert!(!token2.is_cancelled());
}
