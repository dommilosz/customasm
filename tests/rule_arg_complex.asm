; ::: include

#ruledef test
{
    ld {x} => 0x55 @ x`8
}

; :::
ld 1 + 1 ; = 0x5502
; :::
ld (1 + 1) ; = 0x5502
; :::
ld 3 + 4 * 5 ; = 0x5517
; :::
ld (3 + 4) * 5 ; = 0x5523
; :::
ld (3 + x) * 5 ; error: unknown
; :::
ld 1 + + 1 ; error: no match