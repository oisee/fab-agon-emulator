# Screen Modes

Screen modes can be changed in BBC BASIC:

```
MODE n
```

Or from MOS:

```
*VDU 22 n
```

Mode   Dimensions  Colours  Refresh  Double-buffer?  
0      640x480     16       60Hz     N  
1      640x480     4        60Hz     N  
2      640x480     2        60Hz     N  
3      640x240     64       60Hz     N  
4      640x240     16       60Hz     N  
5      640x240     4        60Hz     N  
6      640x240     2        60Hz     N  
8      320x240     64       60Hz     N  
9      320x240     16       60Hz     N  
10     320x240     4        60Hz     N  
11     320x240     2        60Hz     N  
12     320x200     64       70Hz     N  
13     320x200     16       70Hz     N  
14     320x200     4        70Hz     N  
15     320x200     2        70Hz     N  
16     800x600     4        60Hz     N  
17     800x600     2        60Hz     N  
18     1024x768    2        60Hz     N  
129    640x480     4        60Hz     Y  
130    640x480     2        60Hz     Y  
132    640x240     16       60Hz     Y  
133    640x240     4        60Hz     Y  
134    640x240     2        60Hz     Y  
136    320x240     64       60Hz     Y  
137    320x240     16       60Hz     Y  
138    320x240     4        60Hz     Y  
139    320x240     2        60Hz     Y  
140    320x200     64       70Hz     Y  
141    320x200     16       70Hz     Y  
142    320x200     4        70Hz     Y  
143    320x200     2        70Hz     Y  

## Legacy modes (for MOS 1.03 compatibility)

MOS 1.03 legacy modes can be enabled on MOS 1.04 with:

```
VDU 23,0,193,1
```

And disabled again with:

```
VDU 23,0,193,0
```

When legacy modes are enabled, the following mode replace modes 0-3:  

0      1024x768    2        60Hz     N  
1      512x384     16       60Hz     N  
2      320x200     64       75Hz     N  
3      640x480     16       60Hz     N  
