# TODO

## v1.3+
- [ ] Read ini file. there are so many options now...
- [ ] symbol support in debugger?
- [ ] gdbserver support?

## v1.1
- [ ] Test joysticks
- [ ] --renderer does nothing now...
- [ ] interrupt handling optimisation

## EZ80 flaws and omissions
- [ ] Mixed mode push ((ismixed<<1) | isadl) to stack. I haven't implemented the ismixed part
- [ ] SLL opcode is a trap on ez80! emulator accepts it erroneously
- [ ] ez80f92 PRT gpio-b pin 1 source (vblank) not implemented
- [ ] ldir has wrong timing of first/last iteration (needs otirx fix)
- [ ] memory wait states are not honoured

## VDP flaws and omissions
- [ ] ESP32 RAM is unlimited. Figure out ESP32 PSRAM accounting issues (C++ custom allocators)
- [ ] vdp audio sample rate setting not implemented
- [ ] VDP key repeat rate settings are ignored, and host system key repeat is used. But maybe this is good.
- [ ] PAUSE key is subject to key repeat, which it is not on a real agon.
- [ ] Copper effects not implemented
- [ ] Hardware sprites not implemented
