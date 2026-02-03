   10 REM Conway's Game of Life
   20 REM Some assembly required
   30 :
   40 ON ERROR GOTO 940
   50 W%=78
   60 H%=56
   70 DIM L% (W%+1)*(H%+2)+1
   80 DIM N% (W%+1)*(H%+2)+1
   90 DIM code% 200
  100 FOR PASS% = 0 TO 3 STEP 3
  110   P%=code%
  120   [
  130   OPT PASS%
  140   PUSH IX
  150   ; First display the contents of the N% array
  160   LD HL,N%+W%+2
  170   LD D,1:LD C,H%
  180   .loop1
  190   LD A,31:RST 16; VDU 31
  200   LD A,1 :RST 16; column
  210   LD A,D :RST 16; row 
  220   LD B,W%
  230   .loop2
  240   LD A,(HL):RST 16; Print character from array
  250   INC HL          ; Skip guard byte.
  260   DJNZ loop2
  270   INC HL:INC D:DEC C
  280   JR NZ,loop1
  290   ; Copy the N% array into the L% array
  300   LD HL,N%
  310   LD DE,L%
  320   LD BC,+(W%+1)*(H%+2)+2
  330   LDIR
  340   ; The main update loop, update L% int N%
  350   LD IX,L%+W%+2
  360   LD HL,N%+W%+2
  370   LD C,H%
  380   .loop3
  390   LD B,W%
  400   .loop4
  410   XOR A ; Start summing the neigbouring cells
  420   ADD A,(IX-1)
  430   ADD A,(IX+1)
  440   ADD A,(IX-W%)
  450   ADD A,(IX-W%-1)
  460   ADD A,(IX-W%-2)
  470   ADD A,(IX+W%)
  480   ADD A,(IX+W%+1)
  490   ADD A,(IX+W%+2)
  500   CP 20 ; 2 neighbouring cells filled?
  510   JR Z,endloop
  520   CP 30 ; 3 neighbouring cells filled?
  530   LD A,32
  540   JR NZ,skip1
  550   LD A,42
  560   .skip1 LD (HL),A
  570   .endloop INC IX
  580   INC HL
  590   DJNZ loop4
  600   INC IX
  610   INC HL  ; Skip guard byte
  620   DEC C
  630   JR NZ, loop3
  640   POP IX
  650   RET
  660   ]
  670 NEXT PASS%
  680 MODE 0
  690 VDU 23,1,0
  700 FOR I%=0 TO (W%+1)*(H%+2)+1:N%?I%=32:NEXT
  710 FOR I%=0 TO W%-1
  720   FOR J%=0 TO H%-1
  730     IF RND(1)>=.7 N%?((W%+1)*(1+J%)+I%+1)=42
  740   NEXT
  750 NEXT
  760 :
  770 COLOUR 13
  780 FOR I%=0 TO W%+1
  790   PRINT TAB(I%,0);"+"
  800   PRINT TAB(I%,H%+1);"+"
  810 NEXT
  820 FOR I%=0 TO H%+1
  830   PRINT TAB(0,I%);"+"
  840   PRINT TAB(W%+1,I%);"+"
  850 NEXT
  860 G%=0
  870 COLOUR 14
  880 PRINT TAB(0,H%+3);"Generation: ";G%;
  890 COLOUR 15
  900 CALL code%
  910 G%=G%+1
  920 GOTO 870
  930 :
  940 ON ERROR OFF
  950 VDU 23,1,1
  960 PRINT TAB(0,0);
