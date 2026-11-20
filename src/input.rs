#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputKey {
    Text,
    Enter,
    DeleteBackward,
    DeleteForward,
    Left,
    Right,
    Home,
    End,
}
