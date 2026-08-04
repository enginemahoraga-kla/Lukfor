// Safe inline calculator: tokenizer + shunting-yard, no eval().

type Tok =
  | { t: "num"; v: number }
  | { t: "op"; v: string }
  | { t: "lparen" }
  | { t: "rparen" }
  | { t: "func"; v: string };

const FUNCS: Record<string, (x: number) => number> = {
  sqrt: Math.sqrt,
  sin: Math.sin,
  cos: Math.cos,
  tan: Math.tan,
  log: Math.log10,
  ln: Math.log,
  abs: Math.abs,
  round: Math.round,
  floor: Math.floor,
  ceil: Math.ceil,
};

const CONSTS: Record<string, number> = {
  pi: Math.PI,
  e: Math.E,
};

const PREC: Record<string, number> = {
  "+": 1,
  "-": 1,
  "*": 2,
  "/": 2,
  "%": 2,
  "^": 3,
  "neg": 4,
};

const RIGHT_ASSOC = new Set(["^", "neg"]);

function tokenize(input: string): Tok[] | null {
  const toks: Tok[] = [];
  let i = 0;
  const s = input.replace(/\s+/g, "");
  while (i < s.length) {
    const c = s[i];
    if (/[0-9.]/.test(c)) {
      let j = i;
      while (j < s.length && /[0-9.]/.test(s[j])) j++;
      const raw = s.slice(i, j);
      const v = Number(raw);
      if (!Number.isFinite(v)) return null;
      toks.push({ t: "num", v });
      i = j;
    } else if (/[a-zA-Z]/.test(c)) {
      let j = i;
      while (j < s.length && /[a-zA-Z]/.test(s[j])) j++;
      const word = s.slice(i, j).toLowerCase();
      if (word in FUNCS) toks.push({ t: "func", v: word });
      else if (word in CONSTS) toks.push({ t: "num", v: CONSTS[word] });
      else if (word === "x") toks.push({ t: "op", v: "*" }); // 3x4
      else return null;
      i = j;
    } else if ("+-*/%^".includes(c)) {
      toks.push({ t: "op", v: c });
      i++;
    } else if (c === "(") {
      toks.push({ t: "lparen" });
      i++;
    } else if (c === ")") {
      toks.push({ t: "rparen" });
      i++;
    } else if (c === "×") {
      toks.push({ t: "op", v: "*" });
      i++;
    } else {
      return null;
    }
  }
  return toks;
}

export function evaluate(expr: string): number | null {
  const cleaned = expr.trim().replace(/^=/, "").replace(/=$/, "");
  if (!cleaned) return null;
  const toks = tokenize(cleaned);
  if (!toks || toks.length === 0) return null;

  // shunting-yard → RPN
  const out: Tok[] = [];
  const stack: Tok[] = [];
  let prev: Tok | null = null;
  for (const tok of toks) {
    if (tok.t === "num") {
      out.push(tok);
    } else if (tok.t === "func") {
      stack.push(tok);
    } else if (tok.t === "op") {
      let op = tok.v;
      // unary minus / plus
      const unaryPos = !prev || prev.t === "op" || prev.t === "lparen";
      if (unaryPos && op === "-") op = "neg";
      else if (unaryPos && op === "+") { prev = tok; continue; }
      else if (unaryPos) return null;
      while (stack.length) {
        const top = stack[stack.length - 1];
        if (top.t === "func") { out.push(stack.pop()!); continue; }
        if (top.t !== "op") break;
        const pTop = PREC[top.v], pOp = PREC[op];
        if (pTop > pOp || (pTop === pOp && !RIGHT_ASSOC.has(op))) {
          out.push(stack.pop()!);
        } else break;
      }
      stack.push({ t: "op", v: op });
    } else if (tok.t === "lparen") {
      stack.push(tok);
    } else if (tok.t === "rparen") {
      let found = false;
      while (stack.length) {
        const top = stack.pop()!;
        if (top.t === "lparen") { found = true; break; }
        out.push(top);
      }
      if (!found) return null;
      if (stack.length && stack[stack.length - 1].t === "func") out.push(stack.pop()!);
    }
    prev = tok;
  }
  while (stack.length) {
    const top = stack.pop()!;
    if (top.t === "lparen") return null;
    out.push(top);
  }

  // evaluate RPN
  const vals: number[] = [];
  for (const tok of out) {
    if (tok.t === "num") vals.push(tok.v);
    else if (tok.t === "func") {
      const a = vals.pop();
      if (a === undefined) return null;
      vals.push(FUNCS[tok.v](a));
    } else if (tok.t === "op") {
      if (tok.v === "neg") {
        const a = vals.pop();
        if (a === undefined) return null;
        vals.push(-a);
        continue;
      }
      const b = vals.pop(), a = vals.pop();
      if (a === undefined || b === undefined) return null;
      switch (tok.v) {
        case "+": vals.push(a + b); break;
        case "-": vals.push(a - b); break;
        case "*": vals.push(a * b); break;
        case "/": vals.push(a / b); break;
        case "%": vals.push(a % b); break;
        case "^": vals.push(Math.pow(a, b)); break;
        default: return null;
      }
    }
  }
  if (vals.length !== 1 || !Number.isFinite(vals[0])) return null;
  return vals[0];
}

/** True if the string looks like a math expression worth evaluating. */
export function looksLikeMath(q: string): boolean {
  const s = q.trim();
  if (s.startsWith("=")) return true;
  if (!/[0-9]/.test(s)) return false;
  if (!/[+\-*/%^×]/.test(s.slice(1))) return false; // needs an operator (not just leading minus)
  return /^[0-9\s.()+\-*/%^×a-z]+$/i.test(s);
}

export function formatResult(n: number): string {
  if (Number.isInteger(n) && Math.abs(n) < 1e15) return n.toLocaleString("en-US");
  const r = Math.round(n * 1e10) / 1e10;
  return String(r);
}
