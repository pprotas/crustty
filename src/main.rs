use glium::{
    glutin::{event_loop::EventLoop, window::WindowBuilder, ContextBuilder},
    Display, *,
};
use glutin::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("CrusTTY");

    let context = ContextBuilder::new().with_vsync(true);

    let display = Display::new(window, context, &event_loop)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => (),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let mut target = display.draw();
                target.clear_color(0.0, 0.0, 1.0, 1.0);
                target.finish().unwrap();
            }
            _ => (),
        }
    })
}
