use intel8080::{Bus, CPU, MEM_SIZE, RATE};
use pixels::{Pixels, SurfaceTexture};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::{File, read};
use std::io::{BufReader, Cursor};
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const FPS: u32 = 60;
const XSCALE: f64 = 2.5;
const YSCALE: f64 = 2.5;
const WIDTH: u32 = 224;
const HEIGHT: u32 = 256;

// Key Bindings
const LEFT1: KeyCode = KeyCode::ArrowLeft;
const LEFT2: KeyCode = KeyCode::KeyA;
const RIGHT1: KeyCode = KeyCode::ArrowRight;
const RIGHT2: KeyCode = KeyCode::KeyD;
const SHOOT1: KeyCode = KeyCode::ArrowUp;
const SHOOT2: KeyCode = KeyCode::KeyW;
const START1: KeyCode = KeyCode::Digit1;
const START2: KeyCode = KeyCode::Digit2;
const TILT: KeyCode = KeyCode::Space;
const SPEED: KeyCode = KeyCode::Tab;

fn main() -> Result<(), Error> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let (_stream, stream_handle) = OutputStream::try_default()?;

    let _main_theme = {
        let buf = BufReader::new(File::open("games/invaders/audio/spaceinvaders1.mpeg")?);
        let dec = Decoder::new_looped(buf)?;
        let sink = Sink::try_new(&stream_handle)?;

        sink.append(dec);
        sink.set_volume(0.5);
        sink.play();

        sink
    };

    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64 * XSCALE, HEIGHT as f64 * YSCALE);
        WindowBuilder::new()
            .with_title("Space Invaders")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)?
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(
            window_size.width * (XSCALE as u32),
            window_size.height * (YSCALE as u32),
            &window,
        );
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut cpu = load_rom();
    let mut controls = Invaders::new(&stream_handle)?;
    let mut speed = 5;
    let mut count = 0;
    let mut interrupt = 0xcf;

    let res = event_loop.run(|event, elwt| {
        let cpf = RATE / FPS;
        let mut cycles = 0;

        while cycles < speed * (RATE / 1000) {
            let spent = cpu.cycle(&mut controls) as u32;
            cycles += spent;
            count += spent;

            if count >= cpf / 2 {
                count -= cpf / 2;

                cpu.interrupt(interrupt);

                if interrupt != 0xcf {
                    window.request_redraw();
                }

                interrupt = if interrupt == 0xcf { 0xd7 } else { 0xcf };
            }
        }

        // Draw the current frame
        if let Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } = event
        {
            let display = &cpu.memory()[0x2400..=0x3fff];
            map_display(display, pixels.frame_mut());

            if let Err(_) = pixels.render() {
                elwt.exit();
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(KeyCode::Escape) || input.close_requested() {
                elwt.exit();
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(_) = pixels.resize_surface(size.width, size.height) {
                    elwt.exit();
                    return;
                }
            }

            if input.key_pressed(KeyCode::KeyC) {
                controls.input1 |= 1;
            } else {
                controls.input1 &= 0b1111_1110;
            }

            if input.key_pressed(START1) {
                controls.input1 |= 1 << 2;
            } else {
                controls.input1 &= 0b1111_1011;
            }

            if input.key_pressed(START2) {
                controls.input1 |= 1 << 1;
            } else {
                controls.input1 &= 0b1111_1101;
            }

            if input.key_pressed(LEFT1) || input.key_held(LEFT1) {
                controls.input1 |= 1 << 5;
            } else {
                controls.input1 &= 0b1101_1111;
            }

            if input.key_pressed(LEFT2) || input.key_held(LEFT2) {
                controls.input2 |= 1 << 5;
            } else {
                controls.input2 &= 0b1101_1111;
            }

            if input.key_pressed(RIGHT1) || input.key_held(RIGHT1) {
                controls.input1 |= 1 << 6;
            } else {
                controls.input1 &= 0b1011_1111;
            }

            if input.key_pressed(RIGHT2) || input.key_held(RIGHT2) {
                controls.input2 |= 1 << 6;
            } else {
                controls.input2 &= 0b1011_1111;
            }

            if input.key_pressed(SHOOT1) || input.key_held(SHOOT1) {
                controls.input1 |= 1 << 4;
            } else {
                controls.input1 &= 0b1110_1111;
            }

            if input.key_pressed(SHOOT2) || input.key_held(SHOOT2) {
                controls.input2 |= 1 << 4;
            } else {
                controls.input2 &= 0b1110_1111;
            }

            if input.key_pressed(TILT) || input.key_held(TILT) {
                controls.input2 |= 1 << 2;
            } else {
                controls.input2 &= 0b1111_1011;
            }

            if input.key_pressed(SPEED) && !input.held_shift() {
                speed = (speed + 1).min(10);
            }

            if input.key_pressed(SPEED) && input.held_shift() {
                speed = (speed - 1).max(3);
            }

            window.request_redraw();
        }
    });

    res.map_err(|e| Error::Pixels(pixels::Error::UserDefined(Box::new(e))))
}

