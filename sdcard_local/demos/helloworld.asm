	.assume adl=1
	.org $40000
	jp start
	.align $40

	.db "MOS"
	.db 0 ; version
	.db 1 ; ADL enabled (24-bit addressing)
start:
	push ix
	push iy
	ld hl, hello
	ld bc, 0
	xor a
	rst.lil $18
	ld hl, 0
	pop iy
	pop ix
	ret
hello:
	.db "Hello world ADL-mode!", 13, 10, 0
