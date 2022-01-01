#subruledef registers_recursive
{
	a => 0xaa
	b => 0xbb
	c => 0xcc
	{reg1: registers_recursive}, {reg2: registers_recursive} => reg1 @ reg2
}

#ruledef
{
    push {regs: registers_recursive} => 0xff @ regs
}

push a ; = 0xffaa
push b ; = 0xffbb
push c ; = 0xffcc
push a, a ; = 0xffaaaa
push b, c ; = 0xffbbcc
push c, a, b, b, c ; = 0xffccaabbbbcc