fn load_rom() -> CPU {
    let mut program = [0u8; MEM_SIZE];
    let f1 = include_bytes!("../invaders/invaders.h");
    program[0..0x0800].copy_from_slice(f1);

    let f2 = include_bytes!("../invaders/invaders.g");
    program[0x0800..0x1000].copy_from_slice(f2);

    let f3 = include_bytes!("../invaders/invaders.f");
    program[0x1000..0x1800].copy_from_slice(f3);

    let f4 = include_bytes!("../invaders/invaders.e");
    program[0x1800..0x2000].copy_from_slice(f4);

    CPU::new(&program)
}

fn load_audio<'a>(handle: &'a OutputStreamHandle) -> Result<[SFX<'a>; 9], Error> {
    Ok([
        SFX::new(read("games/invaders/audio/ufo_lowpitch.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/shoot.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/explosion.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/invaderkilled.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/fastinvader1.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/fastinvader2.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/fastinvader3.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/fastinvader4.wav")?, handle)?,
        SFX::new(read("games/invaders/audio/ufo_highpitch.wav")?, handle)?,
    ])
}

fn map_display(memory: &[u8], pixels: &mut [u8]) {
    let colors = |value: u8, x: usize, y: usize| {
        let mut r = 0;
        let mut g = 0;
        let mut b = 0;

        let color = 0x80;
        if value == 1 {
            if x < 16 {
                if y < 16 || y > 118 + 16 {
                    r = color;
                    g = color;
                    b = color;
                } else {
                    g = color;
                }
            } else if (x >= 16) && (x <= 16 + 56) {
                g = color;
            } else if (x >= 16 + 56 + 120) && (x < 16 + 56 + 120 + 32) {
                r = color;
            } else {
                r = color;
                g = color;
                b = color;
            }
        }
        return [r, g, b, 0xff];
    };

    for (byte, packed) in memory.iter().enumerate() {
        let x = (byte * 8) % 256;
        let y = (byte * 8) / 256;

        for bit in 0..8 {
            let value = (packed >> bit) & 1;
            let x = x + bit;
            let colors = colors(value, x, y);

            let temp = x;
            let x = y;
            let y = HEIGHT as usize - 1 - temp;

            let pixel = (WIDTH as usize * y) + x;
            let pixel = pixel * 4;

            pixels[pixel..=pixel + 3].copy_from_slice(&colors);
        }
    }
}

