#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Servers,
    Channels,
    Requests,
    Detail,
}

impl Focus {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Servers => Self::Channels,
            Self::Channels => Self::Requests,
            Self::Requests => Self::Detail,
            Self::Detail => Self::Servers,
        }
    }
}
