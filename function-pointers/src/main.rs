struct Data {
    callback: fn(&str),
    callback_2: fn(&str),
}

fn main() {
    let mut data = Data {
        callback: print_word, 
        callback_2: print_word, 
    };
    call_print(&mut data);
}

fn call_print(data: &mut Data) {
    (data.callback)("HI");
    (data.callback_2)("skdjfklasj");
}

fn print_word(text: &str) {
    println!("Print HAHA + {}", text);
}
