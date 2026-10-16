"""Writes test vectors for the correctly rounded functions: hard cases of CORE-MATH's .wc
files (a sample) and random inputs, each with the exact result rounded to nearest by mpmath
at 400 bits. Output line: name,input[,input2],result as C99 hex floats."""
import random, sys, math
import mpmath
from mpmath import mp, libmp
mp.prec = 400
WC = sys.argv[1]
random.seed(20261016)
F = {
 'sin': mpmath.sin, 'cos': mpmath.cos, 'tan': mpmath.tan, 'asin': mpmath.asin, 'acos': mpmath.acos,
 'atan': mpmath.atan, 'sinh': mpmath.sinh, 'cosh': mpmath.cosh, 'tanh': mpmath.tanh,
 'asinh': mpmath.asinh, 'acosh': mpmath.acosh, 'atanh': mpmath.atanh, 'exp': mpmath.exp,
 'log': mpmath.log, 'log2': lambda x: mpmath.log(x, 2), 'log10': mpmath.log10, 'cbrt': mpmath.cbrt,
 'erf': mpmath.erf, 'erfc': mpmath.erfc, 'tgamma': mpmath.gamma,
 'lgamma': lambda x: mpmath.log(abs(mpmath.gamma(x))),
 'atan2': lambda y, x: mpmath.atan2(y, x), 'pow': lambda x, y: mpmath.power(x, y),
 'hypot': lambda x, y: mpmath.sqrt(x*x + y*y),
}
DOM = {  # random input ranges (lo, hi) for one-argument functions, or a sampler
 'sin': (-1e6, 1e6), 'cos': (-1e6, 1e6), 'tan': (-1e6, 1e6), 'asin': (-1, 1), 'acos': (-1, 1),
 'atan': (-1e6, 1e6), 'sinh': (-700, 700), 'cosh': (-700, 700), 'tanh': (-20, 20),
 'asinh': (-1e6, 1e6), 'acosh': (1, 1e6), 'atanh': (-1, 1), 'exp': (-700, 700),
 'log': (1e-300, 1e300), 'log2': (1e-300, 1e300), 'log10': (1e-300, 1e300), 'cbrt': (-1e300, 1e300),
 'erf': (-6, 6), 'erfc': (-6, 26), 'tgamma': (-170, 170), 'lgamma': (-1e3, 1e5),
}
def to_double(r):
    return libmp.to_float(r._mpf_, strict=True, rnd=libmp.round_nearest)
def ok(d):
    return math.isfinite(d) and d != 0 and abs(d) >= 2.0**-1022
def parse(tok):
    tok = tok.strip()
    if not tok or any(s in tok for s in ('nan', 'inf')):
        return None
    try:
        v = float.fromhex(tok)
    except ValueError:
        return None
    return v if ok(v) else None
def logsample(lo, hi):
    if lo > 0:
        return math.exp(random.uniform(math.log(lo), math.log(hi)))
    if random.random() < 0.5:
        return random.uniform(lo, hi)
    m = max(abs(lo), abs(hi))
    v = math.exp(random.uniform(math.log(1e-300), math.log(m)))
    return min(max(v if random.random() < 0.5 else -v, lo), hi)
out = []
for name, f in F.items():
    two = name in ('atan2', 'pow', 'hypot')
    lines = [l.split('#')[0] for l in open(f'{WC}/{name}/{name}.wc') if not l.startswith('#')]
    lines = [l for l in lines if l.strip()]
    step = max(1, len(lines) // 150)
    cases = []
    for l in lines[::step]:
        parts = l.split(',')
        args = [parse(p) for p in parts[:2 if two else 1]]
        if any(a is None for a in args) or len(args) != (2 if two else 1):
            continue
        cases.append(args)
    for _ in range(100):
        if name == 'pow':
            cases.append([logsample(1e-3, 1e3), random.uniform(-50, 50)])
        elif name == 'atan2':
            cases.append([random.uniform(-1e3, 1e3), random.uniform(-1e3, 1e3)])
        elif name == 'hypot':
            cases.append([logsample(-1e150, 1e150), logsample(-1e150, 1e150)])
        else:
            cases.append([logsample(*DOM[name])])
    n = 0
    for args in cases:
        try:
            r = f(*[mpmath.mpf(a) for a in args])
        except (ValueError, ZeroDivisionError):
            continue
        if not isinstance(r, mpmath.mpf) or not mpmath.isfinite(r) or r == 0:
            continue
        try:
            d = to_double(r)
        except OverflowError:
            continue
        if not ok(d):
            continue
        out.append(','.join([name] + [a.hex() for a in args] + [d.hex()]))
        n += 1
    print(name, n, file=sys.stderr)
open('vectors.txt', 'w').write('\n'.join(out) + '\n')
