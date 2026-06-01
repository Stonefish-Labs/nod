#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Servers,
    Channels,
    Events,
    Detail,
}

impl Focus {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Servers => Self::Channels,
            Self::Channels => Self::Events,
            Self::Events => Self::Detail,
            Self::Detail => Self::Servers,
        }
    }
}
