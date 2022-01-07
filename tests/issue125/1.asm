#subruledef register
{
    i  => 0x0a
    ij => 0x0b
    j  => 0x0c
}

#ruledef
{
    ld  {r: register}, {addr: u8} => 0x01 @ r @ addr
    ldi {r: register}, {addr: u8} => 0x02 @ r @ addr
}

ld  i,  0 ; = 0x010a00
ld  ij, 0 ; = 0x010b00
ld  j,  0 ; = 0x010c00
ldi i,  0 ; = 0x020a00
ldi ij, 0 ; = 0x020b00
ldi j,  0 ; = 0x020c00