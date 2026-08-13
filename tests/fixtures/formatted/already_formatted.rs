fn first() {}

fn second() {
    let value = builder
        .build();

    if condition() {
        consume(value);
    }

    finish();
}
