use glfw::{Context, WindowHint};
use sciter::windowless::{handle_message, Message};

mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

fn main() {
    #[cfg(target_os = "linux")]
    sciter::set_library("libsciter.so").unwrap();
    #[cfg(target_os = "windows")]
    sciter::set_library("sciter.dll").unwrap();

    let startup = std::time::Instant::now();

    sciter::set_options(sciter::RuntimeOptions::UxTheming(true)).unwrap();
    sciter::set_options(sciter::RuntimeOptions::DebugMode(true)).unwrap();
    sciter::set_options(sciter::RuntimeOptions::ScriptFeatures(0xFF)).unwrap();

    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let window_width = 500;
    let window_height = 500;

    glfw.window_hint(WindowHint::Resizable(false));

    let (mut window, _) = glfw
        .create_window(
            window_width,
            window_height,
            "Sciter OpenGL window",
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create GLFW window");

    let window_handle = { &window.window_id() as *const _ as sciter::types::HWINDOW };

    handle_message(
        window_handle,
        Message::Create {
            backend: sciter::types::GFX_LAYER::SKIA_OPENGL,
            transparent: true,
        },
    );

    let instance = sciter::Host::attach(window_handle);

    let html = br#"
    <html>
      <head>
        <style>
          html {
            background: transparent;
          }
          * {
            padding: 0;
            margin: 0;
          }
          .small {
            font-size: 25px;
          }
          .big {
            font-size: 50px;
          }
          .white {
            color: white;
          }
        </style>
      </head>
      <body>
        <h1.small>Small Text</h1>
        <h1.big>Big Text</h1>
        <h1.small.white>ABCDEFGHIJKLMNOPQRSTUVWXYZ</h1>
      </body>
    </html>
    "#;
    instance.load_html(html, Some("example://index.htm"));

    glfw.window_hint(WindowHint::Visible(false));

    // Create context to render sciter with
    let (mut sciter_context, _) = window
        .create_shared(
            window_width,
            window_height,
            "",
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create GLFW window");

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    unsafe {
        gl::Ortho(
            0.0,
            window_width as f64,
            0.0,
            window_height as f64,
            0.0,
            1.0,
        );
    }

    handle_message(
        window_handle,
        Message::Size {
            width: window_width,
            height: window_height,
        },
    );

    // Initialize framebuffer for rendering in sciter context
    sciter_context.make_current();
    let sciter_frame_buffer = Framebuffer::create(window_width, window_width);
    window.make_current();

    while !window.should_close() {
        glfw.poll_events();

        // Draw actual game here, in this example just green color for demonstration
        unsafe {
            gl::ClearColor(0.0, 1.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        handle_message(
            window_handle,
            Message::Heartbit {
                milliseconds: std::time::Instant::now()
                    .duration_since(startup)
                    .as_millis() as u32,
            },
        );

        // Enter sciter context and render sciter to an opengl texture using a framebuffer
        sciter_context.make_current();
        sciter_frame_buffer.bind();
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        handle_message(window_handle, Message::Redraw);
        sciter_frame_buffer.unbind();
        window.make_current();


        // Now back in the window context, draw the texture containing sciter
        unsafe {
            gl::Enable(gl::TEXTURE_2D);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
            gl::BindTexture(gl::TEXTURE_2D, sciter_frame_buffer.texture());
            gl::Begin(gl::QUADS);
            gl::TexCoord2f(0.0, 0.0);
            gl::Vertex2f(0.0, 0.0);
            gl::TexCoord2f(0.0, 1.0);
            gl::Vertex2f(0.0, window_height as f32);
            gl::TexCoord2f(1.0, 1.0);
            gl::Vertex2f(window_width as f32, window_height as f32);
            gl::TexCoord2f(1.0, 0.0);
            gl::Vertex2f(window_width as f32, 0.0);
            gl::End();
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::TEXTURE_2D);
        }

        window.swap_buffers();
    }
}

struct Framebuffer {
    framebuffer: u32,
    texture: u32,
}

impl Framebuffer {
    fn create(width: u32, height: u32) -> Framebuffer {
        let mut framebuffer: u32 = 0;
        let mut texture: u32 = 0;

        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                texture,
                0,
            );
            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                panic!("Something went wrong creating framebuffer")
            }
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Framebuffer {
            framebuffer,
            texture,
        }
    }

    fn delete(&self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.framebuffer);
            gl::DeleteTextures(1, &self.texture);
        }
    }

    fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    fn texture(&self) -> u32 {
        self.texture
    }
}