struct Invaders<'a> {
    ///Port 1
    ///bit 0 = CREDIT (1 if deposit)
    ///bit 1 = 2P start (1 if pressed)
    ///bit 2 = 1P start (1 if pressed)
    ///bit 3 = Always 1
    ///bit 4 = 1P shot (1 if pressed)
    ///bit 5 = 1P left (1 if pressed)
    ///bit 6 = 1P right (1 if pressed)
    ///bit 7 = Not connected
    input1: u8,

    ///Port 2
    ///bit 0 = DIP3 00 = 3 ships  10 = 5 ships
    ///bit 1 = DIP5 01 = 4 ships  11 = 6 ships
    ///bit 2 = Tilt
    ///bit 3 = DIP6 0 = extra ship at 1500, 1 = extra ship at 1000
    ///bit 4 = P2 shot (1 if pressed)
    ///bit 5 = P2 left (1 if pressed)
    ///bit 6 = P2 right (1 if pressed)
    ///bit 7 = DIP7 Coin info displayed in demo screen 0=ON
    ///
    ///DIP3-6: Number of starting lives
    input2: u8,

    shift_data: u16,
    shift_offset: u8,

    sfx: [SFX<'a>; 9],
    port3: u8,
    port5: u8,
}

impl<'a> Invaders<'a> {
    fn new(handle: &'a OutputStreamHandle) -> Result<Invaders<'a>, Error> {
        let sfx = load_audio(handle)?;

        Ok(Self {
            shift_data: 0,
            input1: 0b0000_1000,
            input2: 0b0000_0001,
            shift_offset: 0,
            port3: 0,
            port5: 0,
            sfx,
        })
    }
}

impl<'a> Bus for Invaders<'a> {
    fn read(&mut self, _cpu: &CPU, port: u8) -> u8 {
        match port {
            0 => {
                // Not used by code
                //bit 0 DIP4 (Seems to be self-test-request read at power up)
                //bit 1 Always 1
                //bit 2 Always 1
                //bit 3 Always 1
                //bit 4 Fire
                //bit 5 Left
                //bit 6 Right
                //bit 7 ? tied to demux port 7 ?

                0b0111_1110
            }
            1 => self.input1,
            2 => self.input2,
            3 => {
                let value = (self.shift_data >> (8 - self.shift_offset)) & 0xff;
                value as u8
            }
            unknown => {
                panic!("Read at unknown port: 0x{unknown:02x}")
            }
        }
    }

    fn write(&mut self, _cpu: &CPU, port: u8, data: u8) {
        match port {
            2 => {
                self.shift_offset = data & 7;
            }
            4 => {
                let data = (data as u16) << 8;
                self.shift_data = (self.shift_data >> 8) | data;
            }
            3 => {
                if data == self.port3 {
                    return;
                }

                if (data & 1 != 0) && (self.port3 & 1 == 0) {
                    if let Err(error) = self.sfx[0].play() {
                        eprintln!("Error when playing sfx 0");
                        eprintln!("{error}");
                    }
                }

                if (data & 0b10 != 0) && (self.port3 & 0b10 == 0) {
                    if let Err(error) = self.sfx[1].play() {
                        eprintln!("Error when playing sfx 1");
                        eprintln!("{error}");
                    }
                }

                if (data & 0b100 != 0) && (self.port3 & 0b100 == 0) {
                    if let Err(error) = self.sfx[2].play() {
                        eprintln!("Error when playing sfx 2");
                        eprintln!("{error}");
                    }
                }

                if (data & 0b1000 != 0) && (self.port3 & 0b1000 == 0) {
                    if let Err(error) = self.sfx[3].play() {
                        eprintln!("Error when playing sfx 3");
                        eprintln!("{error}");
                    }
                }

                self.port3 = data;
            }
            5 => {
                if (data & 1 != 0) && (self.port5 & 1 == 0) {
                    if let Err(error) = self.sfx[4].play() {
                        eprintln!("Error when playing sfx 4");
                        eprintln!("{error}");
                    }
                }

                if (data & 0b10 != 0) && (self.port5 & 0b10 == 0) {
                    if let Err(error) = self.sfx[5].play() {
                        eprintln!("Error when playing sfx 5");
                        eprintln!("{error}");
                    }
                }

                if (data & 0b100 != 0) && (self.port5 & 0b100 == 0) {
                    if let Err(error) = self.sfx[6].play() {
                        eprintln!("Error when playing sfx 6");
                        eprintln!("{error}");
                    }
                }

                if (data & 0b1000 != 0) && (self.port5 & 0b1000 == 0) {
                    if let Err(error) = self.sfx[7].play() {
                        eprintln!("Error when playing sfx 7");
                        eprintln!("{error}");
                    }
                }

                if (data & 0b10000 != 0) && (self.port5 & 0b10000 == 0) {
                    if let Err(error) = self.sfx[8].play() {
                        eprintln!("Error when playing sfx 8");
                        eprintln!("{error}");
                    }
                }

                self.port5 = data;
            }
            //Watchdog ... read or write to reset
            6 => {}
            unknown => {
                panic!("Write at unknown port: 0x{unknown:02x}")
            }
        }
    }
}

struct SFX<'a> {
    data: Noah,
    sink: Option<Sink>,
    handle: &'a OutputStreamHandle,
}

