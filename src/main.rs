use std::{mem, path::PathBuf, time::Instant};

use age::secrecy::SecretString;

use crate::core::Machine;

mod core;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum Action {
    Decrypt,
    Encrypt,
    Filter,
}

impl Action {
    const COUNT: u8 = 3;
}

struct Gallery {}

impl Gallery {
    pub fn run(&mut self) -> Result<Action, core::Error> {
        let action = self.get_action();
        match action {
            Action::Decrypt | Action::Encrypt => {
                let key = self.get_key();
                let folder = self.select_folder(action);

                let instant = Instant::now();
                let machine = Machine::with_key(key.into(), folder.clone());

                match action {
                    Action::Decrypt => machine.decrypt()?,
                    Action::Encrypt => machine.encrypt()?,
                    _ => (),
                }

                self.clear();
                println!("Time elapsed({:?})\n", instant.elapsed());
            }
            Action::Filter => todo!(),
        }

        Ok(action)
    }

    pub fn get_action(&self) -> Action {
        println!("Enter action:");
        println!("(0) Decrypt files");
        println!("(1) Encrypt files");
        println!("(2) Switch Filter");

        let mut buffer = String::new();
        let _ = std::io::stdin().read_line(&mut buffer);
        match buffer.trim().parse::<u8>() {
            Ok(num) => unsafe { mem::transmute(num.clamp(0, Action::COUNT - 1)) },
            Err(_) => self.get_action(),
        }
    }

    pub fn select_folder(&self, action: Action) -> PathBuf {
        let folder = rfd::FileDialog::new()
            .set_title(format!("Choose folder for {action:?}"))
            .pick_folder();

        match folder {
            Some(path) => path,
            None => self.select_folder(action),
        }
    }

    pub fn get_key(&self) -> SecretString {
        eprint!("Enter key: ");
        let mut key = String::new();
        let _ = std::io::stdin().read_line(&mut key);
        key.into()
    }

    pub fn clear(&self) {
        match console::Term::stdout().clear_screen() {
            Ok(_) => (),
            Err(e) => println!("Console clear exception: {}", e.kind()),
        }
    }
}
/*
 * make a file integrity check, and also suggest deleting the directive
 */

fn main() {
    loop {
        let mut gallery = Gallery {};
        match gallery.run() {
            Ok(action) => match action {
                Action::Decrypt | Action::Encrypt => break,
                _ => (),
            },
            Err(err) => {
                println!("{err:?}");
                break;
            }
        }
    }
}

// fn remove_is(folder: PathBuf) {
//     println!("Delete selected folder?:");
//     println!("(0) No");
//     println!("(1) Yes");
//     let mut buffer = String::new();
//     let result = std::io::stdin().read_line(&mut buffer);
//     if result.is_err() {
//         println!("stdin error: {}", result.unwrap_err())
//     }
//     let action: Boolean =
//         unsafe { mem::transmute(buffer.trim().parse::<u8>().unwrap().clamp(0, 1)) };

//     let _ = match action {
//         Boolean::Yes => fs::remove_dir_all(folder),
//         Boolean::No => return,
//     };
// }
