//! Top-level route state delegated to the shared Human interface core.

use terminal_ui_contract::navigation::TopLevelNavigation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Route {
    Overview,
    Hooks,
    Changes,
    Diagnostics,
    Settings,
}

impl Route {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Hooks,
        Self::Changes,
        Self::Diagnostics,
        Self::Settings,
    ];
}

/// One direct current route with no selected-vs-active split.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavigationState {
    inner: TopLevelNavigation<Route>,
}

impl NavigationState {
    pub const fn new() -> Self {
        Self {
            inner: TopLevelNavigation::new(Route::Overview),
        }
    }

    pub const fn current(self) -> Route {
        self.inner.current()
    }

    pub fn move_by(&mut self, delta: isize) {
        let _ = self.inner.move_by(&Route::ALL, delta);
    }

    pub fn activate(&mut self, route: Route) {
        self.inner.set_current(route);
    }
}

impl Default for NavigationState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{NavigationState, Route};

    #[test]
    fn direct_current_route_moves_without_selected_active_split() {
        let mut navigation = NavigationState::new();
        navigation.move_by(-1);
        assert_eq!(navigation.current(), Route::Settings);
        navigation.move_by(1);
        assert_eq!(navigation.current(), Route::Overview);
    }
}
