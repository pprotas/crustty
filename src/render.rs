use glium::{implement_vertex, program, uniform, Display, Surface};
use glutin::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use rusttype::{gpu_cache::Cache, point, vector, Font, PositionedGlyph, Rect, Scale};
use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

fn layout_text<'a>(
    font: &Font<'a>,
    scale: Scale,
    width: u32,
    text: &str,
) -> Vec<PositionedGlyph<'a>> {
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;

    for c in text.chars() {
        if c.is_control() {
            match c {
                '\r' => {
                    caret = point(0.0, caret.y + advance_height);
                }
                _ => {}
            }
            continue;
        }

        let base_glyph = font.glyph(c);
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }

        last_glyph_id = Some(base_glyph.id());

        let mut glyph = base_glyph.scaled(scale).positioned(caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > width as i32 {
                caret = point(0.0, caret.y + advance_height);
                glyph.set_position(caret);
                last_glyph_id = None;
            }
        }
        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(glyph);
    }

    result
}

pub fn render(text: Arc<Mutex<String>>) {
    // Embed the font file into the binary
    let font_data = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");
    // Load he font into memory
    let font = Font::try_from_bytes(font_data as &[u8]).expect("Failed to load font");

    // This is an OS window, abstracted by the winit library to give us windows across different
    // OSes
    let window = WindowBuilder::new().with_title("CrusTTY");
    // This is used to interact with the OS window and respond to different events, again from the
    // winit library
    let event_loop = EventLoop::new();

    // OpenGL context is used to issue rendering commands to the GPU
    let context = ContextBuilder::new().with_vsync(true);

    // Higher-level abstraction of a combination of an OS window and OpenGL context
    let display = Display::new(window, context, &event_loop).expect("Failed to initialize display");

    // Access the window's scale factor, this is used to have everything scaled correctly on
    // different screen DPIs
    let scale = display.gl_window().window().scale_factor();

    // The base size of the font cache is 512x512, but we need to take the scale factor into
    // account
    let (cache_width, cache_height) = ((512.0 * scale) as u32, (512.0 * scale) as u32);
    // The font cache is used for storing pre-rasterized glyphs, which can improve font rendering
    // performance
    // Glyphs are a typography and font rendering concept. They are a visual representation of a
    // character in a specific font and style.
    // Rasterization means that the glyph data is converted from vectors into a pixel grid that is
    // used to display the letters on the screen.
    // rusttype handles this for us, and OpenGL actually displays the pixels on the screen.
    let mut cache: Cache<'static> = Cache::builder()
        .dimensions(cache_width, cache_height)
        .build();

    // program! is a macro that is used to simplify creating OpenGL shader programs in GLSL (OpenGL
    // Shading Language)
    let program = program!(
    &display,
    // 140 is GLSL 1.40, which corresponds to OpenGL version 3.1
    140 => {
            // Source code for a vertex shader
            vertex: "
                #version 140

                in vec2 position;
                in vec2 tex_coords;
                in vec4 colour;

                out vec2 v_tex_coords;
                out vec4 v_colour;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_tex_coords = tex_coords;
                    v_colour = colour;
                }
            ",
            // Source code for a fragment shader
            fragment: "
                #version 140

                uniform sampler2D tex;

                in vec2 v_tex_coords;
                in vec4 v_colour;

                out vec4 f_colour;

                void main() {
                    f_colour = v_colour * vec4(1.0, 1.0, 1.0, texture(tex, v_tex_coords).r);
                }
            "
    })
    .expect("Failed to compile GLSL shaders");

    // Create a texture for storing the pre-rasterized glyphs
    let cache_tex = glium::texture::Texture2d::with_format(
        // The display contains the OpenGL context which is necessary to create the texture
        &display,
        // This struct represents a 2D image that is used to store the texture
        glium::texture::RawImage2d {
            // Create a vector with the value 128 for each pixel (grayscale texture).
            // Then we use the borrow checker to own the data. This means that it can be
            // manipulated by glium without it having to worry about the original source.
            // TODO: Learn more about borrow checker
            data: Cow::Owned(vec![128u8; cache_width as usize * cache_height as usize]),
            width: cache_width,
            height: cache_height,
            // Each pixel is an unsigned byte (8 bits)
            format: glium::texture::ClientFormat::U8,
        },
        // Indicates that the texture should be stored as 8-bit integers
        glium::texture::UncompressedFloatFormat::U8,
        // Don't generate mipmaps, because we won't be rendering the fonts at different scales or
        // resolutions.
        glium::texture::MipmapsOption::NoMipmap,
    )
    .expect("Failed to create the glyph texture");

    // Automatically implement the Copy and Clone traits
    #[derive(Copy, Clone)]
    // Vertex struct that holds the position of the vertex and its texture
    struct Vertex {
        // Store x and y (2 elements) as a float
        position: [f32; 2],
        tex_coords: [f32; 2],
    }

    // Implement the necessary traits and functions into the Vertex struct, so it can be used in
    // OpenGL
    implement_vertex!(Vertex, position, tex_coords);

    event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => (),
                },
                Event::RedrawRequested(_) => {
                    let scale = display.gl_window().window().scale_factor() as f32;
                    let (width, _): (u32, _) = display.gl_window().window().inner_size().into();

                    let text = text.lock().unwrap().clone();
                    let glyphs = layout_text(&font, Scale::uniform(24.0 * scale), width, &text);
                    for glyph in &glyphs {
                        cache.queue_glyph(0, glyph.clone());
                    }
                    cache.cache_queued(|rect, data| {
                        cache_tex.main_level().write(
                            glium::Rect {
                                left: rect.min.x,
                                bottom: rect.min.y,
                                width: rect.width(),
                                height: rect.height(),
                            },
                        glium::texture::RawImage2d {
                                data: Cow::Borrowed(data),
                                width: rect.width(),
                                height: rect.height(),
                                format: glium::texture::ClientFormat::U8,
                            },
                        );
                    })
                    .unwrap();

                    let uniforms = uniform! {tex: cache_tex.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)};

                    let vertex_buffer = {
                        #[derive(Copy, Clone)]
                        struct Vertex {
                            position: [f32; 2],
                            tex_coords: [f32; 2],
                            colour: [f32; 4],
                        }

                        implement_vertex!(Vertex, position, tex_coords, colour);
                        let colour = [0.0, 0.0, 0.0, 1.0];
                        let (screen_width, screen_height) = {
                            let (w, h) = display.get_framebuffer_dimensions();
                            (w as f32, h as f32)
                        };
                        let origin = point(0.0, 0.0);
                        let vertices: Vec<Vertex> = glyphs
                            .iter()
                                .filter_map(|g| cache.rect_for(0, g).ok().flatten())
                                .flat_map(|(uv_rect, screen_rect)| {
                                    let gl_rect = Rect {
                                        min: origin
                                            + (vector(
                                            screen_rect.min.x as f32 / screen_width - 0.5,
                                            1.0 - screen_rect.min.y as f32 / screen_height - 0.5,
                                        )) * 2.0,
                                        max: origin
                                            + (vector(
                                            screen_rect.max.x as f32 / screen_width - 0.5,
                                            1.0 - screen_rect.max.y as f32 / screen_height - 0.5,
                                        )) * 2.0,
                                    };
                                    vec![
                                        Vertex {
                                            position: [gl_rect.min.x, gl_rect.max.y],
                                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                                            colour,
                                        },
                                        Vertex {
                                            position: [gl_rect.min.x, gl_rect.min.y],
                                            tex_coords: [uv_rect.min.x, uv_rect.min.y],
                                            colour,
                                        },
                                        Vertex {
                                            position: [gl_rect.max.x, gl_rect.min.y],
                                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                                            colour,
                                        },
                                        Vertex {
                                            position: [gl_rect.max.x, gl_rect.min.y],
                                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                                            colour,
                                        },
                                        Vertex {
                                            position: [gl_rect.max.x, gl_rect.max.y],
                                            tex_coords: [uv_rect.max.x, uv_rect.max.y],
                                            colour,
                                        },
                                        Vertex {
                                            position: [gl_rect.min.x, gl_rect.max.y],
                                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                                            colour,
                                        },
                                    ]
                                })
                                .collect();

                        glium::VertexBuffer::new(&display, &vertices).unwrap()
                    };

                    let mut target = display.draw();
                    target.clear_color(1.0, 1.0, 1.0, 0.0);
                    target.draw(
                        &vertex_buffer,
                            glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                            &program,
                            &uniforms,
                            &glium::DrawParameters {
                            blend: glium::Blend::alpha_blending(),
                            ..Default::default()
                        },
                        ).unwrap();

                    target.finish().unwrap();
                }
                _ => (),
            }
        })
}
