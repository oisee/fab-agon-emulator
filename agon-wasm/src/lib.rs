//! Agon eZ80 Emulator for WebAssembly
//!
//! A minimal eZ80 emulator that runs in the browser.

use wasm_bindgen::prelude::*;
use std::cell::Cell;
use std::collections::VecDeque;
use ez80::Reg16;

// Memory sizes
const EXTERNAL_RAM_SIZE: usize = 512 * 1024;
const ROM_SIZE: usize = 128 * 1024;
const ONCHIP_RAM_SIZE: usize = 8 * 1024;

// eZ80 I/O ports for UART0
const UART0_RBR_THR: u8 = 0xC0; // Receive/Transmit buffer
const UART0_IER: u8 = 0xC1;     // Interrupt enable
const UART0_IIR_FCR: u8 = 0xC2; // Interrupt ID / FIFO control
const UART0_LCR: u8 = 0xC3;     // Line control
const UART0_LSR: u8 = 0xC5;     // Line status

// UART LSR bits
const LSR_DR: u8 = 0x01;   // Data ready
const LSR_THRE: u8 = 0x20; // Transmit holding register empty
const LSR_TEMT: u8 = 0x40; // Transmitter empty

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

/// The machine state (memory, I/O) - separate from CPU for borrow checker
struct AgonMachine {
    mem_external: Vec<u8>,
    mem_rom: Vec<u8>,
    mem_internal: Vec<u8>,

    // UART state
    uart_rx_fifo: VecDeque<u8>,
    uart_tx_fifo: VecDeque<u8>,
    uart_ier: u8,
    uart_lcr: u8,

    // Cycle counter for timing
    cycle_counter: Cell<i32>,

    // GPIO for vsync
    gpio_b: u8,
}

impl AgonMachine {
    fn new() -> Self {
        AgonMachine {
            mem_external: vec![0; EXTERNAL_RAM_SIZE],
            mem_rom: vec![0; ROM_SIZE],
            mem_internal: vec![0; ONCHIP_RAM_SIZE],
            uart_rx_fifo: VecDeque::new(),
            uart_tx_fifo: VecDeque::new(),
            uart_ier: 0,
            uart_lcr: 0,
            cycle_counter: Cell::new(0),
            gpio_b: 0,
        }
    }
}

// Memory trait implementation for ez80 CPU
impl ez80::Machine for AgonMachine {
    fn peek(&self, addr: u32) -> u8 {
        let addr = addr as usize & 0xFFFFFF;

        if addr < ROM_SIZE {
            // ROM: 0x000000 - 0x01FFFF
            self.mem_rom[addr]
        } else if addr >= 0x040000 && addr < 0x040000 + EXTERNAL_RAM_SIZE {
            // External RAM: 0x040000 - 0x0BFFFF
            self.mem_external[addr - 0x040000]
        } else if addr >= 0x0BC000 && addr < 0x0BC000 + ONCHIP_RAM_SIZE {
            // Internal RAM: 0x0BC000 - 0x0BDFFF (mirrored at various addresses)
            self.mem_internal[addr - 0x0BC000]
        } else {
            0xFF
        }
    }

    fn poke(&mut self, addr: u32, value: u8) {
        let addr = addr as usize & 0xFFFFFF;

        if addr >= 0x040000 && addr < 0x040000 + EXTERNAL_RAM_SIZE {
            // External RAM
            self.mem_external[addr - 0x040000] = value;
        } else if addr >= 0x0BC000 && addr < 0x0BC000 + ONCHIP_RAM_SIZE {
            // Internal RAM
            self.mem_internal[addr - 0x0BC000] = value;
        }
        // ROM writes are ignored
    }

    fn port_in(&mut self, port: u16) -> u8 {
        let port_lo = (port & 0xFF) as u8;

        match port_lo {
            UART0_RBR_THR => {
                // Read from UART receive buffer
                self.uart_rx_fifo.pop_front().unwrap_or(0)
            }
            UART0_IER => self.uart_ier,
            UART0_IIR_FCR => 0x01, // No interrupt pending
            UART0_LCR => self.uart_lcr,
            UART0_LSR => {
                // Line status: check if data ready and transmit empty
                let mut status = LSR_THRE | LSR_TEMT; // TX always ready
                if !self.uart_rx_fifo.is_empty() {
                    status |= LSR_DR; // Data ready
                }
                status
            }
            // GPIO Port B
            0x9A => self.gpio_b,
            _ => 0xFF,
        }
    }

