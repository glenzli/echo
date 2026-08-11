use super::*;

#[test]
fn recovery_schedule_is_immediate_then_bounded() {
    let mut schedule = RecoverySchedule::default();
    assert!(schedule.remaining().is_none());

    for (index, expected) in RETRY_DELAYS.into_iter().enumerate() {
        schedule.defer();
        let remaining = schedule.remaining().expect("probe is deferred");
        assert!(remaining <= expected);
        assert!(remaining > expected.saturating_sub(Duration::from_millis(50)));
        assert_eq!(schedule.attempt, index + 1);
        schedule.next_probe = Some(Instant::now());
    }

    schedule.defer();
    let capped = schedule.remaining().expect("capped probe is deferred");
    assert!(capped <= Duration::from_mins(1));
    assert!(capped > Duration::from_secs(59));

    schedule.reset();
    assert_eq!(schedule.attempt, 0);
    assert!(schedule.remaining().is_none());
}
