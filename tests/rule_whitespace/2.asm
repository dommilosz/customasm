#ruledef test
{
    test ab => 0x11
    test a b => 0x22
}

testab ; error: no match
testa b ; error: no match