    fn port_out(&mut self, port: u16, value: u8) {
        let port_lo = (port & 0xFF) as u8;

        match port_lo {
            UART0_RBR_THR => {
                // Write to UART transmit buffer
                self.uart_tx_fifo.push_back(value);
            }
            UART0_IER => self.uart_ier = value,
            UART0_LCR => self.uart_lcr = value,
            // GPIO Port B
            0x9A => self.gpio_b = value,
            _ => {}
        }
    }

    fn use_cycles(&self, cycles: i32) {
        self.cycle_counter.set(self.cycle_counter.get() + cycles);
    }
}

/// The WASM Agon Emulator
#[wasm_bindgen]
pub struct AgonEmulator {
    cpu: ez80::Cpu,
    machine: AgonMachine,
    total_cycles: u64,
    vsync_cycles: u64,
}

#[wasm_bindgen]
impl AgonEmulator {
    /// Create a new emulator instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> AgonEmulator {
        console_log!("Creating Agon WASM Emulator");

        let mut cpu = ez80::Cpu::new();

        // Initialize CPU state
        cpu.state.set_pc(0x000000);
        cpu.state.reg.set24(Reg16::SP, 0x0BFFFF); // Stack in RAM
        cpu.state.reg.adl = true; // 24-bit mode

        AgonEmulator {
            cpu,
            machine: AgonMachine::new(),
            total_cycles: 0,
            vsync_cycles: 0,
        }
    }

    /// Load MOS firmware into ROM
    #[wasm_bindgen]
    pub fn load_mos(&mut self, data: &[u8]) {
        console_log!("Loading MOS firmware: {} bytes", data.len());
        let len = data.len().min(ROM_SIZE);
        self.machine.mem_rom[..len].copy_from_slice(&data[..len]);
    }

    /// Run a number of CPU cycles
    /// Returns the number of cycles actually executed
    #[wasm_bindgen]
    pub fn run_cycles(&mut self, max_cycles: u32) -> u32 {
        let start_cycles = self.total_cycles;
        self.machine.cycle_counter.set(0);

        while self.machine.cycle_counter.get() < max_cycles as i32 {
            // Execute one instruction
            self.cpu.fast_execute_instruction(&mut self.machine);

            // Check for vsync (every ~307,200 cycles at 18.432 MHz = 60 Hz)
            let cycles_now = self.total_cycles + self.machine.cycle_counter.get() as u64;
            if cycles_now >= self.vsync_cycles + 307200 {
                self.vsync_cycles = cycles_now;
                // Pulse GPIO B pin 1 for vsync
                self.machine.gpio_b |= 0x02;
                self.machine.gpio_b &= !0x02;
            }
        }

        let executed = self.machine.cycle_counter.get() as u64;
        self.total_cycles += executed;
        (self.total_cycles - start_cycles) as u32
    }

    /// Send a byte to the emulator (from VDP)
    #[wasm_bindgen]
    pub fn send_byte(&mut self, byte: u8) {
        self.machine.uart_rx_fifo.push_back(byte);
    }

    /// Send keyboard input (VDP key packet format)
    #[wasm_bindgen]
    pub fn send_key(&mut self, ascii: u8, down: bool) {
        // VDP key packet: 0x81, len, ascii, modifiers, vkey, down
        self.machine.uart_rx_fifo.push_back(0x81);
        self.machine.uart_rx_fifo.push_back(4);
        self.machine.uart_rx_fifo.push_back(ascii);
        self.machine.uart_rx_fifo.push_back(0); // modifiers
        self.machine.uart_rx_fifo.push_back(0); // vkey
        self.machine.uart_rx_fifo.push_back(if down { 1 } else { 0 });
    }

    /// Get pending output bytes (to VDP)
    #[wasm_bindgen]
    pub fn get_output(&mut self) -> Vec<u8> {
        self.machine.uart_tx_fifo.drain(..).collect()
    }

    /// Check if there's pending output
    #[wasm_bindgen]
    pub fn has_output(&self) -> bool {
        !self.machine.uart_tx_fifo.is_empty()
    }

    /// Get total cycles executed
    #[wasm_bindgen]
    pub fn get_cycles(&self) -> u64 {
        self.total_cycles
    }

    /// Reset the emulator
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.cpu.state.set_pc(0x000000);
        self.cpu.state.reg.set24(Reg16::SP, 0x0BFFFF); // Stack in RAM
        self.machine.uart_rx_fifo.clear();
        self.machine.uart_tx_fifo.clear();
        self.total_cycles = 0;
        self.vsync_cycles = 0;
        console_log!("Emulator reset");
    }
}

impl Default for AgonEmulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
