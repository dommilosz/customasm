#ruledef test
{
    test ab => 0x11
    test a b => 0x22
}

test ab ; = 0x11
test a b ; = 0x22