fn branches(c: bool) {
    let mut x = 1;
    if c {
        x = 2;
        sink(x);
    } else {
        sink(x);
    }
    sink(x);
}

fn inner_block() {
    let mut x = 1;
    {
        x = 2;
    }
    sink(x);
}

fn loop_reentry() {
    let mut x = 1;
    loop {
        sink(x);
        x = 2;
    }
}

fn while_body(mut n: i32) {
    let mut x = 0;
    while n > 0 {
        x = n;
        sink(x);
        x = 7;
        n -= 1;
    }
    sink(x);
}

fn labeled(c: bool) {
    let mut x = 0;
    'a: {
        x = 1;
        if c {
            break 'a;
        }
        x = 2;
        sink(x);
    }
    sink(x);
}

fn labeled_inner(c: bool) {
    let mut x = 0;
    x = 1;
    'a: {
        if c {
            break 'a;
        }
        x = 2;
    }
    sink(x);
}

fn arm_block(o: Option<i32>) {
    let mut x = 0;
    match o {
        Some(_) => {
            x = 1;
            x = 2;
            sink(x);
        }
        None => {
            sink(x);
        }
    }
}

fn unsafe_block() {
    let mut x = 0;
    unsafe {
        x = 1;
        x = 2;
        sink(x);
    }
}

fn early_exit(c: bool) {
    let mut x = 1;
    if c {
        return;
    }
    x = 2;
    sink(x);
}
