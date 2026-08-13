fn outer() {
    if condition() {
        let value = builder
            .build();
        consume(value);
        for item in items() {
            let prepared = prepare(item)
                .finish();
            consume(prepared);
        }
        after_loop();
    }
    after_if();
}
