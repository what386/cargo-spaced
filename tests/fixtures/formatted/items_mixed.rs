struct Thing {
    value: usize,
}

impl Thing {
    fn value(&self) -> usize {
        self.value
    }
}

type Value = usize;

enum State {
    Ready,
    Done,
}

const DEFAULT: Value = 1;
