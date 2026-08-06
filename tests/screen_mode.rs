use termxboard::{MIN_HEIGHT, MIN_WIDTH, ScreenMode, screen_mode};

#[test]
fn undersized_terminal_requests_resize() {
    assert_eq!(screen_mode(MIN_WIDTH - 1, MIN_HEIGHT), ScreenMode::Resize);
    assert_eq!(screen_mode(MIN_WIDTH, MIN_HEIGHT - 1), ScreenMode::Resize);
}

#[test]
fn minimum_terminal_size_shows_dashboard() {
    assert_eq!(screen_mode(MIN_WIDTH, MIN_HEIGHT), ScreenMode::Dashboard);
}
