//! Graphical VDP client for Agon emulator.
//!
//! Connects to a running agon-ez80 instance and provides graphics/audio.

mod audio;
mod parse_args;
mod sdl2ps2;
mod vdp_interface;

use agon_protocol::{Message, ProtocolError, SocketAddr, SocketConnection, PROTOCOL_VERSION};
use parse_args::{parse_args, Verbosity};
use vdp_interface::VdpInterface;

use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3_sys::everything::{SDL_ScaleMode, SDL_SetTextureScaleMode, SDL_PixelFormat};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            std::process::exit(1);
        }
    };

    // Load VDP library
    let firmware_paths = if let Some(ref path) = args.vdp_path {
        vec![path.clone()]
    } else {
        vdp_interface::default_firmware_paths(&args.firmware)
    };

    let vdp = match vdp_interface::init(&firmware_paths, args.verbosity >= Verbosity::Verbose) {
        Some(v) => v,
        None => {
            eprintln!("Failed to load VDP firmware from any of: {:?}", firmware_paths);
            std::process::exit(1);
        }
    };

    // Initialize SDL first
    let sdl_context = sdl3::init().expect("Failed to init SDL");
    let video_subsystem = sdl_context.video().expect("Failed to init SDL video");
    let mut event_pump = sdl_context.event_pump().expect("Failed to get event pump");

    // Create window
    let mut window = video_subsystem
        .window("Agon VDP", 640, 480)
        .position_centered()
        .resizable()
        .build()
        .expect("Failed to create window");

    if args.fullscreen {
        let _ = window.set_fullscreen(true);
    }

    let mut canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            unsafe { sdl3::pixels::PixelFormat::from_ll(SDL_PixelFormat::RGB24) },
            1024,
            768,
        )
        .expect("Failed to create texture");

    unsafe {
        SDL_SetTextureScaleMode(texture.raw(), SDL_ScaleMode::NEAREST);
    }

    // Initialize audio
    let _audio_device = match (|| -> Result<_, sdl3::Error> {
        let audio_subsystem = sdl_context.audio()?;
        let desired_spec = sdl3::audio::AudioSpec {
            format: Some(sdl3::audio::AudioFormat::U8),
            freq: Some(16384),
            channels: Some(1),
        };
        let device = audio_subsystem.open_playback_device(&desired_spec)?;
        let stream = audio_subsystem.open_playback_stream_with_callback(
            &device,
            &desired_spec,
            audio::VdpAudioStream {
                buffer: vec![],
                getAudioSamples: vdp.getAudioSamples.clone(),
            },
        )?;
        stream.resume()?;
        Ok(stream)
    })() {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("Audio init error: {}", e);
            None
        }
    };

    // Start VDP thread BEFORE connecting
    let vdp_setup = vdp.vdp_setup.clone();
    let vdp_loop_fn = vdp.vdp_loop.clone();
    let _vdp_thread = std::thread::spawn(move || unsafe {
        (*vdp_setup)();
        (*vdp_loop_fn)();
    });

    // Warmup: render VDP while waiting for it to initialize
    eprintln!("Initializing VDP...");
    let mut vgabuf: Vec<u8> = vec![0u8; 1024 * 768 * 3];
    let mut mode_w: u32 = 640;
    let mut mode_h: u32 = 480;
    let mut frame_rate_hz: f32 = 60.0;

    for _ in 0..60 {  // ~1 second of warmup at 60fps
        // Process SDL events during warmup
        for event in event_pump.poll_iter() {
            if let Event::Quit { .. } = event {
                std::process::exit(0);
            }
        }

        // Signal vblank
        unsafe { (*vdp.signal_vblank)() };

        // Copy and render framebuffer
        unsafe {
            (*vdp.copyVgaFramebuffer)(
                &mut mode_w,
                &mut mode_h,
                vgabuf.as_mut_ptr(),
                &mut frame_rate_hz,
            );
        }

        if mode_w > 0 && mode_h > 0 {
            let pitch = mode_w as usize * 3;
            let _ = texture.update(
                sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                &vgabuf[..pitch * mode_h as usize],
                pitch,
            );
            let _ = canvas.clear();
            let _ = canvas.copy(&texture,
                sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                None);
            canvas.present();
        }

        std::thread::sleep(Duration::from_millis(16));
    }
    eprintln!("VDP ready");

    // Replay mode: feed VDU bytes from file instead of socket
    if let Some(ref replay_path) = args.replay {
        eprintln!("Replay mode: {}", replay_path.display());
        run_replay_session(&vdp, &args, &mut event_pump, &mut canvas, &mut texture);
        return;
    }

    // Determine socket address
    let addr = if let Some(tcp) = &args.tcp_addr {
        SocketAddr::tcp(tcp.clone())
    } else {
        let path = args
            .socket_path
            .clone()
            .unwrap_or_else(|| agon_protocol::socket::DEFAULT_SOCKET_PATH.to_string());
        #[cfg(unix)]
        {
            SocketAddr::unix(&path)
        }
        #[cfg(not(unix))]
        {
            eprintln!("Unix sockets not supported on this platform, use --tcp");
            std::process::exit(1);
        }
    };

    // Main connection loop - supports reconnection
    loop {
        eprintln!("Connecting to eZ80 at {}...", addr);

        match SocketConnection::connect(&addr) {
            Ok(conn) => {
                eprintln!("Connected!");
                if let Err(e) = run_session(conn, &vdp, &args, &mut event_pump, &mut canvas, &mut texture) {
                    eprintln!("Session error: {}", e);
                }
                eprintln!("Disconnected from eZ80, reconnecting...");
            }
            Err(e) => {
                eprintln!("Failed to connect: {} (retrying in 1s)", e);
            }
        }

        // Keep rendering during reconnect attempts
        for _ in 0..60 {  // ~1 second
            for event in event_pump.poll_iter() {
                if let Event::Quit { .. } = event {
                    std::process::exit(0);
                }
            }

            unsafe { (*vdp.signal_vblank)() };
            unsafe {
                (*vdp.copyVgaFramebuffer)(
                    &mut mode_w,
                    &mut mode_h,
                    vgabuf.as_mut_ptr(),
                    &mut frame_rate_hz,
                );
            }

            if mode_w > 0 && mode_h > 0 {
                let pitch = mode_w as usize * 3;
                let _ = texture.update(
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    &vgabuf[..pitch * mode_h as usize],
                    pitch,
                );
                let _ = canvas.clear();
                let _ = canvas.copy(&texture,
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    None);
                canvas.present();
            }

            std::thread::sleep(Duration::from_millis(16));
        }
    }
}

