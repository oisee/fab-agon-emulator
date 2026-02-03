//! VDP library interface - loads and provides access to VDP .so functions.

use std::path::Path;

#[allow(non_snake_case)]
pub struct VdpInterface {
    pub vdp_setup: libloading::Symbol<'static, unsafe extern "C" fn()>,
    pub vdp_loop: libloading::Symbol<'static, unsafe extern "C" fn()>,
    pub signal_vblank: libloading::Symbol<'static, unsafe extern "C" fn()>,
    pub copyVgaFramebuffer: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            outWidth: *mut u32,
            outHeight: *mut u32,
            buffer: *mut u8,
            frameRateHz: *mut f32,
        ),
    >,
    pub set_startup_screen_mode: libloading::Symbol<'static, unsafe extern "C" fn(m: u32)>,
    pub z80_uart0_is_cts: libloading::Symbol<'static, unsafe extern "C" fn() -> bool>,
    pub z80_send_to_vdp: libloading::Symbol<'static, unsafe extern "C" fn(b: u8)>,
    pub z80_recv_from_vdp: libloading::Symbol<'static, unsafe extern "C" fn(out: *mut u8) -> bool>,
    pub sendVKeyEventToFabgl: libloading::Symbol<'static, unsafe extern "C" fn(vkey: u32, isDown: u8)>,
    pub sendPS2KbEventToFabgl: libloading::Symbol<'static, unsafe extern "C" fn(ps2scancode: u16, isDown: u8)>,
    pub sendHostMouseEventToFabgl: libloading::Symbol<'static, unsafe extern "C" fn(mouse_packet: *const u8)>,
    pub setVdpDebugLogging: libloading::Symbol<'static, unsafe extern "C" fn(state: bool)>,
    pub getAudioSamples: libloading::Symbol<'static, unsafe extern "C" fn(out: *mut u8, length: u32)>,
    pub dump_vdp_mem_stats: libloading::Symbol<'static, unsafe extern "C" fn()>,
    pub vdp_shutdown: libloading::Symbol<'static, unsafe extern "C" fn()>,
}

static mut VDP_DLL: *const libloading::Library = std::ptr::null();

impl VdpInterface {
    fn new(lib: &'static libloading::Library) -> Self {
        unsafe {
            VdpInterface {
                vdp_setup: lib.get(b"vdp_setup").unwrap(),
                vdp_loop: lib.get(b"vdp_loop").unwrap(),
                signal_vblank: lib.get(b"signal_vblank").unwrap(),
                copyVgaFramebuffer: lib.get(b"copyVgaFramebuffer").unwrap(),
                z80_uart0_is_cts: lib.get(b"z80_uart0_is_cts").unwrap(),
                z80_send_to_vdp: lib.get(b"z80_send_to_vdp").unwrap(),
                z80_recv_from_vdp: lib.get(b"z80_recv_from_vdp").unwrap(),
                set_startup_screen_mode: lib.get(b"set_startup_screen_mode").unwrap(),
                sendVKeyEventToFabgl: lib.get(b"sendVKeyEventToFabgl").unwrap(),
                sendPS2KbEventToFabgl: lib.get(b"sendPS2KbEventToFabgl").unwrap(),
                sendHostMouseEventToFabgl: lib.get(b"sendHostMouseEventToFabgl").unwrap(),
                setVdpDebugLogging: lib.get(b"setVdpDebugLogging").unwrap(),
                getAudioSamples: lib.get(b"getAudioSamples").unwrap(),
                dump_vdp_mem_stats: lib.get(b"dump_vdp_mem_stats").unwrap(),
                vdp_shutdown: lib.get(b"vdp_shutdown").unwrap(),
            }
        }
    }
}

/// Load VDP library from given paths (tries each until one succeeds)
pub fn init(firmware_paths: &[std::path::PathBuf], verbose: bool) -> Option<VdpInterface> {
    assert!(unsafe { VDP_DLL.is_null() });

    if verbose {
        eprintln!("VDP firmware search paths: {:?}", firmware_paths);
    }

    for p in firmware_paths {
        if verbose {
            eprintln!("Trying to load VDP: {:?}", p);
        }
        match unsafe { libloading::Library::new(p) } {
            Ok(lib) => {
                eprintln!("Loaded VDP firmware: {:?}", p);
                unsafe {
                    VDP_DLL = Box::leak(Box::new(lib));
                }
                return Some(VdpInterface::new(unsafe { VDP_DLL.as_ref() }.unwrap()));
            }
            Err(e) => {
                if verbose {
                    eprintln!("  Error: {:?}", e);
                }
            }
        }
    }
    None
}

/// Get default firmware paths for a given firmware version
pub fn default_firmware_paths(firmware: &str) -> Vec<std::path::PathBuf> {
    let prefix = option_env!("PREFIX");

    let base_path = match prefix {
        None => std::path::Path::new(".").join("firmware"),
        Some(p) => Path::new(p).join("share").join("fab-agon-emulator"),
    };

    let mut paths = Vec::new();

    // Try requested firmware first
    paths.push(base_path.join(format!("vdp_{}.so", firmware)));

    // Fall back to console8
    if firmware != "console8" {
        paths.push(base_path.join("vdp_console8.so"));
    }

    paths
}
