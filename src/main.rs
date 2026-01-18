use std::{mem, panic, time::Instant};

use crate::core::Machine;

mod core;

#[derive(Debug)]
#[allow(dead_code)]
enum Action {
    Decrypt,
    Encrypt,
}

/*
 * make a file integrity check, and also suggest deleting the directive
 */

fn main() -> Result<(), core::Error> {
    println!("Enter action:");
    println!("(0) Decrypt files");
    println!("(1) Encrypt files");
    let mut buffer = String::new();
    let result = std::io::stdin().read_line(&mut buffer);
    if result.is_err() {
        println!("stdin error: {}", result.unwrap_err())
    }
    let action: Action =
        unsafe { mem::transmute(buffer.trim().parse::<u8>().unwrap().clamp(0, 1)) };

    eprint!("Enter key: ");
    let mut key = String::new();
    let result = std::io::stdin().read_line(&mut key);
    if result.is_err() {
        panic!("{result:?}");
    }

    let term = console::Term::stdout();
    term.clear_screen().expect("Не удалось очистить экран");
    let _ = drop(term);

    let folder = rfd::FileDialog::new()
        .set_title(format!("Choose folder for {action:?}"))
        .pick_folder();

    let folder = match folder {
        Some(path) => path,
        None => return Ok(()),
    };

    let instant = Instant::now();
    let machine = Machine::with_key(key.into(), folder);

    match action {
        Action::Decrypt => machine.decrypt()?,
        Action::Encrypt => machine.encrypt()?,
    }

    println!("Time elapsed({:?})", instant.elapsed());
    Ok(())
}