fn save_frame_png(dir: &str, frame_num: u64, buf: &[u8], w: u32, h: u32) {
    use std::fs;
    use std::io::BufWriter;
    use std::path::Path;

    let dir_path = Path::new(dir);
    if !dir_path.exists() {
        if let Err(e) = fs::create_dir_all(dir_path) {
            eprintln!("Failed to create dump directory {}: {}", dir, e);
            return;
        }
    }

    let filename = dir_path.join(format!("frame_{:06}.png", frame_num));
    let file = match fs::File::create(&filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create {}: {}", filename.display(), e);
            return;
        }
    };
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    match encoder.write_header() {
        Ok(mut png_writer) => {
            let row_bytes = w as usize * 3;
            if let Err(e) = png_writer.write_image_data(&buf[..row_bytes * h as usize]) {
                eprintln!("Failed to write PNG data: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to write PNG header: {}", e);
        }
    }
}

fn open_replay_log(path: &str) -> Box<dyn std::io::Write> {
    if path == "-" {
        Box::new(std::io::stderr())
    } else {
        match std::fs::File::create(path) {
            Ok(f) => Box::new(std::io::BufWriter::new(f)),
            Err(e) => {
                eprintln!("Failed to open replay log '{}': {}", path, e);
                std::process::exit(1);
            }
        }
    }
}

