//! Resource states shared by loading, empty, ready, and stale-error containers.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T> {
    Loading {
        last_accepted: Option<T>,
    },
    Empty,
    Ready(T),
    Error {
        last_accepted: Option<T>,
        message: String,
    },
}

impl<T> ResourceState<T> {
    pub fn loading(self) -> Self {
        match self {
            Self::Ready(value) => Self::Loading {
                last_accepted: Some(value),
            },
            Self::Error { last_accepted, .. } | Self::Loading { last_accepted } => {
                Self::Loading { last_accepted }
            }
            Self::Empty => Self::Loading {
                last_accepted: None,
            },
        }
    }

    pub fn ready(self, value: T) -> Self {
        Self::Ready(value)
    }

    pub fn error(self, message: impl Into<String>) -> Self {
        let last_accepted = match self {
            Self::Ready(value) => Some(value),
            Self::Loading { last_accepted } | Self::Error { last_accepted, .. } => last_accepted,
            Self::Empty => None,
        };
        Self::Error {
            last_accepted,
            message: message.into(),
        }
    }

    pub const fn accepted(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Loading { last_accepted } | Self::Error { last_accepted, .. } => {
                last_accepted.as_ref()
            }
            Self::Empty => None,
        }
    }

    pub const fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error { message, .. } => Some(message),
            Self::Loading { .. } | Self::Empty | Self::Ready(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_and_error_keep_last_accepted_value() {
        let state = ResourceState::Ready("accepted").loading();
        assert!(state.is_loading());
        assert_eq!(state.accepted(), Some(&"accepted"));
        let state = state.error("refresh failed");
        assert_eq!(state.accepted(), Some(&"accepted"));
        assert_eq!(state.error_message(), Some("refresh failed"));
    }

    #[test]
    fn first_load_error_has_no_invented_content() {
        let state: ResourceState<&str> = ResourceState::Empty.loading().error("unavailable");
        assert_eq!(state.accepted(), None);
        assert_eq!(state.error_message(), Some("unavailable"));
    }
}
