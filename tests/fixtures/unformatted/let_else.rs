fn parse(value: Option<usize>) {
    let Some(value) = value else {
        return;
    };
    println!("{value}");
}
