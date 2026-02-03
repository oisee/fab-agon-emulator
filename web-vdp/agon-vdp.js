// Agon VDP WebSocket Client
// Implements the agon-protocol for browser-based VDP

const PROTOCOL_VERSION = 1;

// Message types
const MSG_UART_DATA = 0x01;
const MSG_VSYNC = 0x02;
const MSG_CTS = 0x03;
const MSG_HELLO = 0x10;
const MSG_HELLO_ACK = 0x11;
const MSG_SHUTDOWN = 0x20;

class AgonVDP {
    constructor(terminal) {
        this.terminal = terminal;
        this.ws = null;
        this.connected = false;
        this.vsyncInterval = null;
        this.pendingInput = [];
        this.inputInterval = null;

        // VDU command parser state
        this.vduBuffer = [];
        this.vduExpected = 0;
        this.inVduSequence = false;

        // Cursor position (for VDP queries)
        this.cursorX = 0;
        this.cursorY = 0;
    }

    // Encode a message to binary format: [len:u16-LE][type:u8][payload...]
    encodeMessage(type, payload = []) {
        const len = 1 + payload.length;
        const buffer = new ArrayBuffer(2 + len);
        const view = new DataView(buffer);
        view.setUint16(0, len, true); // little-endian
        view.setUint8(2, type);
        for (let i = 0; i < payload.length; i++) {
            view.setUint8(3 + i, payload[i]);
        }
        return buffer;
    }

    // Decode a message from binary format
    decodeMessage(data) {
        const view = new DataView(data);
        if (data.byteLength < 3) {
            throw new Error('Message too short');
        }
        const len = view.getUint16(0, true);
        const type = view.getUint8(2);
        const payload = new Uint8Array(data, 3, len - 1);
        return { type, payload };
    }

    connect(url) {
        if (this.ws) {
            this.disconnect();
        }

        this.updateStatus('connecting', 'Connecting...');
        this.ws = new WebSocket(url);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = () => {
            console.log('WebSocket connected, sending HELLO');
            this.sendHello();
        };

        this.ws.onmessage = (event) => {
            this.handleMessage(event.data);
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            this.updateStatus('disconnected', 'Error');
        };

        this.ws.onclose = () => {
            console.log('WebSocket closed');
            this.connected = false;
            this.stopVsync();
            this.stopInputProcessing();
            this.updateStatus('disconnected', 'Disconnected');
            updateButtons(false);
        };
    }

    disconnect() {
        if (this.ws) {
            // Send shutdown message
            this.ws.send(this.encodeMessage(MSG_SHUTDOWN));
            this.ws.close();
            this.ws = null;
        }
        this.connected = false;
        this.stopVsync();
        this.stopInputProcessing();
    }

    sendHello() {
        // HELLO: version:u8, flags:u8
        const payload = [PROTOCOL_VERSION, 0];
        this.ws.send(this.encodeMessage(MSG_HELLO, payload));
    }

    handleMessage(data) {
        try {
            const msg = this.decodeMessage(data);
            switch (msg.type) {
                case MSG_HELLO_ACK:
                    this.handleHelloAck(msg.payload);
                    break;
                case MSG_UART_DATA:
                    this.handleUartData(msg.payload);
                    break;
                case MSG_SHUTDOWN:
                    console.log('Received SHUTDOWN from server');
                    this.disconnect();
                    break;
                default:
                    console.log('Unknown message type:', msg.type);
            }
        } catch (e) {
            console.error('Error decoding message:', e);
        }
    }

    handleHelloAck(payload) {
        if (payload.length < 1) {
            console.error('Invalid HELLO_ACK');
            return;
        }
        const version = payload[0];
        const capsJson = new TextDecoder().decode(payload.slice(1));
        console.log('HELLO_ACK: version=' + version + ', caps=' + capsJson);

        this.connected = true;
        this.updateStatus('connected', 'Connected');
        updateButtons(true);

        // Start sending VSYNC at ~60Hz
        this.startVsync();

        // Start processing keyboard input
        this.startInputProcessing();

        // Send CTS ready
        this.sendCts(true);
    }

    handleUartData(payload) {
        // Simple approach: buffer bytes and look for VDP query patterns
        for (const byte of payload) {
            this.vduBuffer.push(byte);
        }

        // Check for and handle VDP system queries
        this.processVduBuffer();
    }

    processVduBuffer() {
        // Look for VDU 23, 0, cmd sequences and respond
        // Also pass printable text to terminal
        while (this.vduBuffer.length > 0) {
            const byte = this.vduBuffer[0];

            // Check for VDU 23 (0x17) - system command
            if (byte === 0x17) {
                // Need at least 4 bytes: 23, 0, cmd, param
                if (this.vduBuffer.length < 4) break; // Wait for more

                if (this.vduBuffer[1] === 0) {
                    // VDU 23, 0, cmd - VDP system command
                    const cmd = this.vduBuffer[2];
                    const param = this.vduBuffer[3];

                    console.log('VDP cmd:', cmd.toString(16), param.toString(16));

                    if (cmd === 0x80) {
                        // VDU 23, 0, &80, n - packet command
                        this.handlePacketCommand(param);
                    }

                    // Consume these 4 bytes
                    this.vduBuffer.splice(0, 4);
                    continue;
                }
            }

            // Handle other bytes
            this.vduBuffer.shift();

            if (byte >= 32 && byte < 127) {
                // Printable ASCII
                this.terminal.write(String.fromCharCode(byte));
            } else if (byte === 10) {
                this.terminal.write('\n');
            } else if (byte === 13) {
                this.terminal.write('\r');
            } else if (byte === 8) {
                this.terminal.write('\b');
            } else if (byte === 12) {
                this.terminal.clear();
            }
            // Ignore other control chars
        }
    }

