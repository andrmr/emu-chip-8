
use std::collections::HashMap;

use pixels::{Pixels, SurfaceTexture, wgpu::Color};
use winit::event::ElementState;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use clap::Parser;

mod chip8;
use chip8::*;

#[derive(Parser)]
#[command(about="Chip-8 Emulator")]
struct Cli {
    #[arg(short, long, value_name="FILE", help="ROM file")]
    rom: String,
}

fn main() {
    let cli = Cli::parse();
    let rom = cli.rom;

    let mut chip8 = Chip8::new();
    chip8.load_rom(rom.as_str())
        .expect("TODO: panic message");

    // CHIP-8 key mapping
    // |1|2|3|C| => |1|2|3|4|
    // |4|5|6|D| =>	|Q|W|E|R|
    // |7|8|9|E| =>	|A|S|D|F|
    // |A|0|B|F| =>	|Z|X|C|V|
    let key_mapping = HashMap::from([
        (VirtualKeyCode::Key1, 0x1),
        (VirtualKeyCode::Key2, 0x2),
        (VirtualKeyCode::Key3, 0x3),
        (VirtualKeyCode::Key4, 0xC),
        (VirtualKeyCode::Q, 0x4),
        (VirtualKeyCode::W, 0x5),
        (VirtualKeyCode::E, 0x6),
        (VirtualKeyCode::R, 0xD),
        (VirtualKeyCode::A, 0x7),
        (VirtualKeyCode::S, 0x8),
        (VirtualKeyCode::D, 0x9),
        (VirtualKeyCode::F, 0xE),
        (VirtualKeyCode::Z, 0xA),
        (VirtualKeyCode::X, 0x0),
        (VirtualKeyCode::C, 0xB),
        (VirtualKeyCode::V, 0xF),
    ]);

    let event_loop = EventLoop::new();

    let window = {
        let size = LogicalSize::new(SCREEN_WIDTH, SCREEN_HEIGHT);
        let scaled_size = LogicalSize::new(SCREEN_WIDTH as f64 * 10., SCREEN_HEIGHT as f64 * 10.);
        WindowBuilder::new()
            .with_title("Chip-8 Emulator")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .with_max_inner_size(scaled_size)
            .build(&event_loop)
            .expect("Failed to create window")
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);

        Pixels::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, surface_texture)
            .expect("Failed to create pixels")
    };

    pixels.set_clear_color(Color::BLACK);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(key) = input.virtual_keycode {
                        if let Some(chip8_key) = key_mapping.get(&key) {
                            if input.state == ElementState::Released {
                                chip8.key = 0;
                            } else {
                                chip8.key = *chip8_key;
                            }
                        }
                    }
                },

                _ => (),
            },

            Event::MainEventsCleared => {                
                let frame = pixels.get_frame_mut();
                for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                    let x = i % SCREEN_WIDTH as usize;
                    let y = i / SCREEN_WIDTH as usize;

                    if chip8.display[y][x] {
                        pixel.copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
                    } else {
                        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xFF]);
                    }
                }
                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                chip8.handle_op();
            },

            _ => (),
        }
    });
}
