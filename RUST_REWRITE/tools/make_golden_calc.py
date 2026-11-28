"""golden/calc.json: zenko.py calc_eval + fmt_num on many expressions (the chat calculator's maths and number style).
Run: env PYTHONDONTWRITEBYTECODE=1 xvfb-run -a /usr/bin/python3.12 tools/make_golden_calc.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

EX = ["15% of 240", "2^10", "12 times 7", "(3+4)*2^3", "10/4", "1/0", "sqrt 81 + 3", "banana", "3 x 4", "2**300", "-2**2",
      "7//2", "7.5 % 2", "20% off 50", "sin(30)", "pi*2", "round(2.5)", "1e3+1", "0.1+0.2", "100 divided by 8", "10 mod 3",
      "2 squared", "ln(e)", "1,000,000 * 3", "2^-2", "5 - -3", "abs(-4.5)", "floor(7.8)", "10/3", "123456789*987654321",
      "2^100", "square root of 2", "3 cubed", "log(1000)", "0x1f + 1", "1_000 + 1", "2 ** 0.5", "(-8) ** (1/3)", "4 % 0",
      "1e308*10", "ceil(-0.5)", "exp(1)", "tan(45)", "1/3*3", ".5+.25", "7 % -3", "-7 // 2", "2^3^2", "round(-2.5)", "100",
      "0xff", "0b101", "0o17", "007", "00", "0_0", "1__0", "5.", "1e", "2j", "1+2j", "True+1", "None", "(1,2)", "()",
      "abs((-8)**(1/3))", "((-8)**(1/3))*2", "((-8)**(1/3))**2", "((-8)**(1/3))**0.5", "sqrt(-1)", "ln(0)", "log(0)",
      "log(10**300)", "ln(2**2000)", "exp(1000)", "exp(-1000)", "10.0**400", "10**400", "(10**200)**2", "2**200", "2**201",
      "7.5//2", "-7.5//2", "7.5%-2", "-0.0", "0*-1.0", "1e15", "1e15+0.5", "999999999999999.5", "1e16", "1e20/3",
      "123456.789", "-1234.5678901", "0.0000001", "1e-7*3", "2**0.5*1e20", "(2**100)/3", "(2**100)/(3**50)", "10**30/7",
      "1/3", "2/3", "round(1e300)", "floor(-1e20)", "sin(180)", "cos(90)", "tan(90)", "sqrt(2**1100)", "abs(-2**70)",
      "0.1*3", "1-0.9", "3%", "50% of 50%", "1 + (2", "1 +", "2 3", "sqrt(4, 2)", "sqrt()", "f(2)", "sqrt(4)(2)",
      "~3", "1@2", "1 < 2", "1/0 < 2", "1/0 + banana", "banana + 1/0", "2**-1074", "2.0**-1080", "0**-1", "0.0**-1.5",
      "(-1)**0.5", "(-2)**3", "(-2.0)**3", "(-2)**-3", "1e308+1e308", "-1e308*10", "(1e308*10)-(1e308*10)", "round(1e308*10)",
      "5 // 0", "5.0 // 0", "5 % 0.0", "((-8)**(1/3))//2", "((-8)**(1/3))/0", "4 plus 4 minus 2 times 3 over 3",
      "2 to the power of 10", "root of 16", "π", "1 ÷ 4", "3 × 3", "8 − 3", "what is 2+2", "calc 2+2", "12x(3)", "(2)x3",
      "1,5", "2^1000", "-(2^63)", "9007199254740993/1", "9007199254740993*1.0", "2**64/2**10", "18446744073709551617/3"]
out = []
for e in EX:
    try:
        v = Z.calc_eval(e)
        out.append([e, Z.fmt_num(v)])
    except ZeroDivisionError:
        out.append([e, "ZD"])
    except Exception:
        out.append([e, "ERR"])
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "calc.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
print(out)
os._exit(0)
