use sigmaos_analytics::{Analytics, Event};

#[test]
fn test_record_and_query_aaa() {
    // Arrange
    let mut a = Analytics::new();

    // Act
    a.record(Event::new("page", "view").with_tag("home"));
    a.record(Event::new("page", "view").with_tag("about"));
    a.record(Event::new("click", "button").with_value(1.0));

    // Assert
    assert_eq!(a.event_count(), 3);
    assert_eq!(a.get_counter("page", "view"), 2);
    assert_eq!(a.get_counter("click", "button"), 1);
}
