fn example(items: &[usize], value: Option<usize>) {
    if items.is_empty() {
        return;
    }
    println!("items");
    for item in items {
        println!("{item}");
    }
    finish_loop();
    while ready() {
        tick();
    }
    finish_while();
    loop {
        break;
    }
    finish_loop_expr();
    match value {
        Some(value) => println!("{value}"),
        None => return,
    }
    finish_match();
    unsafe {
        do_unsafe();
    }
    finish_unsafe();
}
