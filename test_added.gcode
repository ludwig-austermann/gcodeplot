G28
G0  X 20 Y 28
M280 P0 S50
G3  X   18  Y   30  I   -2  J   0           ; Between arguments arbitrarily many whitespaces are allowed. These include ' ' and '\t'.
G1  X   14  Y   30
G3  X   10  Y   26  I   0   J   -4
G1  X   10  Y   21
G3  X   11  Y   20  I   1   J   0
G1  X   15  Y   20
G3 X17 Y22 I0 J2
G01 X17 Y18                                 ; It makes no difference if G01 or G1 is used.
G3 X15 Y20 I-2 J0
M280 P0 S0
G1 X11 Y20
M280 P0 S50
G3 X10 Y19 I0 J-1
G1 X10 Y14
G3 X14 Y10 I4 J0
G1 X18 Y10
G3 X20 Y12 I0 J2
; added by gcodeplot
M280 P0 S0
G0 X30 Y15
M280 P0 S50
G0 X40 Y15
G0 X40 Y25
G0 X30 Y25
G0 X30 Y15