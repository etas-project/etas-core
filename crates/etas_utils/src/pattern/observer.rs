#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueChange {
    pub kind: ValueChangeKind,
}

impl ValueChange {
    pub const fn new(kind: ValueChangeKind) -> Self {
        Self { kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueChangeKind {
    Created,
    Updated,
    Replaced,
    Removed,
}

pub trait Observer<T, E = ValueChange> {
    fn on_change(&mut self, value: &T, event: &E);
}

impl<T, E, F> Observer<T, E> for F
where
    F: FnMut(&T, &E),
{
    fn on_change(&mut self, value: &T, event: &E) {
        self(value, event);
    }
}

pub struct Observable<T, E = ValueChange> {
    value: T,
    observers: Vec<Box<dyn Observer<T, E>>>,
}

impl<T, E> Observable<T, E> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            observers: Vec::new(),
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn observe(&mut self, observer: impl Observer<T, E> + 'static) {
        self.observers.push(Box::new(observer));
    }

    pub fn notify(&mut self, event: &E) {
        for observer in &mut self.observers {
            observer.on_change(&self.value, event);
        }
    }

    pub fn update(&mut self, event: &E, update: impl FnOnce(&mut T)) {
        update(&mut self.value);
        self.notify(event);
    }

    pub fn replace(&mut self, value: T, event: &E) -> T {
        let old = std::mem::replace(&mut self.value, value);
        self.notify(event);
        old
    }
}
