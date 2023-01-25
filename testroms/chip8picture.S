# Decompiled with PEACH-8 Disassembler
CLS
LD   I, 0x248
LD   V0, 0x00
LD   V1, 0x1e
LD   V2, 0x00

# 0x20A BELOW
DRW  V2, V0, 0x02 # top left bold line
DRW  V2, V1, 0x02 # bottom left bold line
ADD  V2, 0x08

#if V2 == 0x40 (64), skip JP
SE   V2, 0x40
JP   0x20a

LD   V0, 0x00
LD   V1, 0x3e
LD   V2, 0x02
LD   I, 0x24a
DRW  V0, V2, 0x0e # long left top line
DRW  V1, V2, 0x0e # long right top line
ADD  V2, 0x0e
DRW  V0, V2, 0x0e # long left bottom line
DRW  V1, V2, 0x0e # long right bottom line
LD   I, 0x258
LD   V0, 0x0b
LD   V1, 0x08
DRW  V0, V1, 0x0f # C
ADD  V0, 0x0a
LD   I, 0x267
DRW  V0, V1, 0x0f # H
ADD  V0, 0x0a
LD   I, 0x276
DRW  V0, V1, 0x0f # I
ADD  V0, 0x03
LD   I, 0x285
DRW  V0, V1, 0x0f # P
ADD  V0, 0x0a
LD   I, 0x294
DRW  V0, V1, 0x0f # 8
JP   0x246 # SELF JUMP (ENDS HERE)

#DATA:

FILL 0xffff
RND  V0, 0xc0
RND  V0, 0xc0
RND  V0, 0xc0
RND  V0, 0xc0
RND  V0, 0xc0
RND  V0, 0xc0
RND  V0, 0xc0
FILL 0xff80
LD   V0, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
FILL 0xff81
OR   V1, V8
OR   V1, V8
OR   V1, V8
FILL 0xff81
OR   V1, V8
OR   V1, V8
OR   V1, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
LD   V0, V8
FILL 0x80ff
OR   V1, V8
OR   V1, V8
OR   V1, V8
FILL 0xff80
LD   V0, V8
LD   V0, V8
LD   V0, V8
FILL 0xff81
OR   V1, V8
OR   V1, V8
FILL 0x81ff
OR   V1, V8
OR   V1, V8
OR   V1, V8
FILL 0xffff