impl<'a> SFX<'a> {
    fn new(data: Vec<u8>, handle: &'a OutputStreamHandle) -> Result<SFX<'a>, Error> {
        Ok(Self {
            data: Noah(Arc::new(data)),
            sink: None,
            handle,
        })
    }

    fn play(&mut self) -> Result<(), Error> {
        if self
            .sink
            .as_ref()
            .map(|sink| !sink.empty())
            .unwrap_or_default()
        {
            return Ok(());
        }

        let sink = Sink::try_new(self.handle)?;
        let cursor = Cursor::new(self.data.clone());
        let dec = Decoder::new_wav(cursor)?;

        sink.append(dec);
        self.sink = Some(sink);

        Ok(())
    }
}

struct Noah(Arc<Vec<u8>>);

impl Clone for Noah {
    fn clone(&self) -> Self {
        let clone = self.0.clone();
        Self(clone)
    }
}

impl AsRef<[u8]> for Noah {
    fn as_ref(&self) -> &[u8] {
        (*self.0).as_ref()
    }
}

#[derive(Debug)]
pub enum Error {
    File(std::io::Error),
    Pixels(pixels::Error),
    AudioDevice(rodio::DevicesError),
    Decoder(rodio::decoder::DecoderError),
    Play(rodio::PlayError),
    Stream(rodio::StreamError),
    OS(OsError),
    EventLoop(EventLoopError),
}

impl From<rodio::DevicesError> for Error {
    fn from(value: rodio::DevicesError) -> Self {
        Self::AudioDevice(value)
    }
}

impl From<rodio::PlayError> for Error {
    fn from(value: rodio::PlayError) -> Self {
        Self::Play(value)
    }
}

impl From<rodio::StreamError> for Error {
    fn from(value: rodio::StreamError) -> Self {
        Self::Stream(value)
    }
}

impl From<pixels::Error> for Error {
    fn from(value: pixels::Error) -> Self {
        Self::Pixels(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::File(value)
    }
}

impl From<rodio::decoder::DecoderError> for Error {
    fn from(value: rodio::decoder::DecoderError) -> Self {
        Self::Decoder(value)
    }
}

impl From<OsError> for Error {
    fn from(value: OsError) -> Self {
        Self::OS(value)
    }
}

impl From<EventLoopError> for Error {
    fn from(value: EventLoopError) -> Self {
        Self::EventLoop(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pixels(error) => error.fmt(f),
            Self::AudioDevice(error) => error.fmt(f),
            Self::Play(error) => error.fmt(f),
            Self::Stream(error) => error.fmt(f),
            Self::File(error) => error.fmt(f),
            Self::Decoder(error) => error.fmt(f),
            Self::OS(error) => error.fmt(f),
            Self::EventLoop(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pixels(error) => Some(error),
            Self::AudioDevice(error) => Some(error),
            Self::Play(error) => Some(error),
            Self::Stream(error) => Some(error),
            Self::File(error) => Some(error),
            Self::Decoder(error) => Some(error),
            Self::OS(error) => Some(error),
            Self::EventLoop(error) => Some(error),
        }
    }
}
