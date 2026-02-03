# Popup MOS

## A batteries-included SDCard template for Agon Light / Console8

Compatible with Console8 MOS 2.2+

## Updates

The most recent version of this software collection can be found at
https://github.com/tomm/popup-mos

## Contents
### files in bin/
bin/bbcbasic.bin v1.06
Agon BBC BASIC in Z80 mode (limited to 64KiB RAM)
https://github.com/breakintoprogram/agon-bbc-basic/

bin/bbcbasic24.bin v1.03
Agon BBC BASIC in ADL mode (uses the full 512KiB RAM)
https://github.com/breakintoprogram/agon-bbc-basic-adl/

bin/crc32.bin v1.0
Creates a CRC32 checksum of a given file
https://github.com/envenomator/agon-crc32

bin/ez80asm.bin v2.1
EZ80 Assembler for Agon
https://github.com/envenomator/agon-ez80asm

bin/gunzip.bin v0.3
Decompressor for GNU-zipped archive files
https://github.com/sijnstra/agon-projects/tree/main/gunzip

bin/loadfont.bin
Requires VDP-2.8.0+. Loads raw binary fonts and PSF fonts as used by the Linux console.
https://github.com/lennart-benschop/agon-utilities

bin/unzip.bin v0.4
Decompressor for PKZip/Windows Zip files (including subdirectories)
https://github.com/sijnstra/agon-projects/tree/main/unzip

bin/vi.bin v1.06
A port of Busybox VI
https://github.com/tomm/toms-agon-experiments/tree/main/vi

bin/sort.bin
Sorts lines in a text file in ASCII-lexigographic order.
https://github.com/lennart-benschop/agon-utilities

bin/gpiovideodriver.bin
GPIO video driver for Framebuffer MOS
https://github.com/tomm/vga-ez80

### files in demos/
Various demo files to showcase features of the Agon Light/Console8

### files in docs/
Agon documentation based on https://github.com/breakintoprogram/agon-docs/wiki

### files in fonts/
fonts/fantasy.F08
Example free bitmap font by EPTO to demonstrate loadfont, downloaded from:
https://github.com/epto/epto-fonts/tree/master

### files in games/
games/akalabeth.bas
Akalabeth: World of Doom, often referred to as Ultima 0 (the first Ultima style game), ported to Agon's BBC Basic. More at:
https://github.com/sijnstra/Akalabeth

games/chuckie.bin
Agon rewrite of Chuckie Egg, including sound and joystick. Source code from the author at:
https://github.com/xianpinder/AgonChuckieEgg

games/drugwars v1.1
An adaption of the DOS version of the game, converted to BBC BASIC (Z80) for the Agon Light
https://github.com/oldpatientsea/DRUG-WARS-for-the-Agon-Light

games/flagon.bin
Flappy bird clone for Agon
https://github.com/sijnstra/agdev-projects/tree/main/flagon

games/invaders.bin v0.1
Space Invaders
https://github.com/pgregory/agon-light-invaders

games/lander_28.bin
Lunar Lander
https://github.com/richardturnnidge/lunar_lander

games/sokoban.bin v1.3
A custom port of the Sokoban game for the Agon platform
https://github.com/envenomator/agon-sokoban

games/rokky.bin
Dungeons and Diamonds game for Agon Light
https://github.com/nihirash/Agon-rokky

games/X-O.bin
Naughts and Crosses written in Pascal. Source coming soon.

### files in mos/
mos/bootlogo.bin
Bootlogo showing current screen settings and palette; recommended to add to autoexec.txt
https://github.com/sijnstra/agon-projects/tree/main/bootlogo

mos/cal.bin
Calendar utility
https://github.com/lennart-benschop/agon-utilities

mos/calc24.bin
24bit integer calculator usable from command line or interactive
https://github.com/sijnstra/agon-projects/tree/main/calc24

mos/comp.bin
File compare uitility
https://github.com/lennart-benschop/agon-utilities

mos/concat.bin
File concatenation utility
https://github.com/lennart-benschop/agon-utilities

mos/find.bin
Utility to find files in any directory
https://github.com/lennart-benschop/agon-utilities

mos/flash.bin v1.9
Agon MOS firmware upgrade utility
https://github.com/envenomator/agon-flash

mos/fontctl.bin
Font control utility to select which loaded font to use
https://github.com/lennart-benschop/agon-utilities

mos/grep.bin
*nix grep utility for Agon
https://github.com/lennart-benschop/agon-utilities

mos/hexdump.bin
Interactive file hexdumper
https://github.com/sijnstra/agon-projects

mos/memfill.bin
Fill a memory space with a particular value to assist with debugging
https://github.com/lennart-benschop/agon-utilities

mos/more.bin
Text file viewer
https://github.com/lennart-benschop/agon-utilities/

mos/nano.bin
Text editor
https://github.com/lennart-benschop/agon-utilities/

mos/playmod.bin release11
Agon .MOD tracker
https://github.com/HeathenUK/agon_mod

mos/strings.bin
A minimal implementation of the *nix strings utility
https://github.com/sijnstra/agon-projects

mos/wc.bin
Tool to count lines, words and characters in a file
https://github.com/lennart-benschop/agon-utilities

utils/agon-bench.bin
Benchmarking tool
https://github.com/tomm/toms-agon-experiments
