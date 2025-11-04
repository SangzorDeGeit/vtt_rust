use vtt_rust::fog_of_war::Operation;
use vtt_rust::open_vtt;
use vtt_rust::vtt::Coordinate;
#[test]
fn test_rectangle_count_basic() {
    let mut vtt = open_vtt("tests/resources/example3.dd2vtt").expect("Could not open vtt");

    vtt.fow_hide_all();
    let pov = Coordinate { x: 9., y: 9. };
    vtt.fow_change(pov, Operation::SHOW, true, true)
        .expect("Failed to change fow");
    assert_eq!(
        vtt.get_fow().get_rectangles().len(),
        vtt.get_fow()
            .rectangle_count
            .load(std::sync::atomic::Ordering::Relaxed),
        "Expected amount of rectangles and vec allocation to be the same"
    );
}

#[test]
fn test_rectangle_count_hide_hide() {
    let mut vtt = open_vtt("tests/resources/example3.dd2vtt").expect("Could not open vtt");

    vtt.fow_hide_all();
    let pov = Coordinate { x: 9., y: 9. };
    vtt.fow_change(pov, Operation::HIDE, true, true)
        .expect("Failed to change fow");
    vtt.fow_change(pov, Operation::SHOW, true, true)
        .expect("Failed to change fow");
    assert_eq!(
        vtt.get_fow().get_rectangles().len(),
        vtt.get_fow()
            .rectangle_count
            .load(std::sync::atomic::Ordering::Relaxed),
        "Expected amount of rectangles and vec allocation to be the same"
    );
}

#[test]
fn test_rectangle_count_show_show() {
    let mut vtt = open_vtt("tests/resources/example3.dd2vtt").expect("Could not open vtt");

    vtt.fow_show_all();
    let pov = Coordinate { x: 9., y: 9. };
    vtt.fow_change(pov, Operation::SHOW, true, true)
        .expect("Failed to change fow");
    assert_eq!(
        vtt.get_fow().get_rectangles().len(),
        vtt.get_fow()
            .rectangle_count
            .load(std::sync::atomic::Ordering::Relaxed),
        "Expected amount of rectangles and vec allocation to be the same"
    );
}
