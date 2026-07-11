use skilltap_core::domain::ObservationBatch;
use skilltap_harnesses::normalize_observations;

#[test]
fn normalization_is_ephemeral_and_deterministic_for_an_empty_batch() {
    let first = normalize_observations(ObservationBatch::new([]).unwrap(), []).unwrap();
    let second = normalize_observations(ObservationBatch::new([]).unwrap(), []).unwrap();
    assert!(first.is_empty());
    assert_eq!(first, second);
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}
