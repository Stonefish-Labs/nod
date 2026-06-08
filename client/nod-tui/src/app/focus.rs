#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Servers,
    Sources,
    Requests,
    Detail,
}

impl Focus {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Servers => Self::Sources,
            Self::Sources => Self::Requests,
            Self::Requests => Self::Detail,
            Self::Detail => Self::Servers,
        }
    }
}
