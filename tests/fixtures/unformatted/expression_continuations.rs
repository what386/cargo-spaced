fn example(condition: bool, value: Option<usize>) {
    let result = if condition {
        1
    } else {
        2
    };
    consume(result);

    let value = match value {
        Some(value) => value,
        None => return,
    }
    .saturating_add(1);
    consume(value);
}
