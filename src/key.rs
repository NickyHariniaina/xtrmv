#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Key {
    Character(u8),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
}
