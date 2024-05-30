use crustty::{render::render, shell::spawn_shell};
use std::{
    env,
    sync::{Arc, Mutex},
};

fn main() {
    // Use Mesa framework for rendering
    std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");

    // Use X11 instead of Wayland. Wayland has transparent letters, for some reason.
    if cfg!(target_os = "linux") && env::var("WINIT_UNIX_BACKEND").is_err() {
        env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    // This is basically the terminal buffer. The shell modifies it, and the OpenGL event loop
    // displays it on the screen.
    let text = Arc::new(Mutex::new(String::new()));

    spawn_shell(&text).unwrap();

    render(text);
}
