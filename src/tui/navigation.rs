//! Typed, application-owned route selection with no terminal-widget dependency.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Route {
    Overview,
    Hooks,
    Diagnostics,
    Settings,
}

impl Route {
    pub const ALL: [Self; 4] = [
        Self::Overview,
        Self::Hooks,
        Self::Diagnostics,
        Self::Settings,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Hooks => 1,
            Self::Diagnostics => 2,
            Self::Settings => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavigationState {
    selected: Route,
    active: Route,
}

impl NavigationState {
    pub const fn new() -> Self {
        Self {
            selected: Route::Overview,
            active: Route::Overview,
        }
    }

    pub const fn selected(self) -> Route {
        self.selected
    }

    pub const fn active(self) -> Route {
        self.active
    }

    pub fn move_by(&mut self, delta: isize) {
        let index =
            (self.selected.index() as isize + delta).rem_euclid(Route::ALL.len() as isize) as usize;
        self.selected = Route::ALL[index];
    }

    pub fn activate_selected(&mut self) {
        self.active = self.selected;
    }

    pub fn activate(&mut self, route: Route) {
        self.selected = route;
        self.active = route;
    }

    pub fn back(&mut self) {
        self.selected = self.active;
    }
}

impl Default for NavigationState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_wraps_and_activates_a_typed_route() {
        let mut navigation = NavigationState::new();
        navigation.move_by(-1);
        assert_eq!(navigation.selected(), Route::Settings);
        navigation.activate_selected();
        assert_eq!(navigation.active(), Route::Settings);
        navigation.move_by(1);
        navigation.back();
        assert_eq!(navigation.selected(), Route::Settings);
    }
}
