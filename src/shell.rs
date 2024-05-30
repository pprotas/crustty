use nix::{
    fcntl::{open, OFlag},
    pty::*,
    sys::{stat::Mode, wait::*},
    unistd::*,
};
use std::{
    error::Error,
    os::unix::{io::AsRawFd, process::CommandExt},
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

pub fn spawn_shell(text: &Arc<Mutex<String>>) -> Result<(), Box<dyn Error>> {
    // Interact with shell
    let master = posix_openpt(OFlag::O_RDWR)?;
    grantpt(&master)?;
    unlockpt(&master)?;

    let slave_name = ptsname_r(&master)?;
    let slave_fd = open(&*slave_name, OFlag::O_RDWR, Mode::empty())?;

    let text_clone = Arc::clone(&text);

    thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        let master_fd = master.as_raw_fd();

        match unsafe { fork().unwrap() } {
            ForkResult::Parent { child } => {
                close(slave_fd).unwrap();

                loop {
                    match read(master_fd, &mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut text = text_clone.lock().unwrap();
                            text.push_str(&String::from_utf8_lossy(&buffer[..n]));
                        }
                        Err(err) => {
                            eprintln!("Error reading from PTY: {}", err);
                            break;
                        }
                    }
                }

                waitpid(child, None).unwrap();
            }
            ForkResult::Child => {
                close(master_fd).unwrap();

                dup2(slave_fd, 0).unwrap();
                dup2(slave_fd, 1).unwrap();
                dup2(slave_fd, 2).unwrap();

                Command::new("/bin/zsh")
                    .arg("-c")
                    .arg("ping www.google.com")
                    .exec();
            }
        }
    });

    Ok(())
}
