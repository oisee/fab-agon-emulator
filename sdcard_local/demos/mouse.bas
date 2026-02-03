   10 REM Mouse Test
   20 B%=0:OX%=0:OY%=0
   30 REPEAT
   40   X%=FN_getWordVDP(&29)
   50   Y%=FN_getWordVDP(&2B)
   60   NB%=FN_getByteVDP(&2D)
   70   DX%=FN_getWordVDP(&2F)
   80   DY%=FN_getWordVDP(&31)
   90   IF (OX% <> X%) OR (OY% <> Y%) OR (B% <> NB%) THEN PRINT TAB(10,5)X%,Y%,B%,DX%,DY%
  100   B%=NB%:OX%=X%:OY%=Y%
  110 UNTIL FALSE
  130 :
  140 DEF FN_getByteVDP(var%)
  150 A%=&A0
  160 L%=var%
  170 =USR(&FFF4)
  180 :
  190 DEF FN_getWordVDP(var%)
  200 =FN_getByteVDP(var%)+256*FN_getByteVDP(var%+1)