fn run_replay_session(
    vdp: &VdpInterface,
    args: &parse_args::AppArgs,
    event_pump: &mut sdl3::EventPump,
    canvas: &mut sdl3::render::Canvas<sdl3::video::Window>,
    texture: &mut sdl3::render::Texture,
) {
    use std::io::Read as _;
    use std::io::Write as _;

    let replay_path = args.replay.as_ref().unwrap();
    let file_data = match std::fs::read(replay_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read replay file '{}': {}", replay_path.display(), e);
            std::process::exit(1);
        }
    };

    let fps = args.replay_fps.unwrap_or(60.0);
    let vsync_interval = if fps > 0.0 {
        Some(Duration::from_secs_f64(1.0 / fps))
    } else {
        None // max speed
    };

    let mut log: Option<Box<dyn std::io::Write>> = args.replay_log.as_deref().map(open_replay_log);
    let start_time = Instant::now();

    let mut vgabuf: Vec<u8> = vec![0u8; 1024 * 768 * 3];
    let mut mode_w: u32 = 640;
    let mut mode_h: u32 = 480;
    let mut frame_rate_hz: f32 = 60.0;
    let mut vsync_count: u64 = 0;
    let mut dump_frame_num: u64 = 0;
    let mut last_vsync = Instant::now();
    let mut cursor = std::io::Cursor::new(&file_data);
    let mut eof = false;
    let mut eof_grace: u32 = 0; // vsyncs remaining after EOF before exit
    const EOF_GRACE_FRAMES: u32 = 120; // ~2 seconds at 60fps

    macro_rules! replay_log {
        ($log:expr, $start:expr, $($arg:tt)*) => {
            if let Some(ref mut w) = $log {
                let elapsed = $start.elapsed().as_secs_f64();
                let _ = write!(w, "[{:7.3}] ", elapsed);
                let _ = writeln!(w, $($arg)*);
            }
        }
    }

    loop {
        // Process SDL events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return,
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => return,
                _ => {}
            }
        }

        // Check vsync timing
        let do_vsync = match vsync_interval {
            Some(interval) => last_vsync.elapsed() >= interval,
            None => true,
        };

        if do_vsync && !eof {
            // Feed next chunk to VDP
            if args.replay_raw {
                // Raw mode: feed everything at once on first vsync
                if vsync_count == 0 {
                    for &byte in file_data.iter() {
                        unsafe { (*vdp.z80_send_to_vdp)(byte) };
                    }
                    replay_log!(log, start_time, "RAW: fed {} bytes", file_data.len());
                }
                eof = true;
            } else {
                // VSYNC-chunked: read [u16-LE length][data]
                let mut len_buf = [0u8; 2];
                match cursor.read_exact(&mut len_buf) {
                    Ok(()) => {
                        let chunk_len = u16::from_le_bytes(len_buf) as usize;
                        if chunk_len == 0 {
                            replay_log!(log, start_time, "EOF marker at byte {}", cursor.position());
                            eof = true;
                        } else {
                            let pos = cursor.position() as usize;
                            if pos + chunk_len > file_data.len() {
                                replay_log!(log, start_time, "WARN: truncated chunk at byte {}", pos);
                                eof = true;
                            } else {
                                for &byte in &file_data[pos..pos + chunk_len] {
                                    // Respect CTS flow control (VDP may be busy)
                                    let mut cts_waits = 0u32;
                                    while !unsafe { (*vdp.z80_uart0_is_cts)() } {
                                        cts_waits += 1;
                                        if cts_waits > 1000 {
                                            // VDP thread may need a vblank to make progress
                                            unsafe { (*vdp.signal_vblank)() };
                                            std::thread::sleep(Duration::from_micros(100));
                                            cts_waits = 0;
                                        } else {
                                            std::thread::yield_now();
                                        }
                                    }
                                    unsafe { (*vdp.z80_send_to_vdp)(byte) };
                                }
                                cursor.set_position((pos + chunk_len) as u64);
                                replay_log!(log, start_time, "CHUNK: {} bytes at frame {}", chunk_len, vsync_count);
                            }
                        }
                    }
                    Err(_) => {
                        replay_log!(log, start_time, "EOF (end of file)");
                        eof = true;
                    }
                }
            }

            // Signal vblank
            unsafe { (*vdp.signal_vblank)() };
            vsync_count += 1;
            replay_log!(log, start_time, "VSYNC #{}", vsync_count);

            // Drain VDP→eZ80 responses (discard, but log them)
            loop {
                let mut byte: u8 = 0;
                if unsafe { (*vdp.z80_recv_from_vdp)(&mut byte) } {
                    replay_log!(log, start_time, "VDP->eZ80: 0x{:02X}", byte);
                } else {
                    break;
                }
            }

            // Copy framebuffer
            unsafe {
                (*vdp.copyVgaFramebuffer)(
                    &mut mode_w,
                    &mut mode_h,
                    vgabuf.as_mut_ptr(),
                    &mut frame_rate_hz,
                );
            }

            // Dump frame if requested
            if mode_w > 0 && mode_h > 0 {
                if args.dump_frames.is_some() || args.dump_keyframes.is_some() {
                    dump_frame_num += 1;
                    if args.frame_spec.includes(dump_frame_num) {
                        let dir = args.dump_frames.as_deref()
                            .or(args.dump_keyframes.as_deref())
                            .unwrap();
                        save_frame_png(dir, dump_frame_num, &vgabuf, mode_w, mode_h);
                    }
                }
            }

            // Render
            if mode_w > 0 && mode_h > 0 {
                let pitch = mode_w as usize * 3;
                let _ = texture.update(
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    &vgabuf[..pitch * mode_h as usize],
                    pitch,
                );
                let _ = canvas.clear();
                let _ = canvas.copy(texture,
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    None);
                canvas.present();
            }

            last_vsync = last_vsync
                .checked_add(vsync_interval.unwrap_or(Duration::ZERO))
                .unwrap_or_else(Instant::now);
        } else if eof {
            // After EOF, continue signaling vsyncs for grace period
            // (lets VDP finish processing buffered commands / VSYNC callbacks)
            eof_grace += 1;
            if eof_grace > EOF_GRACE_FRAMES {
                replay_log!(log, start_time, "EOF grace period done ({} vsyncs), exiting", EOF_GRACE_FRAMES);
                return;
            }
            unsafe { (*vdp.signal_vblank)() };
            unsafe {
                (*vdp.copyVgaFramebuffer)(
                    &mut mode_w,
                    &mut mode_h,
                    vgabuf.as_mut_ptr(),
                    &mut frame_rate_hz,
                );
            }
            if mode_w > 0 && mode_h > 0 {
                let pitch = mode_w as usize * 3;
                let _ = texture.update(
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    &vgabuf[..pitch * mode_h as usize],
                    pitch,
                );
                let _ = canvas.clear();
                let _ = canvas.copy(texture,
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    None);
                canvas.present();
            }
            std::thread::sleep(Duration::from_millis(16));
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn run_session(
    mut conn: SocketConnection,
    vdp: &VdpInterface,
    args: &parse_args::AppArgs,
    event_pump: &mut sdl3::EventPump,
    canvas: &mut sdl3::render::Canvas<sdl3::video::Window>,
    texture: &mut sdl3::render::Texture,
) -> Result<(), ProtocolError> {
    // Perform handshake (as connector, we send HELLO first)
    let caps = r#"{"type":"sdl","width":640,"height":480,"audio":true}"#;
    if args.verbosity >= Verbosity::Verbose {
        eprintln!("[VDP] -> HELLO version={}, flags=0", PROTOCOL_VERSION);
    }
    conn.send(&Message::Hello {
        version: PROTOCOL_VERSION,
        flags: 0,
    })?;

    // Wait for HELLO_ACK
    let msg = conn.recv()?;
    match msg {
        Message::HelloAck { version, capabilities } => {
            if args.verbosity >= Verbosity::Verbose {
                eprintln!("[VDP] <- HELLO_ACK version={}, caps={}", version, capabilities);
            }
            eprintln!("eZ80 version {}, capabilities: {}", version, if capabilities.is_empty() { "(none)" } else { &capabilities });
        }
        _ => {
            return Err(ProtocolError::InvalidFormat("Expected HELLO_ACK".to_string()));
        }
    }
    eprintln!("Handshake complete");

    let shutdown = Arc::new(AtomicBool::new(false));

    // Split connection
    let (mut reader, mut writer) = conn.split();

    // Set up socket reader thread
    let (tx_from_ez80, rx_from_ez80): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let shutdown_reader = shutdown.clone();
    let _reader_thread = std::thread::spawn(move || {
        loop {
            if shutdown_reader.load(Ordering::Relaxed) {
                break;
            }
            match reader.recv() {
                Ok(msg) => {
                    if tx_from_ez80.send(msg).is_err() {
                        break;
                    }
                }
                Err(ProtocolError::ConnectionClosed) => break,
                Err(_) => break,
            }
        }
    });

    // Framebuffer
    let mut vgabuf: Vec<u8> = vec![0u8; 1024 * 768 * 3];
    let mut mode_w: u32 = 640;
    let mut mode_h: u32 = 480;
    let mut frame_rate_hz: f32 = 60.0;
    let mut mouse_btn_state: u8 = 0;

    // Main loop
    let mut last_vsync = Instant::now();
    let vsync_interval = Duration::from_micros(16666);
    let mut rctrl_pressed = false;
    let mut vsync_count: u64 = 0;
    let mut uart_had_activity = false;
    let mut dump_frame_num: u64 = 0;

    'running: loop {
        // Process SDL events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    shutdown.store(true, Ordering::Relaxed);
                    std::process::exit(0);
                }
                Event::KeyDown { scancode: Some(scancode), keycode, repeat: false, .. } => {
                    if scancode == sdl3::keyboard::Scancode::RCtrl {
                        rctrl_pressed = true;
                        continue;
                    }
                    if rctrl_pressed {
                        match keycode {
                            Some(Keycode::Q) => {
                                shutdown.store(true, Ordering::Relaxed);
                                std::process::exit(0);
                            }
                            Some(Keycode::M) => unsafe {
                                (*vdp.dump_vdp_mem_stats)();
                            }
                            _ => {}
                        }
                        continue;
                    }
                    let ps2 = sdl2ps2::sdl2ps2(scancode, false);
                    unsafe { (*vdp.sendPS2KbEventToFabgl)(ps2, 1) };
                }
                Event::KeyUp { scancode: Some(scancode), repeat: false, .. } => {
                    if scancode == sdl3::keyboard::Scancode::RCtrl {
                        rctrl_pressed = false;
                        continue;
                    }
                    let ps2 = sdl2ps2::sdl2ps2(scancode, false);
                    unsafe { (*vdp.sendPS2KbEventToFabgl)(ps2, 0) };
                }
                Event::MouseMotion { .. } => {
                    let packet: [u8; 4] = [0x08 | mouse_btn_state, 0, 0, 0];
                    unsafe { (*vdp.sendHostMouseEventToFabgl)(packet.as_ptr()) };
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    match mouse_btn {
                        sdl3::mouse::MouseButton::Left => mouse_btn_state |= 1,
                        sdl3::mouse::MouseButton::Right => mouse_btn_state |= 2,
                        sdl3::mouse::MouseButton::Middle => mouse_btn_state |= 4,
                        _ => {}
                    }
                    let packet: [u8; 4] = [0x08 | mouse_btn_state, 0, 0, 0];
                    unsafe { (*vdp.sendHostMouseEventToFabgl)(packet.as_ptr()) };
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    match mouse_btn {
                        sdl3::mouse::MouseButton::Left => mouse_btn_state &= !1,
                        sdl3::mouse::MouseButton::Right => mouse_btn_state &= !2,
                        sdl3::mouse::MouseButton::Middle => mouse_btn_state &= !4,
                        _ => {}
                    }
                    let packet: [u8; 4] = [0x08 | mouse_btn_state, 0, 0, 0];
                    unsafe { (*vdp.sendHostMouseEventToFabgl)(packet.as_ptr()) };
                }
                _ => {}
            }
        }

        // Process messages from eZ80
        while let Ok(msg) = rx_from_ez80.try_recv() {
            match msg {
                Message::UartData(data) => {
                    if args.verbosity >= Verbosity::Trace {
                        eprintln!("[VDP] <- UART ({} bytes)", data.len());
                    }
                    for byte in data {
                        unsafe { (*vdp.z80_send_to_vdp)(byte) };
                    }
                    uart_had_activity = true;
                }
                Message::Shutdown => {
                    if args.verbosity >= Verbosity::Verbose {
                        eprintln!("[VDP] <- SHUTDOWN");
                    }
                    shutdown.store(true, Ordering::Relaxed);
                    break 'running;
                }
                _ => {}
            }
        }

        // Collect data from VDP to send to eZ80
        let mut tx_bytes = Vec::new();
        loop {
            let mut byte: u8 = 0;
            if unsafe { (*vdp.z80_recv_from_vdp)(&mut byte) } {
                tx_bytes.push(byte);
            } else {
                break;
            }
        }
        if !tx_bytes.is_empty() {
            if args.verbosity >= Verbosity::Trace {
                eprintln!("[VDP] -> UART ({} bytes)", tx_bytes.len());
            }
            let _ = writer.send(&Message::UartData(tx_bytes));
        }

        // VSYNC and rendering
        if last_vsync.elapsed() >= vsync_interval {
            // Signal vblank to VDP
            unsafe { (*vdp.signal_vblank)() };

            // Send VSYNC to eZ80
            vsync_count += 1;
            if args.verbosity >= Verbosity::Trace && vsync_count % 60 == 0 {
                eprintln!("[VDP] VSYNC #{} (~{} seconds)", vsync_count, vsync_count / 60);
            }
            if let Err(e) = writer.send(&Message::Vsync) {
                eprintln!("[VDP] Failed to send VSYNC: {}", e);
                break 'running;
            }

            // Copy framebuffer
            unsafe {
                (*vdp.copyVgaFramebuffer)(
                    &mut mode_w,
                    &mut mode_h,
                    vgabuf.as_mut_ptr(),
                    &mut frame_rate_hz,
                );
            }

            // Dump frame if requested
            if mode_w > 0 && mode_h > 0 {
                let should_dump = args.dump_frames.is_some()
                    || (args.dump_keyframes.is_some() && uart_had_activity);
                if should_dump {
                    dump_frame_num += 1;
                    if args.frame_spec.includes(dump_frame_num) {
                        let dir = args.dump_frames.as_deref()
                            .or(args.dump_keyframes.as_deref())
                            .unwrap();
                        save_frame_png(dir, dump_frame_num, &vgabuf, mode_w, mode_h);
                    }
                }
                uart_had_activity = false;
            }

            // Update texture and render
            if mode_w > 0 && mode_h > 0 {
                let pitch = mode_w as usize * 3;
                let _ = texture.update(
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    &vgabuf[..pitch * mode_h as usize],
                    pitch,
                );

                let _ = canvas.clear();
                let _ = canvas.copy(texture,
                    sdl3::rect::Rect::new(0, 0, mode_w, mode_h),
                    None);
                canvas.present();
            }

            last_vsync = last_vsync
                .checked_add(vsync_interval)
                .unwrap_or_else(Instant::now);
        }

        // Small sleep
        std::thread::sleep(Duration::from_millis(1));
    }

    // Cleanup
    let _ = writer.send(&Message::Shutdown);
    Ok(())
}