    handlePacketCommand(subcmd) {
        console.log('Packet subcmd:', subcmd);

        switch (subcmd) {
            case 0: // General poll - send version
            case 1: // Also responds to mode 1
                this.sendVdpVersion();
                break;
        }
    }

    sendPacket(data) {
        // Send a VDP response packet
        const packet = [data.length, ...data];
        console.log('Sending packet:', packet);
        this.sendUartData(packet);
    }

    sendVdpVersion() {
        // Send VDP version response
        const response = [
            0x80,  // Command echo
            2,     // Major version
            3,     // Minor version
            0,     // Patch
            0,     // Reserved
            5      // Candidate
        ];
        this.sendPacket(response);
        console.log('Sent VDP version');
    }

    sendUartData(bytes) {
        if (!this.connected || !this.ws) return;
        this.ws.send(this.encodeMessage(MSG_UART_DATA, bytes));
    }

    sendVsync() {
        if (!this.connected || !this.ws) return;
        this.ws.send(this.encodeMessage(MSG_VSYNC));
    }

    sendCts(ready) {
        if (!this.ws) return;
        this.ws.send(this.encodeMessage(MSG_CTS, [ready ? 1 : 0]));
    }

    startVsync() {
        this.stopVsync();
        // 60Hz = 16.666ms
        this.vsyncInterval = setInterval(() => this.sendVsync(), 16.666);
    }

    stopVsync() {
        if (this.vsyncInterval) {
            clearInterval(this.vsyncInterval);
            this.vsyncInterval = null;
        }
    }

    // Queue keyboard input
    queueInput(data) {
        // Convert string or array to byte array
        let bytes;
        if (typeof data === 'string') {
            bytes = new TextEncoder().encode(data);
        } else {
            bytes = data;
        }
        for (const b of bytes) {
            this.pendingInput.push(b);
        }
    }

    // Process queued input with delays (matching real hardware timing)
    startInputProcessing() {
        this.stopInputProcessing();
        // Send queued bytes at 10ms intervals
        this.inputInterval = setInterval(() => {
            if (this.pendingInput.length > 0 && this.connected) {
                const byte = this.pendingInput.shift();
                this.sendUartData([byte]);
            }
        }, 10);
    }

    stopInputProcessing() {
        if (this.inputInterval) {
            clearInterval(this.inputInterval);
            this.inputInterval = null;
        }
    }

    updateStatus(state, text) {
        const dot = document.getElementById('statusDot');
        const statusText = document.getElementById('statusText');
        dot.className = 'status-dot ' + state;
        statusText.textContent = text;
    }
}

// Initialize terminal and VDP
const terminal = new Terminal({
    cursorBlink: true,
    fontSize: 16,
    fontFamily: 'Consolas, "Courier New", monospace',
    theme: {
        background: '#1a1a2e',
        foreground: '#eee',
        cursor: '#4ecca3',
        cursorAccent: '#1a1a2e',
        selectionBackground: '#4ecca355',
    },
    cols: 80,
    rows: 30,
});

const fitAddon = new FitAddon.FitAddon();
terminal.loadAddon(fitAddon);
terminal.open(document.getElementById('terminal'));
fitAddon.fit();

const vdp = new AgonVDP(terminal);

// Handle terminal input
terminal.onData((data) => {
    vdp.queueInput(data);
});

// Handle special keys
terminal.onKey(({ key, domEvent }) => {
    // Handle special keys that aren't covered by onData
    if (domEvent.key === 'Escape') {
        vdp.queueInput([0x1B]); // ESC
    }
});

// UI event handlers
function updateButtons(connected) {
    document.getElementById('connectBtn').disabled = connected;
    document.getElementById('disconnectBtn').disabled = !connected;
    document.getElementById('wsUrl').disabled = connected;
}

document.getElementById('connectBtn').addEventListener('click', () => {
    const url = document.getElementById('wsUrl').value;
    vdp.connect(url);
});

document.getElementById('disconnectBtn').addEventListener('click', () => {
    vdp.disconnect();
});

// Allow Enter to connect
document.getElementById('wsUrl').addEventListener('keypress', (e) => {
    if (e.key === 'Enter' && !vdp.connected) {
        document.getElementById('connectBtn').click();
    }
});

// Handle window resize
window.addEventListener('resize', () => {
    fitAddon.fit();
});

// Welcome message
terminal.writeln('Agon Web VDP - Text Terminal');
terminal.writeln('Enter WebSocket URL and click Connect to start.');
terminal.writeln('');
