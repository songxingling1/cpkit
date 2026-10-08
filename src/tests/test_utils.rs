use super::*;

#[test]
fn valid_string() {
    let vec = vec![65, 66, 67];
    let res = vecu8_to_string_lossy(vec);
    assert_eq!(res, "ABC");
}

#[test]
fn invalid_string() {
    let vec = vec![65, 66, 67, 0xFF];
    let res = vecu8_to_string_lossy(vec);
    assert_eq!(res, "ABC\u{FFFD}");
}
