fn example(value: Value) {
    match value {
        Value::A => {
            first();
        }
        Value::B => {
            second();
        }
        Value::C => {
            third();
        }
    }
}
