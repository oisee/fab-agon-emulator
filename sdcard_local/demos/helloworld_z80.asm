	.assume adl=0
	.org $0
	jp start
	.align $40

	.db "MOS"
	.db 0 ; version
	.db 0 ; ADL disabled
start:
	push.lil ix
	push.lil iy
	ld hl, hello
	ld bc, 0
	xor a
	rst.lis $18
	ld hl, 0
	pop.lil iy
	pop.lil ix
	ret.lis
hello:
	.db "Hello world Z80-mode!", 13, 10, 0
