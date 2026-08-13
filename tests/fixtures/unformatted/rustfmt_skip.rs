#[rustfmt::skip]
fn skipped() {
    let value = builder
        .build();
    consume(value);
}
fn normal() {
    let value = builder
        .build();
    consume(value);
}
