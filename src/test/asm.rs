use diagn::RcReport;
use util::{FileServer, FileServerMock};
use super::ExpectedResult::*;
use super::{ExpectedResult, expect_result};
use ::assemble;


fn test<S, T>(instrset: S, asm: T, expected: ExpectedResult<(usize, &'static str)>)
where S: Into<String>, T: Into<String>
{
	let mut cpudef = "#cpudef \"test\" { \n".to_string();
	cpudef.push_str(&instrset.into());
	cpudef.push_str("\n }");

	let mut asm_with_cpudef = "#include \"instrset\" \n".to_string();
	asm_with_cpudef.push_str(&asm.into());
	
	let mut fileserver = FileServerMock::new();
	fileserver.add("instrset", cpudef.bytes().collect::<Vec<u8>>());
	fileserver.add("asm", asm_with_cpudef);
	
	let adjusted_result =
		if let Fail((filename, line, msg)) = expected
			{ Fail((filename, line + 1, msg)) }
		else
			{ expected };
	
	test_fileserver(&fileserver, "asm", adjusted_result);
}


fn test_fileserver<S>(fileserver: &FileServer, asm_filename: S, expected: ExpectedResult<(usize, &'static str)>)
where S: Into<String>
{
	let bits = if let Pass(expected) = expected
		{ expected.0 }
	else
		{ 4 };
		
	let report = RcReport::new();
	
	let result = assemble(report.clone(), fileserver, asm_filename).ok();
	let result = result.map(|r| (bits, r.generate_str(bits, 0, r.len())));
	let result = result.as_ref().map(|r| (r.0, r.1.as_ref()));
	
	expect_result(report.clone(), fileserver, result, expected);
}


#[test]
fn test_simple()
{
	test("",            "", Pass((1, "")));
	test("halt -> 8'0", "", Pass((1, "")));
	
	test("halt -> 8'0",             "halt", Pass((4, "00")));
	test("halt -> 16'0x1234",       "halt", Pass((4, "1234")));
	test("halt -> 8'0x12 @ 8'0x34", "halt", Pass((4, "1234")));
	test("halt -> 4'0xa  @ 4'0xb",  "halt", Pass((4, "ab")));
	
	test("halt -> (1 + 1)[7:0]", "halt", Pass((4, "02")));
	test("halt -> pc[7:0]",      "halt", Pass((4, "00")));
	
	test("#align 1 \n halt -> 1'0",     "halt", Pass((1, "0")));
	test("#align 1 \n halt -> 2'0b10",  "halt", Pass((1, "10")));
	test("#align 3 \n halt -> 3'0b101", "halt", Pass((1, "101")));
	test("#align 5 \n halt -> 5'0x13",  "halt", Pass((1, "10011")));
	
	test("#align 128 \n halt -> ((1 << 256) / 0xfedc)[255:0]", "halt", Pass((4, "000101254e8d998319892068f7ba90cd2a03ec79bad91fa81bbfa69a07b0c5a1")));
	
	test("#align 3 \n halt -> 3'0b101 \n cli -> 3'0b110", "halt \n cli \n halt \n cli", Pass((1, "101110101110")));
	test("#align 8 \n halt -> 8'0x12  \n cli -> 8'0x34",  "halt \n cli \n halt \n cli", Pass((4, "12341234")));
	
	test("halt -> 8'0", "unknown",         Fail(("asm", 1, "no match")));
	test("halt -> 8'0", "halt \n unknown", Fail(("asm", 2, "no match")));
	
	test("halt -> 8'0", "#unknown \n halt", Fail(("asm", 1, "unknown")));
}


#[test]
fn test_parameters()
{
	test("load {a} -> 8'0x12 @ a[7:0]",          "load 0x34", Pass((4, "1234")));
	test("load {a} -> 8'0x12 @ a[7:0]",          "load pc",   Pass((4, "1200")));
	test("load {a} -> 8'0x12 @ a[3:0] @ a[7:4]", "load 0x34", Pass((4, "1243")));
	test("load {a} -> 8'0x12 @ a[15:0]",         "load 0x34", Pass((4, "120034")));
	
	test("load {a}, {b} -> 8'0x12 @ a[7:0] @ b[7:0]", "load 0x34, 0x56", Pass((4, "123456")));
	test("load {a}, {b} -> 8'0x12 @ b[7:0] @ a[7:0]", "load 0x34, 0x56", Pass((4, "125634")));
	
	test("load {a}      -> 8'0x12 @ (a +  0x22)[7:0]", "load 0x34",       Pass((4, "1256")));
	test("load {a}      -> 8'0x12 @ (a + 0xf22)[7:0]", "load 0x34",       Pass((4, "1256")));
	test("load {a}, {b} -> 8'0x12 @ (a + b)[7:0]",     "load 0x34, 0x56", Pass((4, "128a")));
	
	test("load {a} -> 8'0x12 @ a[7:0]", "load 1 == 1", Fail(("asm", 1, "integer")));
	test("load {a} -> 8'0x12 @ a[7:0]", "load",        Fail(("asm", 1, "no match")));
	test("load {a} -> 8'0x12 @ a[7:0]", "load 1, 2",   Fail(("asm", 1, "no match")));
	test("load {a} -> 8'0x12 @ a[7:0]", "load a",      Fail(("asm", 1, "unknown")));
	
	test("load {a}, {b} -> 8'0x12 @ a[7:0] @ b[7:0]", "load 1",       Fail(("asm", 1, "no match")));
	test("load {a}, {b} -> 8'0x12 @ a[7:0] @ b[7:0]", "load 1, 2, 3", Fail(("asm", 1, "no match")));
}


#[test]
fn test_constraints()
{
	test("load {a} :: a % 2 == 0 -> 8'0x12 @ a[7:0]", "load 0x34", Pass((4, "1234")));
	
	test("load {a} :: a % 2 == 0               -> 8'0x12 @ a[7:0]", "load 0x23", Fail(("asm", 1, "constraint")));
	test("load {a} :: a % 2 == 0, \"not even\" -> 8'0x12 @ a[7:0]", "load 0x23", Fail(("asm", 1, "not even")));
	test("load {a} :: pc >= 0x02, \"too low\"  -> 8'0x12 @ a[7:0]", "load 0x34", Fail(("asm", 1, "too low")));
}


#[test]
fn test_addr_directive()
{
	test("halt -> 8'0x12", "                     halt", Pass((4, "12")));
	test("halt -> 8'0x12", "#addr 0x00        \n halt", Pass((4, "12")));
	test("halt -> 8'0x12", "#addr 0x34        \n halt", Pass((4, "12")));
	test("halt -> 8'0x12", "#addr 0xffff_ffff \n halt", Pass((4, "12")));
	
	test("halt -> 8'0x12 @ pc[7:0]", "                     halt", Pass((4, "1200")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x00        \n halt", Pass((4, "1200")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x34        \n halt", Pass((4, "1234")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0xffff_ffff \n halt", Pass((4, "12ff")));
	
	test("halt -> 8'0x12 @ pc[7:0]", "halt \n halt \n halt",                       Pass((4, "120012021204")));
	test("halt -> 8'0x12 @ pc[7:0]", "halt \n halt \n #addr 0x33 \n halt \n halt", Pass((4, "1200120212331235")));
	
	test("halt :: pc % 2 == 0 -> 8'0x12 @ pc[7:0]", "halt \n halt \n halt",               Pass((4, "120012021204")));
	test("halt :: pc % 2 == 0 -> 8'0x12 @ pc[7:0]", "halt \n halt \n #addr 0x33 \n halt", Fail(("asm", 4, "constraint")));
	
	test("halt -> 8'0x12", "#addr 0xffff_ffff_ffff_ffff / 8 - 1 \n halt", Pass((4, "12")));
	test("halt -> 8'0x12", "#addr 0xffff_ffff_ffff_ffff / 8",             Pass((4, "")));
	test("halt -> 8'0x12", "#addr 0xffff_ffff_ffff_ffff / 8     \n halt", Fail(("asm", 2, "valid range")));
	test("halt -> 8'0x12", "#addr 0x1_0000_0000_0000_0000 / 8   \n halt", Fail(("asm", 1, "valid range")));
	test("halt -> 8'0x12", "#addr 0x1_0000_0000_0000_0000       \n halt", Fail(("asm", 1, "large")));
}


#[test]
fn test_outp_directive()
{
	test("halt -> 8'0x12", "              halt", Pass((4, "12")));
	test("halt -> 8'0x12", "#outp 0x00 \n halt", Pass((4, "12")));
	test("halt -> 8'0x12", "#outp 0x01 \n halt", Pass((4, "0012")));
	test("halt -> 8'0x12", "#outp 0x02 \n halt", Pass((4, "000012")));
	test("halt -> 8'0x12", "#outp 0x10 \n halt", Pass((4, "0000000000000000000000000000000012")));
	
	test("halt -> 8'0x12 @ pc[7:0]", "              halt", Pass((4, "1200")));
	test("halt -> 8'0x12 @ pc[7:0]", "#outp 0x00 \n halt", Pass((4, "1200")));
	test("halt -> 8'0x12 @ pc[7:0]", "#outp 0x01 \n halt", Pass((4, "001200")));
	test("halt -> 8'0x12 @ pc[7:0]", "#outp 0x02 \n halt", Pass((4, "00001200")));
	test("halt -> 8'0x12 @ pc[7:0]", "#outp 0x10 \n halt", Pass((4, "000000000000000000000000000000001200")));
	
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x45 \n #outp 0x00 \n halt", Pass((4, "1245")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x77 \n #outp 0x01 \n halt", Pass((4, "001277")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x93 \n #outp 0x02 \n halt", Pass((4, "00001293")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0xbf \n #outp 0x10 \n halt", Pass((4, "0000000000000000000000000000000012bf")));
	
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x45 \n #outp 0x00 \n halt \n halt \n halt", Pass((4, "124512471249")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x77 \n #outp 0x01 \n halt \n halt \n halt", Pass((4, "0012771279127b")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0x93 \n #outp 0x02 \n halt \n halt \n halt", Pass((4, "0000129312951297")));
	test("halt -> 8'0x12 @ pc[7:0]", "#addr 0xbf \n #outp 0x10 \n halt \n halt \n halt", Pass((4, "0000000000000000000000000000000012bf12c112c3")));
	
	test("halt -> 8'0x12 @ pc[7:0]", "#outp 0x00 \n halt \n halt \n #outp 0x10 \n halt \n halt", Pass((4, "1200120200000000000000000000000012041206")));
	
	test("halt -> 8'0x12", "#outp 0xffff_ffff_ffff_ffff / 8",       Pass((4, "")));
	test("halt -> 8'0x12", "#outp 0x1_0000_0000_0000_0000 / 8",     Fail(("asm", 1, "valid range")));
	test("halt -> 8'0x12", "#outp 0x1_0000_0000_0000_0000 \n halt", Fail(("asm", 1, "large")));
}


#[test]
fn test_res_directive()
{
	test("halt -> 8'0x12 @ pc[7:0]", "halt \n #res 0", Pass((4, "1200")));
	test("halt -> 8'0x12 @ pc[7:0]", "halt \n #res 1", Pass((4, "120000")));
	test("halt -> 8'0x12 @ pc[7:0]", "halt \n #res 2", Pass((4, "12000000")));
	test("halt -> 8'0x12 @ pc[7:0]", "halt \n #res 4", Pass((4, "120000000000")));
	
	test("halt -> 8'0x12 @ pc[7:0]", "#res 0 \n halt", Pass((4, "1200")));
	test("halt -> 8'0x12 @ pc[7:0]", "#res 1 \n halt", Pass((4, "001201")));
	test("halt -> 8'0x12 @ pc[7:0]", "#res 2 \n halt", Pass((4, "00001202")));
	test("halt -> 8'0x12 @ pc[7:0]", "#res 4 \n halt", Pass((4, "000000001204")));
}


#[test]
fn test_data_directive()
{
	test("", "#d8 0",     Pass((4, "00")));
	test("", "#d8 0xff",  Pass((4, "ff")));
	test("", "#d8 -1",    Pass((4, "ff")));
	test("", "#d8 0x80",  Pass((4, "80")));
	test("", "#d8 -0x7f", Pass((4, "81")));
	test("", "#d8 -0x80", Pass((4, "80")));
	test("", "#d8 1 + 1", Pass((4, "02")));
	test("", "#d8 pc",    Pass((4, "00")));
	
	test("", "#d8 0x1ff[7:0]", Pass((4, "ff")));
	test("", "#d8 -0x81[7:0]", Pass((4, "7f")));
	
	test("", "#d16 0",       Pass((4, "0000")));
	test("", "#d16 0xffff",  Pass((4, "ffff")));
	test("", "#d16 -1",      Pass((4, "ffff")));
	test("", "#d16 0x8000",  Pass((4, "8000")));
	test("", "#d16 -0x7fff", Pass((4, "8001")));
	test("", "#d16 -0x8000", Pass((4, "8000")));
	test("", "#d16 1 + 1",   Pass((4, "0002")));
	test("", "#d16 pc",      Pass((4, "0000")));
	
	test("", "#d16 0x1ffff[15:0]", Pass((4, "ffff")));
	test("", "#d16 -0x8001[15:0]", Pass((4, "7fff")));
	
	test("#align 3", "#d3 0",      Pass((1, "000")));
	test("#align 3", "#d3 0b111",  Pass((1, "111")));
	test("#align 3", "#d3 -1",     Pass((1, "111")));
	test("#align 3", "#d3 0b100",  Pass((1, "100")));
	test("#align 3", "#d3 -0b011", Pass((1, "101")));
	test("#align 3", "#d3 -0b100", Pass((1, "100")));
	test("#align 3", "#d3 1 + 1",  Pass((1, "010")));
	test("#align 3", "#d3 pc",     Pass((1, "000")));
	
	test("", "#d8 1,    2,    3", Pass((4, "010203")));
	test("", "#d8 1, \n 2, \n 3", Pass((4, "010203")));
	
	test("", "#d16  1,  2,  3", Pass((4, "000100020003")));
	test("", "#d16 -1, -2, -3", Pass((4, "fffffffefffd")));
	test("", "#d32  1,  2,  3", Pass((4, "000000010000000200000003")));
	test("", "#d32 -1, -2, -3", Pass((4, "fffffffffffffffefffffffd")));
	
	test("#align 1", "#d1 1, 0, 1, -1", Pass((1, "1011")));
	test("#align 2", "#d2 1, 2, 3, -1", Pass((1, "01101111")));
	test("#align 3", "#d3 1, 2, 3, -1", Pass((1, "001010011111")));
	test("#align 5", "#d5 1, 2, 3, -1", Pass((1, "00001000100001111111")));
	test("#align 7", "#d7 1, 2, 3, -1", Pass((1, "0000001000001000000111111111")));
	test("#align 9", "#d9 1, 2, 3, -1", Pass((1, "000000001000000010000000011111111111")));
	
	test("", "#d8  x \n  x =  0x12", Pass((4, "12")));
	test("", "#d8  x \n  x = 0x100", Fail(("asm", 1, "large")));
	test("", "#d8  x \n  x = -0x81", Fail(("asm", 1, "large")));
	test("", "#d8  x",               Fail(("asm", 1, "unknown")));
	test("", "#d8 .x \n .x =  0x12", Pass((4, "12")));
	test("", "#d8 .x \n .x = 0x100", Fail(("asm", 1, "large")));
	test("", "#d8 .x \n .x = -0x81", Fail(("asm", 1, "large")));
	test("", "#d8 .x",               Fail(("asm", 1, "unknown")));
	
	test("", "              #d8 x + pc \n #addr 0x55 \n x = 0x11", Pass((4, "11")));
	test("", "#addr 0x55 \n #d8 x + pc \n               x = 0x11", Pass((4, "66")));
	
	test("", "#d8 0x100", Fail(("asm", 1, "large")));
	test("", "#d8 -0x81", Fail(("asm", 1, "large")));
	
	test("", "#d16 0x10000", Fail(("asm", 1, "large")));
	test("", "#d16 -0x8001", Fail(("asm", 1, "large")));
	
	test("#align 1", "#d1 2",  Fail(("asm", 1, "large")));
	test("#align 1", "#d1 -2", Fail(("asm", 1, "large")));
	
	test("", "#d1 0", Fail(("asm", 1, "align")));
	test("", "#d2 0", Fail(("asm", 1, "align")));
	test("", "#d3 0", Fail(("asm", 1, "align")));
	test("", "#d4 0", Fail(("asm", 1, "align")));
	test("", "#d5 0", Fail(("asm", 1, "align")));
	test("", "#d6 0", Fail(("asm", 1, "align")));
	test("", "#d7 0", Fail(("asm", 1, "align")));
	test("", "#d9 0", Fail(("asm", 1, "align")));
	
	test("", "#d0    0",      Fail(("asm", 1, "invalid")));
	test("", "#d16y  0xffff", Fail(("asm", 1, "unknown")));
	test("", "#da16  0xffff", Fail(("asm", 1, "unknown")));
	test("", "#d0x10 0xffff", Fail(("asm", 1, "unknown")));
}


#[test]
fn test_str_directive()
{
	test("", "#str \"\"",                   Pass((4, "")));
	test("", "#str \"\\thello\\r\\n\\0\"",  Pass((4, "0968656c6c6f0d0a00")));
	test("", "#str \"\\x00\\x01\\x02\"",    Pass((4, "000102")));
	test("", "#str \"\\u{0}\\u{1}\\u{2}\"", Pass((4, "000102")));
	
	test("#align 16", "#str \"\"",                         Pass((4, "")));
	test("#align 16", "#str \"hello\\r\\n\\0\"",           Pass((4, "68656c6c6f0d0a00")));
	test("#align 32", "#str \"\\x00\\x01\\x02\\x03\"",     Pass((4, "00010203")));
	test("#align 32", "#str \"\\u{0}\\u{1}\\u{2}\\u{3}\"", Pass((4, "00010203")));
	
	test("", "#str \"\\u{7f}\\u{80}\\u{ff}\\u{10ffff}\"", Pass((4, "7fc280c3bff48fbfbf")));
	
	test("", "#str \"木水火\"", Pass((4, "e69ca8e6b0b4e781ab")));
	
	test("", "#str \"\\z\"", Fail(("asm", 1, "invalid")));
	
	test("#align 5",  "#str \"abc\"", Fail(("asm", 1, "misaligned")));
	test("#align 32", "#str \"abc\"", Fail(("asm", 1, "misaligned")));
}


#[test]
fn test_labels()
{
	static INSTRSET: &'static str = "
		halt -> 8'0x12 \n
		jump {a} -> 8'0x77 @ a[7:0]";
	
	test(INSTRSET, "label: halt \n jump label",                Pass((4, "127700")));
	test(INSTRSET, "       halt \n jump label \n label: halt", Pass((4, "12770312")));
	test(INSTRSET, "       halt \n jump label",                Fail(("asm", 2, "unknown")));
	
	test(INSTRSET, "label = 0x55 \n halt \n jump label",                 Pass((4, "127755")));
	test(INSTRSET, "                halt \n jump label \n label = 0x55", Pass((4, "127755")));
	
	test(INSTRSET, ".label: halt \n jump .label",                 Pass((4, "127700")));
	test(INSTRSET, "        halt \n jump .label \n .label: halt", Pass((4, "12770312")));
	test(INSTRSET, "        halt \n jump .label",                 Fail(("asm", 2, "unknown")));
	test(INSTRSET, " label: halt \n jump .label",                 Fail(("asm", 2, "unknown")));
	
	test(INSTRSET, ".label = 0x55 \n halt \n jump .label",                  Pass((4, "127755")));
	test(INSTRSET, "                 halt \n jump .label \n .label = 0x55", Pass((4, "127755")));
	test(INSTRSET, " label = 0x55 \n halt \n jump .label",                  Fail(("asm", 3, "unknown")));
	
	test(INSTRSET, "label1 = 0x55          \n label2 = label1 + 0x11 \n jump label2", Pass((4, "7766")));
	test(INSTRSET, "label2 = label1 + 0x11 \n label1 = 0x55          \n jump label2", Fail(("asm", 1, "unknown")));
	
	test(INSTRSET, "start: halt \n .br: jump .br \n mid: halt \n .br: jump .br", Pass((4, "127701127704")));
	test(INSTRSET, "start: halt \n      jump .br \n mid: halt \n .br: jump .br", Fail(("asm", 2, "unknown")));
	test(INSTRSET, "start: halt \n .br: jump .br \n mid: halt \n      jump .br", Fail(("asm", 4, "unknown")));
	
	test(INSTRSET, "jump = 0x33 \n jump jump", Pass((4, "7733")));
	
	test(INSTRSET, "start: halt \n .br: jump .br \n #addr 0xf0 \n mid: halt \n .br: jump .br", Pass((4, "1277011277f1")));
	
	test(INSTRSET, " label: halt \n  label: halt", Fail(("asm", 2, "duplicate")));
	test(INSTRSET, ".label: halt \n .label: halt", Fail(("asm", 2, "duplicate")));
}


#[test]
fn test_cascading()
{
	static INSTRSET: &'static str = "
		load {a} :: a < 0x10 -> 8'0x10 @ a[7:0] \n
		load {a} :: a < 0x20 -> 8'0x20 @ a[7:0] \n
		load {a}             -> 8'0xff @ a[7:0] \n
		
		store {a} :: a < 0x10 -> 8'0x30 @ a[7:0] \n
		store {a} :: a < 0x20 -> 8'0x40 @ a[7:0] \n
		store {a} :: a < 0x30 -> 8'0x50 @ a[7:0] \n
		
		add {a}, {b} :: a < 0x10 :: b < 0x10 -> 8'0xaa @ a[7:0] @ b[7:0] \n
		add {a}, {b} :: a < 0x20             -> 8'0xbb @ a[7:0] @ b[7:0] \n
		add {a}, {b}             :: b < 0x20 -> 8'0xcc @ a[7:0] @ b[7:0] \n 
		add {a}, {b}                         -> 8'0xdd @ a[7:0] @ b[7:0]";
		
	test(INSTRSET, "load 0x05", Pass((4, "1005")));
	test(INSTRSET, "load 0x15", Pass((4, "2015")));
	test(INSTRSET, "load 0x25", Pass((4, "ff25")));
	
	test(INSTRSET, "value = 0x05 \n load value", Pass((4, "1005")));
	test(INSTRSET, "value = 0x15 \n load value", Pass((4, "2015")));
	test(INSTRSET, "value = 0x25 \n load value", Pass((4, "ff25")));
	
	test(INSTRSET, "load value \n value = 0x05", Pass((4, "ff05")));
	test(INSTRSET, "load value \n value = 0x15", Pass((4, "ff15")));
	test(INSTRSET, "load value \n value = 0x25", Pass((4, "ff25")));
	
	test(INSTRSET, "store 0x05", Pass((4, "3005")));
	test(INSTRSET, "store 0x15", Pass((4, "4015")));
	test(INSTRSET, "store 0x25", Pass((4, "5025")));
	test(INSTRSET, "store 0x35", Fail(("asm", 1, "constraint")));
	
	test(INSTRSET, "value = 0x05 \n store value", Pass((4, "3005")));
	test(INSTRSET, "value = 0x15 \n store value", Pass((4, "4015")));
	test(INSTRSET, "value = 0x25 \n store value", Pass((4, "5025")));
	test(INSTRSET, "value = 0x35 \n store value", Fail(("asm", 2, "constraint")));
	
	test(INSTRSET, "store value \n value = 0x05", Pass((4, "5005")));
	test(INSTRSET, "store value \n value = 0x15", Pass((4, "5015")));
	test(INSTRSET, "store value \n value = 0x25", Pass((4, "5025")));
	test(INSTRSET, "store value \n value = 0x35", Fail(("asm", 1, "constraint")));
	
	test(INSTRSET, "add 0x05, 0x07", Pass((4, "aa0507")));
	test(INSTRSET, "add 0x15, 0x25", Pass((4, "bb1525")));
	test(INSTRSET, "add 0x25, 0x15", Pass((4, "cc2515")));
	test(INSTRSET, "add 0x25, 0x25", Pass((4, "dd2525")));
	
	test(INSTRSET, "a = 0x05 \n b = 0x07 \n add a, b",                         Pass((4, "aa0507")));
	test(INSTRSET, "a = 0x05 \n             add a, b \n b = 0x07",             Pass((4, "bb0507")));
	test(INSTRSET, "a = 0x15 \n             add a, b \n b = 0x25",             Pass((4, "bb1525")));
	test(INSTRSET, "            b = 0x07 \n add a, b \n a = 0x05",             Pass((4, "cc0507")));
	test(INSTRSET, "            b = 0x15 \n add a, b \n a = 0x25",             Pass((4, "cc2515")));
	test(INSTRSET, "                        add a, b \n a = 0x07 \n b = 0x07", Pass((4, "dd0707")));
	test(INSTRSET, "                        add a, b \n a = 0x25 \n b = 0x25", Pass((4, "dd2525")));
}


#[test]
fn test_include_directive()
{
	static INSTRSET: &'static str = "
		#cpudef
		{
			halt     -> 8'0x12 @ pc[7:0]
			load {a} -> 8'0x34 @  a[7:0]
		}";
		
	static MAIN1: &'static str = "
		#include \"instrset\"
		
		start:
			halt
			load start
			halt
			
		#include \"folder1/file1\"
		#include \"file1\"
		
			halt
			load start
			load at_folder1_file1
			load at_folder1_file2
			load at_file1";
			
	static FOLDER1_FILE1: &'static str ="
		at_folder1_file1:
			halt
			load start
			
		#include \"file2\"";
	
	static FOLDER1_FILE2: &'static str ="
		at_folder1_file2:
			halt
			load start
			load at_folder1_file1
			load at_folder1_file2";
			
	static FILE1: &'static str ="
		at_file1:
			halt
			load start
			load at_folder1_file1
			load at_folder1_file2
			load at_file1";
			
	static MAIN2: &'static str ="
		#include \"instrset\"
		#include \"unknown\"";
			
	static MAIN3: &'static str ="
		#include \"instrset\"
		#include \"../invalid\"";
			
	static MAIN4: &'static str ="
		#include \"instrset\"
		#include \"./invalid\"";
		
	static MAIN5: &'static str ="
		#include \"instrset\"
		#include \"C:\\invalid\"";
	
	let mut fileserver = FileServerMock::new();
	fileserver.add("instrset", INSTRSET);
	fileserver.add("main1", MAIN1);
	fileserver.add("folder1/file1", FOLDER1_FILE1);
	fileserver.add("folder1/file2", FOLDER1_FILE2);
	fileserver.add("file1", FILE1);
	fileserver.add("main2", MAIN2);
	fileserver.add("main3", MAIN3);
	fileserver.add("main4", MAIN4);
	fileserver.add("main5", MAIN5);
	
	test_fileserver(&fileserver, "main1", Pass((4, "12003400120412063400120a34003406340a121234003406340a3412121c34003406340a3412")));
	test_fileserver(&fileserver, "main2", Fail(("main2", 3, "not found")));
	test_fileserver(&fileserver, "main3", Fail(("main3", 3, "invalid")));
	test_fileserver(&fileserver, "main4", Fail(("main4", 3, "invalid")));
	test_fileserver(&fileserver, "main5", Fail(("main5", 3, "invalid")));
}


#[test]
fn test_incbin_directive()
{
	static INSTRSET1: &'static str = "#cpudef { }";

	static INSTRSET2: &'static str = "#cpudef { \n #align 5 \n }";
	
	static INSTRSET3: &'static str = "#cpudef { \n #align 32 \n }";
	
	static MAIN1_1: &'static str = "#include \"instrset1\" \n #incbin \"binary1\"";
	static MAIN1_2: &'static str = "#include \"instrset1\" \n #incbin \"binary2\"";
	static MAIN1_3: &'static str = "#include \"instrset1\" \n #incbin \"binary3\"";
	static MAIN2_1: &'static str = "#include \"instrset2\" \n #incbin \"binary1\"";
	static MAIN2_2: &'static str = "#include \"instrset2\" \n #incbin \"binary2\"";
	static MAIN2_3: &'static str = "#include \"instrset2\" \n #incbin \"binary3\"";
	static MAIN3_1: &'static str = "#include \"instrset3\" \n #incbin \"binary1\"";
	static MAIN3_2: &'static str = "#include \"instrset3\" \n #incbin \"binary2\"";
	static MAIN3_3: &'static str = "#include \"instrset3\" \n #incbin \"binary3\"";
		
	static MAIN4: &'static str = "#include \"instrset1\" \n #incbin \"unknown\"";
	static MAIN5: &'static str = "#include \"instrset1\" \n #incbin \"../invalid\"";
	
	static BINARY1: &'static str = "\x12\x34\x56\x78";
	
	static BINARY2: &'static str = "testing!!!";
	
	static BINARY3: &'static str = "\u{80}\u{ff}\u{5927}";
	
	let mut fileserver = FileServerMock::new();
	fileserver.add("instrset1", INSTRSET1);
	fileserver.add("instrset2", INSTRSET2);
	fileserver.add("instrset3", INSTRSET3);
	fileserver.add("main1_1", MAIN1_1);
	fileserver.add("main1_2", MAIN1_2);
	fileserver.add("main1_3", MAIN1_3);
	fileserver.add("main2_1", MAIN2_1);
	fileserver.add("main2_2", MAIN2_2);
	fileserver.add("main2_3", MAIN2_3);
	fileserver.add("main3_1", MAIN3_1);
	fileserver.add("main3_2", MAIN3_2);
	fileserver.add("main3_3", MAIN3_3);
	fileserver.add("main4", MAIN4);
	fileserver.add("main5", MAIN5);
	fileserver.add("binary1", BINARY1);
	fileserver.add("binary2", BINARY2);
	fileserver.add("binary3", BINARY3);
	
	test_fileserver(&fileserver, "main1_1", Pass((4, "12345678")));
	test_fileserver(&fileserver, "main1_2", Pass((4, "74657374696e67212121")));
	test_fileserver(&fileserver, "main1_3", Pass((4, "c280c3bfe5a4a7")));
	
	test_fileserver(&fileserver, "main2_1", Fail(("main2_1", 2, "align")));
	test_fileserver(&fileserver, "main2_2", Pass((4, "74657374696e67212121")));
	test_fileserver(&fileserver, "main2_3", Fail(("main2_3", 2, "align")));
	
	test_fileserver(&fileserver, "main3_1", Pass((4, "12345678")));
	test_fileserver(&fileserver, "main3_2", Fail(("main3_2", 2, "align")));
	test_fileserver(&fileserver, "main3_3", Fail(("main3_3", 2, "align")));
	
	test_fileserver(&fileserver, "main4", Fail(("main4", 2, "not found")));
	test_fileserver(&fileserver, "main5", Fail(("main5", 2, "invalid")));
}


#[test]
fn test_incstr_directives()
{
	static INSTRSET1: &'static str = "#cpudef { }";

	static INSTRSET2: &'static str = "#cpudef { \n #align 5 \n }";
	
	static INSTRSET3: &'static str = "#cpudef { \n #align 32 \n }";
	
	static MAIN1_1: &'static str = "#include \"instrset1\" \n #incbinstr \"str1\"";
	static MAIN1_2: &'static str = "#include \"instrset1\" \n #inchexstr \"str1\"";
	static MAIN1_3: &'static str = "#include \"instrset1\" \n #incbinstr \"str2\"";
	static MAIN1_4: &'static str = "#include \"instrset1\" \n #inchexstr \"str2\"";
	static MAIN1_5: &'static str = "#include \"instrset1\" \n #incbinstr \"str3\"";
	static MAIN1_6: &'static str = "#include \"instrset1\" \n #inchexstr \"str3\"";
	static MAIN1_7: &'static str = "#include \"instrset1\" \n #incbinstr \"str4\"";
	
	static MAIN2_1: &'static str = "#include \"instrset2\" \n #incbinstr \"str1\"";
	static MAIN2_2: &'static str = "#include \"instrset2\" \n #inchexstr \"str1\"";
	static MAIN2_3: &'static str = "#include \"instrset2\" \n #incbinstr \"str2\"";
	static MAIN2_4: &'static str = "#include \"instrset2\" \n #inchexstr \"str2\"";
	static MAIN2_5: &'static str = "#include \"instrset2\" \n #incbinstr \"str3\"";
	static MAIN2_6: &'static str = "#include \"instrset2\" \n #inchexstr \"str3\"";
	static MAIN2_7: &'static str = "#include \"instrset2\" \n #incbinstr \"str4\"";
	
	static MAIN3_1: &'static str = "#include \"instrset3\" \n #incbinstr \"str1\"";
	static MAIN3_2: &'static str = "#include \"instrset3\" \n #inchexstr \"str1\"";
	static MAIN3_3: &'static str = "#include \"instrset3\" \n #incbinstr \"str2\"";
	static MAIN3_4: &'static str = "#include \"instrset3\" \n #inchexstr \"str2\"";
	static MAIN3_5: &'static str = "#include \"instrset3\" \n #incbinstr \"str3\"";
	static MAIN3_6: &'static str = "#include \"instrset3\" \n #inchexstr \"str3\"";
	static MAIN3_7: &'static str = "#include \"instrset3\" \n #incbinstr \"str4\"";
	
	static MAIN4: &'static str = "#include \"instrset1\" \n #incbin \"unknown\"";
	static MAIN5: &'static str = "#include \"instrset1\" \n #incbin \"../invalid\"";
	
	static STR1: &'static str = "1110101101000111";
	
	static STR2: &'static str = "0123456789abcdef";
	
	static STR3: &'static str = "0123456789abcdeg";
	
	static STR4: &'static str = "111010110100011";
	
	let mut fileserver = FileServerMock::new();
	fileserver.add("instrset1", INSTRSET1);
	fileserver.add("instrset2", INSTRSET2);
	fileserver.add("instrset3", INSTRSET3);
	fileserver.add("main1_1", MAIN1_1);
	fileserver.add("main1_2", MAIN1_2);
	fileserver.add("main1_3", MAIN1_3);
	fileserver.add("main1_4", MAIN1_4);
	fileserver.add("main1_5", MAIN1_5);
	fileserver.add("main1_6", MAIN1_6);
	fileserver.add("main1_7", MAIN1_7);
	fileserver.add("main2_1", MAIN2_1);
	fileserver.add("main2_2", MAIN2_2);
	fileserver.add("main2_3", MAIN2_3);
	fileserver.add("main2_4", MAIN2_4);
	fileserver.add("main2_5", MAIN2_5);
	fileserver.add("main2_6", MAIN2_6);
	fileserver.add("main2_7", MAIN2_7);
	fileserver.add("main3_1", MAIN3_1);
	fileserver.add("main3_2", MAIN3_2);
	fileserver.add("main3_3", MAIN3_3);
	fileserver.add("main3_4", MAIN3_4);
	fileserver.add("main3_5", MAIN3_5);
	fileserver.add("main3_6", MAIN3_6);
	fileserver.add("main3_7", MAIN3_7);
	fileserver.add("main4", MAIN4);
	fileserver.add("main5", MAIN5);
	fileserver.add("str1", STR1);
	fileserver.add("str2", STR2);
	fileserver.add("str3", STR3);
	fileserver.add("str4", STR4);
	
	test_fileserver(&fileserver, "main1_1", Pass((1, "1110101101000111")));
	test_fileserver(&fileserver, "main1_2", Pass((4, "1110101101000111")));
	test_fileserver(&fileserver, "main1_3", Fail(("main1_3", 2, "invalid character")));
	test_fileserver(&fileserver, "main1_4", Pass((4, "0123456789abcdef")));
	test_fileserver(&fileserver, "main1_5", Fail(("main1_5", 2, "invalid character")));
	test_fileserver(&fileserver, "main1_6", Fail(("main1_6", 2, "invalid character")));
	test_fileserver(&fileserver, "main1_7", Fail(("main1_7", 2, "align")));
	
	test_fileserver(&fileserver, "main2_1", Fail(("main2_1", 2, "align")));
	test_fileserver(&fileserver, "main2_2", Fail(("main2_2", 2, "align")));
	test_fileserver(&fileserver, "main2_3", Fail(("main2_3", 2, "invalid character")));
	test_fileserver(&fileserver, "main2_4", Fail(("main2_4", 2, "align")));
	test_fileserver(&fileserver, "main2_5", Fail(("main2_5", 2, "invalid character")));
	test_fileserver(&fileserver, "main2_6", Fail(("main2_6", 2, "invalid character")));
	test_fileserver(&fileserver, "main2_7", Pass((1, "111010110100011")));
	
	test_fileserver(&fileserver, "main3_1", Fail(("main3_1", 2, "align")));
	test_fileserver(&fileserver, "main3_2", Pass((4, "1110101101000111")));
	test_fileserver(&fileserver, "main3_3", Fail(("main3_3", 2, "invalid character")));
	test_fileserver(&fileserver, "main3_4", Pass((4, "0123456789abcdef")));
	test_fileserver(&fileserver, "main3_5", Fail(("main3_5", 2, "invalid character")));
	test_fileserver(&fileserver, "main3_6", Fail(("main3_6", 2, "invalid character")));
	test_fileserver(&fileserver, "main3_7", Fail(("main3_7", 2, "align")));
	
	test_fileserver(&fileserver, "main4", Fail(("main4", 2, "not found")));
	test_fileserver(&fileserver, "main5", Fail(("main5", 2, "invalid")));
}