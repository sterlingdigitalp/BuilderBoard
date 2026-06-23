pub trait AuthSessionStore {
    fn current_subject(&self) -> Option<&str>;
}
