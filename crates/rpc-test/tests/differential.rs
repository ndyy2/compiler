//! Differential tests: rpc vs reference pawncc 3.10.11, both executed by
//! the reference `pawnruns` runtime. Skipped when the reference toolchain
//! is absent (set `RPC_REF_DIR` to override the location).

use rpc_test::differential;

macro_rules! diff_case {
    ($name:ident, $src:literal) => {
        #[test]
        fn $name() {
            if !rpc_test::have_reference() {
                eprintln!("SKIP: no reference toolchain");
                return;
            }
            differential($src).unwrap();
        }
    };
}

diff_case!(simple_return,
    r#"
public main()
{
    new x = 10 * 5
    new y = x + 20
    return y
}
"#);

diff_case!(calls_and_arith,
    r#"
public main()
{
    new x = f()
    new y = g(7)
    return x + y
}
f()
{
    return 100
}
g(a)
{
    return a * 2
}
"#);

diff_case!(globals_and_while,
    r#"
new gN = 7
public main()
{
    new i = 0
    while (i < 3)
    {
        i = i + 1
    }
    if (i == 3)
        gN = gN + 1
    return gN
}
"#);

diff_case!(div_mod_and_neg,
    r#"
public main()
{
    new a = -17
    new b = 5
    new d = a / b
    new m = a % b
    return d * 100 + m
}
"#);

diff_case!(bitwise_and_shift,
    r#"
public main()
{
    new a = 12
    new b = 10
    return (a & b) * 1000 + (a | b) * 10 + (a ^ b) + (a << 2) * 100 + (128 >> 3)
}
"#);

diff_case!(nested_calls_three_args,
    r#"
public main()
{
    new r = add3(1, 2, 3)
    new q = sub2(r, 10)
    return q
}
add3(a, b, c)
{
    return a + b + c
}
sub2(x, y)
{
    return x - y
}
"#);

diff_case!(for_loop,
    r#"
public main()
{
    new i = 0
    new sum = 0
    for (new j = 1; j <= 10; j = j + 1)
    {
        sum = sum + j
        i = i + 1
    }
    return sum * 100 + i
}
"#);

diff_case!(if_else_chain,
    r#"
public main()
{
    new x = 42
    if (x > 50)
        x = 1
    else if (x > 40)
        x = 2
    else
        x = 3
    return x
}
"#);

diff_case!(deeply_nested,
    r#"
public main()
{
    return fac(5)
}
fac(n)
{
    new r = 1
    new i = 1
    while (i <= n)
    {
        r = r * i
        i = i + 1
    }
    return r
}
"#);

diff_case!(native_call,
    r#"
native print(...);
public main()
{
    new x = 40
    print(x)
    return x + 2
}
"#);

diff_case!(native_call_two_args,
    r#"
native print(...);
public main()
{
    new a = 6
    new b = 7
    print(a, b)
    return a * b
}
"#);
