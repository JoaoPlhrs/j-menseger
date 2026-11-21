(() => {
  function fn(t) {
    return (
      t instanceof Uint8Array ||
      (ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array")
    );
  }
  function nt(t, e = "") {
    if (!Number.isSafeInteger(t) || t < 0) {
      let a = e && `"${e}" `;
      throw new Error(`${a}expected integer >= 0, got ${t}`);
    }
  }
  function ce(t, e, a = "") {
    let n = fn(t),
      s = t?.length,
      o = e !== void 0;
    if (!n || (o && s !== e)) {
      let r = a && `"${a}" `,
        i = o ? ` of length ${e}` : "",
        c = n ? `length=${s}` : `type=${typeof t}`;
      throw new Error(r + "expected Uint8Array" + i + ", got " + c);
    }
    return t;
  }
  function cs(t) {
    if (typeof t != "function" || typeof t.create != "function")
      throw new Error("Hash must wrapped by utils.createHasher");
    nt(t.outputLen), nt(t.blockLen);
  }
  function La(t, e = !0) {
    if (t.destroyed) throw new Error("Hash instance has been destroyed");
    if (e && t.finished)
      throw new Error("Hash#digest() has already been called");
  }
  function sc(t, e) {
    ce(t, void 0, "digestInto() output");
    let a = e.outputLen;
    if (t.length < a)
      throw new Error('"digestInto() output" expected to be of length >=' + a);
  }
  function Qt(...t) {
    for (let e = 0; e < t.length; e++) t[e].fill(0);
  }
  function ls(t) {
    return new DataView(t.buffer, t.byteOffset, t.byteLength);
  }
  function We(t, e) {
    return (t << (32 - e)) | (t >>> e);
  }
  var oc =
      typeof Uint8Array.from([]).toHex == "function" &&
      typeof Uint8Array.fromHex == "function",
    If = Array.from({ length: 256 }, (t, e) => e.toString(16).padStart(2, "0"));
  function ea(t) {
    if ((ce(t), oc)) return t.toHex();
    let e = "";
    for (let a = 0; a < t.length; a++) e += If[t[a]];
    return e;
  }
  var lt = { _0: 48, _9: 57, A: 65, F: 70, a: 97, f: 102 };
  function nc(t) {
    if (t >= lt._0 && t <= lt._9) return t - lt._0;
    if (t >= lt.A && t <= lt.F) return t - (lt.A - 10);
    if (t >= lt.a && t <= lt.f) return t - (lt.a - 10);
  }
  function ta(t) {
    if (typeof t != "string")
      throw new Error("hex string expected, got " + typeof t);
    if (oc) return Uint8Array.fromHex(t);
    let e = t.length,
      a = e / 2;
    if (e % 2)
      throw new Error("hex string expected, got unpadded hex of length " + e);
    let n = new Uint8Array(a);
    for (let s = 0, o = 0; s < a; s++, o += 2) {
      let r = nc(t.charCodeAt(o)),
        i = nc(t.charCodeAt(o + 1));
      if (r === void 0 || i === void 0) {
        let c = t[o] + t[o + 1];
        throw new Error(
          'hex string expected, got non-hex character "' +
            c +
            '" at index ' +
            o,
        );
      }
      n[s] = r * 16 + i;
    }
    return n;
  }
  function De(...t) {
    let e = 0;
    for (let n = 0; n < t.length; n++) {
      let s = t[n];
      ce(s), (e += s.length);
    }
    let a = new Uint8Array(e);
    for (let n = 0, s = 0; n < t.length; n++) {
      let o = t[n];
      a.set(o, s), (s += o.length);
    }
    return a;
  }
  function rc(t, e = {}) {
    let a = (s, o) => t(o).update(s).digest(),
      n = t(void 0);
    return (
      (a.outputLen = n.outputLen),
      (a.blockLen = n.blockLen),
      (a.create = (s) => t(s)),
      Object.assign(a, e),
      Object.freeze(a)
    );
  }
  function aa(t = 32) {
    let e = typeof globalThis == "object" ? globalThis.crypto : null;
    if (typeof e?.getRandomValues != "function")
      throw new Error("crypto.getRandomValues must be defined");
    return e.getRandomValues(new Uint8Array(t));
  }
  var ic = (t) => ({
    oid: Uint8Array.from([6, 9, 96, 134, 72, 1, 101, 3, 4, 2, t]),
  });
  function cc(t, e, a) {
    return (t & e) ^ (~t & a);
  }
  function lc(t, e, a) {
    return (t & e) ^ (t & a) ^ (e & a);
  }
  var ds = class {
      blockLen;
      outputLen;
      padOffset;
      isLE;
      buffer;
      view;
      finished = !1;
      length = 0;
      pos = 0;
      destroyed = !1;
      constructor(e, a, n, s) {
        (this.blockLen = e),
          (this.outputLen = a),
          (this.padOffset = n),
          (this.isLE = s),
          (this.buffer = new Uint8Array(e)),
          (this.view = ls(this.buffer));
      }
      update(e) {
        La(this), ce(e);
        let { view: a, buffer: n, blockLen: s } = this,
          o = e.length;
        for (let r = 0; r < o; ) {
          let i = Math.min(s - this.pos, o - r);
          if (i === s) {
            let c = ls(e);
            for (; s <= o - r; r += s) this.process(c, r);
            continue;
          }
          n.set(e.subarray(r, r + i), this.pos),
            (this.pos += i),
            (r += i),
            this.pos === s && (this.process(a, 0), (this.pos = 0));
        }
        return (this.length += e.length), this.roundClean(), this;
      }
      digestInto(e) {
        La(this), sc(e, this), (this.finished = !0);
        let { buffer: a, view: n, blockLen: s, isLE: o } = this,
          { pos: r } = this;
        (a[r++] = 128),
          Qt(this.buffer.subarray(r)),
          this.padOffset > s - r && (this.process(n, 0), (r = 0));
        for (let d = r; d < s; d++) a[d] = 0;
        n.setBigUint64(s - 8, BigInt(this.length * 8), o), this.process(n, 0);
        let i = ls(e),
          c = this.outputLen;
        if (c % 4) throw new Error("_sha2: outputLen must be aligned to 32bit");
        let l = c / 4,
          u = this.get();
        if (l > u.length) throw new Error("_sha2: outputLen bigger than state");
        for (let d = 0; d < l; d++) i.setUint32(4 * d, u[d], o);
      }
      digest() {
        let { buffer: e, outputLen: a } = this;
        this.digestInto(e);
        let n = e.slice(0, a);
        return this.destroy(), n;
      }
      _cloneInto(e) {
        (e ||= new this.constructor()), e.set(...this.get());
        let {
          blockLen: a,
          buffer: n,
          length: s,
          finished: o,
          destroyed: r,
          pos: i,
        } = this;
        return (
          (e.destroyed = r),
          (e.finished = o),
          (e.length = s),
          (e.pos = i),
          s % a && e.buffer.set(n),
          e
        );
      }
      clone() {
        return this._cloneInto();
      }
    },
    dt = Uint32Array.from([
      1779033703, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924,
      528734635, 1541459225,
    ]);
  var Hf = Uint32Array.from([
      1116352408, 1899447441, 3049323471, 3921009573, 961987163, 1508970993,
      2453635748, 2870763221, 3624381080, 310598401, 607225278, 1426881987,
      1925078388, 2162078206, 2614888103, 3248222580, 3835390401, 4022224774,
      264347078, 604807628, 770255983, 1249150122, 1555081692, 1996064986,
      2554220882, 2821834349, 2952996808, 3210313671, 3336571891, 3584528711,
      113926993, 338241895, 666307205, 773529912, 1294757372, 1396182291,
      1695183700, 1986661051, 2177026350, 2456956037, 2730485921, 2820302411,
      3259730800, 3345764771, 3516065817, 3600352804, 4094571909, 275423344,
      430227734, 506948616, 659060556, 883997877, 958139571, 1322822218,
      1537002063, 1747873779, 1955562222, 2024104815, 2227730452, 2361852424,
      2428436474, 2756734187, 3204031479, 3329325298,
    ]),
    At = new Uint32Array(64),
    Do = class extends ds {
      constructor(e) {
        super(64, e, 8, !1);
      }
      get() {
        let { A: e, B: a, C: n, D: s, E: o, F: r, G: i, H: c } = this;
        return [e, a, n, s, o, r, i, c];
      }
      set(e, a, n, s, o, r, i, c) {
        (this.A = e | 0),
          (this.B = a | 0),
          (this.C = n | 0),
          (this.D = s | 0),
          (this.E = o | 0),
          (this.F = r | 0),
          (this.G = i | 0),
          (this.H = c | 0);
      }
      process(e, a) {
        for (let d = 0; d < 16; d++, a += 4) At[d] = e.getUint32(a, !1);
        for (let d = 16; d < 64; d++) {
          let f = At[d - 15],
            p = At[d - 2],
            v = We(f, 7) ^ We(f, 18) ^ (f >>> 3),
            k = We(p, 17) ^ We(p, 19) ^ (p >>> 10);
          At[d] = (k + At[d - 7] + v + At[d - 16]) | 0;
        }
        let { A: n, B: s, C: o, D: r, E: i, F: c, G: l, H: u } = this;
        for (let d = 0; d < 64; d++) {
          let f = We(i, 6) ^ We(i, 11) ^ We(i, 25),
            p = (u + f + cc(i, c, l) + Hf[d] + At[d]) | 0,
            k = ((We(n, 2) ^ We(n, 13) ^ We(n, 22)) + lc(n, s, o)) | 0;
          (u = l),
            (l = c),
            (c = i),
            (i = (r + p) | 0),
            (r = o),
            (o = s),
            (s = n),
            (n = (p + k) | 0);
        }
        (n = (n + this.A) | 0),
          (s = (s + this.B) | 0),
          (o = (o + this.C) | 0),
          (r = (r + this.D) | 0),
          (i = (i + this.E) | 0),
          (c = (c + this.F) | 0),
          (l = (l + this.G) | 0),
          (u = (u + this.H) | 0),
          this.set(n, s, o, r, i, c, l, u);
      }
      roundClean() {
        Qt(At);
      }
      destroy() {
        this.set(0, 0, 0, 0, 0, 0, 0, 0), Qt(this.buffer);
      }
    },
    No = class extends Do {
      A = dt[0] | 0;
      B = dt[1] | 0;
      C = dt[2] | 0;
      D = dt[3] | 0;
      E = dt[4] | 0;
      F = dt[5] | 0;
      G = dt[6] | 0;
      H = dt[7] | 0;
      constructor() {
        super(32);
      }
    };
  var us = rc(() => new No(), ic(1));
  var Oo = BigInt(0),
    _o = BigInt(1);
  function pn(t, e = "") {
    if (typeof t != "boolean") {
      let a = e && `"${e}" `;
      throw new Error(a + "expected boolean, got type=" + typeof t);
    }
    return t;
  }
  function dc(t) {
    if (typeof t == "bigint") {
      if (!fs(t)) throw new Error("positive bigint expected, got " + t);
    } else nt(t);
    return t;
  }
  function hn(t) {
    let e = dc(t).toString(16);
    return e.length & 1 ? "0" + e : e;
  }
  function uc(t) {
    if (typeof t != "string")
      throw new Error("hex string expected, got " + typeof t);
    return t === "" ? Oo : BigInt("0x" + t);
  }
  function St(t) {
    return uc(ea(t));
  }
  function jo(t) {
    return uc(ea(Uf(ce(t)).reverse()));
  }
  function ps(t, e) {
    nt(e), (t = dc(t));
    let a = ta(t.toString(16).padStart(e * 2, "0"));
    if (a.length !== e) throw new Error("number too large");
    return a;
  }
  function qo(t, e) {
    return ps(t, e).reverse();
  }
  function Uf(t) {
    return Uint8Array.from(t);
  }
  function fc(t) {
    return Uint8Array.from(t, (e, a) => {
      let n = e.charCodeAt(0);
      if (e.length !== 1 || n > 127)
        throw new Error(
          `string contains non-ASCII character "${t[a]}" with code ${n} at position ${a}`,
        );
      return n;
    });
  }
  var fs = (t) => typeof t == "bigint" && Oo <= t;
  function Df(t, e, a) {
    return fs(t) && fs(e) && fs(a) && e <= t && t < a;
  }
  function pc(t, e, a, n) {
    if (!Df(e, a, n))
      throw new Error(
        "expected valid " + t + ": " + a + " <= n < " + n + ", got " + e,
      );
  }
  function Po(t) {
    let e;
    for (e = 0; t > Oo; t >>= _o, e += 1);
    return e;
  }
  var mn = (t) => (_o << BigInt(t)) - _o;
  function hc(t, e, a) {
    if ((nt(t, "hashLen"), nt(e, "qByteLen"), typeof a != "function"))
      throw new Error("hmacFn must be a function");
    let n = (L) => new Uint8Array(L),
      s = Uint8Array.of(),
      o = Uint8Array.of(0),
      r = Uint8Array.of(1),
      i = 1e3,
      c = n(t),
      l = n(t),
      u = 0,
      d = () => {
        c.fill(1), l.fill(0), (u = 0);
      },
      f = (...L) => a(l, De(c, ...L)),
      p = (L = s) => {
        (l = f(o, L)), (c = f()), L.length !== 0 && ((l = f(r, L)), (c = f()));
      },
      v = () => {
        if (u++ >= i) throw new Error("drbg: tried max amount of iterations");
        let L = 0,
          N = [];
        for (; L < e; ) {
          c = f();
          let V = c.slice();
          N.push(V), (L += c.length);
        }
        return De(...N);
      };
    return (L, N) => {
      d(), p(L);
      let V;
      for (; !(V = N(v())); ) p();
      return d(), V;
    };
  }
  function bn(t, e = {}, a = {}) {
    if (!t || typeof t != "object")
      throw new Error("expected valid options object");
    function n(o, r, i) {
      let c = t[o];
      if (i && c === void 0) return;
      let l = typeof c;
      if (l !== r || c === null)
        throw new Error(`param "${o}" is invalid: expected ${r}, got ${l}`);
    }
    let s = (o, r) => Object.entries(o).forEach(([i, c]) => n(i, c, r));
    s(e, !1), s(a, !0);
  }
  function Vo(t) {
    let e = new WeakMap();
    return (a, ...n) => {
      let s = e.get(a);
      if (s !== void 0) return s;
      let o = t(a, ...n);
      return e.set(a, o), o;
    };
  }
  var Me = BigInt(0),
    Ce = BigInt(1),
    na = BigInt(2),
    gc = BigInt(3),
    xc = BigInt(4),
    yc = BigInt(5),
    Nf = BigInt(7),
    vc = BigInt(8),
    _f = BigInt(9),
    wc = BigInt(16);
  function Ye(t, e) {
    let a = t % e;
    return a >= Me ? a : e + a;
  }
  function Ne(t, e, a) {
    let n = t;
    for (; e-- > Me; ) (n *= n), (n %= a);
    return n;
  }
  function mc(t, e) {
    if (t === Me) throw new Error("invert: expected non-zero number");
    if (e <= Me) throw new Error("invert: expected positive modulus, got " + e);
    let a = Ye(t, e),
      n = e,
      s = Me,
      o = Ce,
      r = Ce,
      i = Me;
    for (; a !== Me; ) {
      let l = n / a,
        u = n % a,
        d = s - r * l,
        f = o - i * l;
      (n = a), (a = u), (s = r), (o = i), (r = d), (i = f);
    }
    if (n !== Ce) throw new Error("invert: does not exist");
    return Ye(s, e);
  }
  function Go(t, e, a) {
    if (!t.eql(t.sqr(e), a)) throw new Error("Cannot find square root");
  }
  function kc(t, e) {
    let a = (t.ORDER + Ce) / xc,
      n = t.pow(e, a);
    return Go(t, n, e), n;
  }
  function Of(t, e) {
    let a = (t.ORDER - yc) / vc,
      n = t.mul(e, na),
      s = t.pow(n, a),
      o = t.mul(e, s),
      r = t.mul(t.mul(o, na), s),
      i = t.mul(o, t.sub(r, t.ONE));
    return Go(t, i, e), i;
  }
  function jf(t) {
    let e = Ta(t),
      a = Ec(t),
      n = a(e, e.neg(e.ONE)),
      s = a(e, n),
      o = a(e, e.neg(n)),
      r = (t + Nf) / wc;
    return (i, c) => {
      let l = i.pow(c, r),
        u = i.mul(l, n),
        d = i.mul(l, s),
        f = i.mul(l, o),
        p = i.eql(i.sqr(u), c),
        v = i.eql(i.sqr(d), c);
      (l = i.cmov(l, u, p)), (u = i.cmov(f, d, v));
      let k = i.eql(i.sqr(u), c),
        L = i.cmov(l, u, k);
      return Go(i, L, c), L;
    };
  }
  function Ec(t) {
    if (t < gc) throw new Error("sqrt is not defined for small field");
    let e = t - Ce,
      a = 0;
    for (; e % na === Me; ) (e /= na), a++;
    let n = na,
      s = Ta(t);
    for (; bc(s, n) === 1; )
      if (n++ > 1e3)
        throw new Error("Cannot find square root: probably non-prime P");
    if (a === 1) return kc;
    let o = s.pow(n, e),
      r = (e + Ce) / na;
    return function (c, l) {
      if (c.is0(l)) return l;
      if (bc(c, l) !== 1) throw new Error("Cannot find square root");
      let u = a,
        d = c.mul(c.ONE, o),
        f = c.pow(l, e),
        p = c.pow(l, r);
      for (; !c.eql(f, c.ONE); ) {
        if (c.is0(f)) return c.ZERO;
        let v = 1,
          k = c.sqr(f);
        for (; !c.eql(k, c.ONE); )
          if ((v++, (k = c.sqr(k)), v === u))
            throw new Error("Cannot find square root");
        let L = Ce << BigInt(u - v - 1),
          N = c.pow(d, L);
        (u = v), (d = c.sqr(N)), (f = c.mul(f, d)), (p = c.mul(p, N));
      }
      return p;
    };
  }
  function qf(t) {
    return t % xc === gc
      ? kc
      : t % vc === yc
        ? Of
        : t % wc === _f
          ? jf(t)
          : Ec(t);
  }
  var Pf = [
    "create",
    "isValid",
    "is0",
    "neg",
    "inv",
    "sqrt",
    "sqr",
    "eql",
    "add",
    "sub",
    "mul",
    "pow",
    "div",
    "addN",
    "subN",
    "mulN",
    "sqrN",
  ];
  function Ko(t) {
    let e = { ORDER: "bigint", BYTES: "number", BITS: "number" },
      a = Pf.reduce((n, s) => ((n[s] = "function"), n), e);
    return bn(t, a), t;
  }
  function Vf(t, e, a) {
    if (a < Me) throw new Error("invalid exponent, negatives unsupported");
    if (a === Me) return t.ONE;
    if (a === Ce) return e;
    let n = t.ONE,
      s = e;
    for (; a > Me; ) a & Ce && (n = t.mul(n, s)), (s = t.sqr(s)), (a >>= Ce);
    return n;
  }
  function hs(t, e, a = !1) {
    let n = new Array(e.length).fill(a ? t.ZERO : void 0),
      s = e.reduce(
        (r, i, c) => (t.is0(i) ? r : ((n[c] = r), t.mul(r, i))),
        t.ONE,
      ),
      o = t.inv(s);
    return (
      e.reduceRight(
        (r, i, c) => (t.is0(i) ? r : ((n[c] = t.mul(r, n[c])), t.mul(r, i))),
        o,
      ),
      n
    );
  }
  function bc(t, e) {
    let a = (t.ORDER - Ce) / na,
      n = t.pow(e, a),
      s = t.eql(n, t.ONE),
      o = t.eql(n, t.ZERO),
      r = t.eql(n, t.neg(t.ONE));
    if (!s && !o && !r) throw new Error("invalid Legendre symbol result");
    return s ? 1 : o ? 0 : -1;
  }
  function zf(t, e) {
    e !== void 0 && nt(e);
    let a = e !== void 0 ? e : t.toString(2).length,
      n = Math.ceil(a / 8);
    return { nBitLength: a, nByteLength: n };
  }
  var zo = class {
    ORDER;
    BITS;
    BYTES;
    isLE;
    ZERO = Me;
    ONE = Ce;
    _lengths;
    _sqrt;
    _mod;
    constructor(e, a = {}) {
      if (e <= Me)
        throw new Error("invalid field: expected ORDER > 0, got " + e);
      let n;
      (this.isLE = !1),
        a != null &&
          typeof a == "object" &&
          (typeof a.BITS == "number" && (n = a.BITS),
          typeof a.sqrt == "function" && (this.sqrt = a.sqrt),
          typeof a.isLE == "boolean" && (this.isLE = a.isLE),
          a.allowedLengths && (this._lengths = a.allowedLengths?.slice()),
          typeof a.modFromBytes == "boolean" && (this._mod = a.modFromBytes));
      let { nBitLength: s, nByteLength: o } = zf(e, n);
      if (o > 2048)
        throw new Error("invalid field: expected ORDER of <= 2048 bytes");
      (this.ORDER = e),
        (this.BITS = s),
        (this.BYTES = o),
        (this._sqrt = void 0),
        Object.preventExtensions(this);
    }
    create(e) {
      return Ye(e, this.ORDER);
    }
    isValid(e) {
      if (typeof e != "bigint")
        throw new Error(
          "invalid field element: expected bigint, got " + typeof e,
        );
      return Me <= e && e < this.ORDER;
    }
    is0(e) {
      return e === Me;
    }
    isValidNot0(e) {
      return !this.is0(e) && this.isValid(e);
    }
    isOdd(e) {
      return (e & Ce) === Ce;
    }
    neg(e) {
      return Ye(-e, this.ORDER);
    }
    eql(e, a) {
      return e === a;
    }
    sqr(e) {
      return Ye(e * e, this.ORDER);
    }
    add(e, a) {
      return Ye(e + a, this.ORDER);
    }
    sub(e, a) {
      return Ye(e - a, this.ORDER);
    }
    mul(e, a) {
      return Ye(e * a, this.ORDER);
    }
    pow(e, a) {
      return Vf(this, e, a);
    }
    div(e, a) {
      return Ye(e * mc(a, this.ORDER), this.ORDER);
    }
    sqrN(e) {
      return e * e;
    }
    addN(e, a) {
      return e + a;
    }
    subN(e, a) {
      return e - a;
    }
    mulN(e, a) {
      return e * a;
    }
    inv(e) {
      return mc(e, this.ORDER);
    }
    sqrt(e) {
      return this._sqrt || (this._sqrt = qf(this.ORDER)), this._sqrt(this, e);
    }
    toBytes(e) {
      return this.isLE ? qo(e, this.BYTES) : ps(e, this.BYTES);
    }
    fromBytes(e, a = !1) {
      ce(e);
      let { _lengths: n, BYTES: s, isLE: o, ORDER: r, _mod: i } = this;
      if (n) {
        if (!n.includes(e.length) || e.length > s)
          throw new Error(
            "Field.fromBytes: expected " + n + " bytes, got " + e.length,
          );
        let l = new Uint8Array(s);
        l.set(e, o ? 0 : l.length - e.length), (e = l);
      }
      if (e.length !== s)
        throw new Error(
          "Field.fromBytes: expected " + s + " bytes, got " + e.length,
        );
      let c = o ? jo(e) : St(e);
      if ((i && (c = Ye(c, r)), !a && !this.isValid(c)))
        throw new Error("invalid field element: outside of range 0..ORDER");
      return c;
    }
    invertBatch(e) {
      return hs(this, e);
    }
    cmov(e, a, n) {
      return n ? a : e;
    }
  };
  function Ta(t, e = {}) {
    return new zo(t, e);
  }
  function $c(t) {
    if (typeof t != "bigint") throw new Error("field order must be bigint");
    let e = t.toString(2).length;
    return Math.ceil(e / 8);
  }
  function Fo(t) {
    let e = $c(t);
    return e + Math.ceil(e / 2);
  }
  function ms(t, e, a = !1) {
    ce(t);
    let n = t.length,
      s = $c(e),
      o = Fo(e);
    if (n < 16 || n < o || n > 1024)
      throw new Error("expected " + o + "-1024 bytes of input, got " + n);
    let r = a ? jo(t) : St(t),
      i = Ye(r, e - Ce) + Ce;
    return a ? qo(i, s) : ps(i, s);
  }
  var Ba = BigInt(0),
    sa = BigInt(1);
  function gn(t, e) {
    let a = e.negate();
    return t ? a : e;
  }
  function Jo(t, e) {
    let a = hs(
      t.Fp,
      e.map((n) => n.Z),
    );
    return e.map((n, s) => t.fromAffine(n.toAffine(a[s])));
  }
  function Lc(t, e) {
    if (!Number.isSafeInteger(t) || t <= 0 || t > e)
      throw new Error(
        "invalid window size, expected [1.." + e + "], got W=" + t,
      );
  }
  function Wo(t, e) {
    Lc(t, e);
    let a = Math.ceil(e / t) + 1,
      n = 2 ** (t - 1),
      s = 2 ** t,
      o = mn(t),
      r = BigInt(t);
    return { windows: a, windowSize: n, mask: o, maxNumber: s, shiftBy: r };
  }
  function Ac(t, e, a) {
    let { windowSize: n, mask: s, maxNumber: o, shiftBy: r } = a,
      i = Number(t & s),
      c = t >> r;
    i > n && ((i -= o), (c += sa));
    let l = e * n,
      u = l + Math.abs(i) - 1,
      d = i === 0,
      f = i < 0,
      p = e % 2 !== 0;
    return { nextN: c, offset: u, isZero: d, isNeg: f, isNegF: p, offsetF: l };
  }
  var Yo = new WeakMap(),
    Tc = new WeakMap();
  function Zo(t) {
    return Tc.get(t) || 1;
  }
  function Sc(t) {
    if (t !== Ba) throw new Error("invalid wNAF");
  }
  var bs = class {
    BASE;
    ZERO;
    Fn;
    bits;
    constructor(e, a) {
      (this.BASE = e.BASE),
        (this.ZERO = e.ZERO),
        (this.Fn = e.Fn),
        (this.bits = a);
    }
    _unsafeLadder(e, a, n = this.ZERO) {
      let s = e;
      for (; a > Ba; ) a & sa && (n = n.add(s)), (s = s.double()), (a >>= sa);
      return n;
    }
    precomputeWindow(e, a) {
      let { windows: n, windowSize: s } = Wo(a, this.bits),
        o = [],
        r = e,
        i = r;
      for (let c = 0; c < n; c++) {
        (i = r), o.push(i);
        for (let l = 1; l < s; l++) (i = i.add(r)), o.push(i);
        r = i.double();
      }
      return o;
    }
    wNAF(e, a, n) {
      if (!this.Fn.isValid(n)) throw new Error("invalid scalar");
      let s = this.ZERO,
        o = this.BASE,
        r = Wo(e, this.bits);
      for (let i = 0; i < r.windows; i++) {
        let {
          nextN: c,
          offset: l,
          isZero: u,
          isNeg: d,
          isNegF: f,
          offsetF: p,
        } = Ac(n, i, r);
        (n = c), u ? (o = o.add(gn(f, a[p]))) : (s = s.add(gn(d, a[l])));
      }
      return Sc(n), { p: s, f: o };
    }
    wNAFUnsafe(e, a, n, s = this.ZERO) {
      let o = Wo(e, this.bits);
      for (let r = 0; r < o.windows && n !== Ba; r++) {
        let { nextN: i, offset: c, isZero: l, isNeg: u } = Ac(n, r, o);
        if (((n = i), !l)) {
          let d = a[c];
          s = s.add(u ? d.negate() : d);
        }
      }
      return Sc(n), s;
    }
    getPrecomputes(e, a, n) {
      let s = Yo.get(a);
      return (
        s ||
          ((s = this.precomputeWindow(a, e)),
          e !== 1 && (typeof n == "function" && (s = n(s)), Yo.set(a, s))),
        s
      );
    }
    cached(e, a, n) {
      let s = Zo(e);
      return this.wNAF(s, this.getPrecomputes(s, e, n), a);
    }
    unsafe(e, a, n, s) {
      let o = Zo(e);
      return o === 1
        ? this._unsafeLadder(e, a, s)
        : this.wNAFUnsafe(o, this.getPrecomputes(o, e, n), a, s);
    }
    createCache(e, a) {
      Lc(a, this.bits), Tc.set(e, a), Yo.delete(e);
    }
    hasCache(e) {
      return Zo(e) !== 1;
    }
  };
  function Bc(t, e, a, n) {
    let s = e,
      o = t.ZERO,
      r = t.ZERO;
    for (; a > Ba || n > Ba; )
      a & sa && (o = o.add(s)),
        n & sa && (r = r.add(s)),
        (s = s.double()),
        (a >>= sa),
        (n >>= sa);
    return { p1: o, p2: r };
  }
  function Cc(t, e, a) {
    if (e) {
      if (e.ORDER !== t)
        throw new Error("Field.ORDER must match order: Fp == p, Fn == n");
      return Ko(e), e;
    } else return Ta(t, { isLE: a });
  }
  function Mc(t, e, a = {}, n) {
    if ((n === void 0 && (n = t === "edwards"), !e || typeof e != "object"))
      throw new Error(`expected valid ${t} CURVE object`);
    for (let c of ["p", "n", "h"]) {
      let l = e[c];
      if (!(typeof l == "bigint" && l > Ba))
        throw new Error(`CURVE.${c} must be positive bigint`);
    }
    let s = Cc(e.p, a.Fp, n),
      o = Cc(e.n, a.Fn, n),
      i = ["Gx", "Gy", "a", t === "weierstrass" ? "b" : "d"];
    for (let c of i)
      if (!s.isValid(e[c]))
        throw new Error(`CURVE.${c} must be valid field element of CURVE.Fp`);
    return (
      (e = Object.freeze(Object.assign({}, e))), { CURVE: e, Fp: s, Fn: o }
    );
  }
  function gs(t, e) {
    return function (n) {
      let s = t(n);
      return { secretKey: s, publicKey: e(s) };
    };
  }
  var xs = class {
      oHash;
      iHash;
      blockLen;
      outputLen;
      finished = !1;
      destroyed = !1;
      constructor(e, a) {
        if (
          (cs(e),
          ce(a, void 0, "key"),
          (this.iHash = e.create()),
          typeof this.iHash.update != "function")
        )
          throw new Error(
            "Expected instance of class which extends utils.Hash",
          );
        (this.blockLen = this.iHash.blockLen),
          (this.outputLen = this.iHash.outputLen);
        let n = this.blockLen,
          s = new Uint8Array(n);
        s.set(a.length > n ? e.create().update(a).digest() : a);
        for (let o = 0; o < s.length; o++) s[o] ^= 54;
        this.iHash.update(s), (this.oHash = e.create());
        for (let o = 0; o < s.length; o++) s[o] ^= 106;
        this.oHash.update(s), Qt(s);
      }
      update(e) {
        return La(this), this.iHash.update(e), this;
      }
      digestInto(e) {
        La(this),
          ce(e, this.outputLen, "output"),
          (this.finished = !0),
          this.iHash.digestInto(e),
          this.oHash.update(e),
          this.oHash.digestInto(e),
          this.destroy();
      }
      digest() {
        let e = new Uint8Array(this.oHash.outputLen);
        return this.digestInto(e), e;
      }
      _cloneInto(e) {
        e ||= Object.create(Object.getPrototypeOf(this), {});
        let {
          oHash: a,
          iHash: n,
          finished: s,
          destroyed: o,
          blockLen: r,
          outputLen: i,
        } = this;
        return (
          (e = e),
          (e.finished = s),
          (e.destroyed = o),
          (e.blockLen = r),
          (e.outputLen = i),
          (e.oHash = a._cloneInto(e.oHash)),
          (e.iHash = n._cloneInto(e.iHash)),
          e
        );
      }
      clone() {
        return this._cloneInto();
      }
      destroy() {
        (this.destroyed = !0), this.oHash.destroy(), this.iHash.destroy();
      }
    },
    Xo = (t, e, a) => new xs(t, e).update(a).digest();
  Xo.create = (t, e) => new xs(t, e);
  var Rc = (t, e) => (t + (t >= 0 ? e : -e) / Ic) / e;
  function Gf(t, e, a) {
    let [[n, s], [o, r]] = e,
      i = Rc(r * t, a),
      c = Rc(-s * t, a),
      l = t - i * n - c * o,
      u = -i * s - c * r,
      d = l < ut,
      f = u < ut;
    d && (l = -l), f && (u = -u);
    let p = mn(Math.ceil(Po(a) / 2)) + Ma;
    if (l < ut || l >= p || u < ut || u >= p)
      throw new Error("splitScalar (endomorphism): failed, k=" + t);
    return { k1neg: d, k1: l, k2neg: f, k2: u };
  }
  function er(t) {
    if (!["compact", "recovered", "der"].includes(t))
      throw new Error(
        'Signature format must be "compact", "recovered", or "der"',
      );
    return t;
  }
  function Qo(t, e) {
    let a = {};
    for (let n of Object.keys(e)) a[n] = t[n] === void 0 ? e[n] : t[n];
    return (
      pn(a.lowS, "lowS"),
      pn(a.prehash, "prehash"),
      a.format !== void 0 && er(a.format),
      a
    );
  }
  var tr = class extends Error {
      constructor(e = "") {
        super(e);
      }
    },
    Ct = {
      Err: tr,
      _tlv: {
        encode: (t, e) => {
          let { Err: a } = Ct;
          if (t < 0 || t > 256) throw new a("tlv.encode: wrong tag");
          if (e.length & 1) throw new a("tlv.encode: unpadded data");
          let n = e.length / 2,
            s = hn(n);
          if ((s.length / 2) & 128)
            throw new a("tlv.encode: long form length too big");
          let o = n > 127 ? hn((s.length / 2) | 128) : "";
          return hn(t) + o + s + e;
        },
        decode(t, e) {
          let { Err: a } = Ct,
            n = 0;
          if (t < 0 || t > 256) throw new a("tlv.encode: wrong tag");
          if (e.length < 2 || e[n++] !== t)
            throw new a("tlv.decode: wrong tlv");
          let s = e[n++],
            o = !!(s & 128),
            r = 0;
          if (!o) r = s;
          else {
            let c = s & 127;
            if (!c)
              throw new a("tlv.decode(long): indefinite length not supported");
            if (c > 4) throw new a("tlv.decode(long): byte length is too big");
            let l = e.subarray(n, n + c);
            if (l.length !== c)
              throw new a("tlv.decode: length bytes not complete");
            if (l[0] === 0) throw new a("tlv.decode(long): zero leftmost byte");
            for (let u of l) r = (r << 8) | u;
            if (((n += c), r < 128))
              throw new a("tlv.decode(long): not minimal encoding");
          }
          let i = e.subarray(n, n + r);
          if (i.length !== r) throw new a("tlv.decode: wrong value length");
          return { v: i, l: e.subarray(n + r) };
        },
      },
      _int: {
        encode(t) {
          let { Err: e } = Ct;
          if (t < ut) throw new e("integer: negative integers are not allowed");
          let a = hn(t);
          if ((Number.parseInt(a[0], 16) & 8 && (a = "00" + a), a.length & 1))
            throw new e("unexpected DER parsing assertion: unpadded hex");
          return a;
        },
        decode(t) {
          let { Err: e } = Ct;
          if (t[0] & 128) throw new e("invalid signature integer: negative");
          if (t[0] === 0 && !(t[1] & 128))
            throw new e("invalid signature integer: unnecessary leading zero");
          return St(t);
        },
      },
      toSig(t) {
        let { Err: e, _int: a, _tlv: n } = Ct,
          s = ce(t, void 0, "signature"),
          { v: o, l: r } = n.decode(48, s);
        if (r.length)
          throw new e("invalid signature: left bytes after parsing");
        let { v: i, l: c } = n.decode(2, o),
          { v: l, l: u } = n.decode(2, c);
        if (u.length)
          throw new e("invalid signature: left bytes after parsing");
        return { r: a.decode(i), s: a.decode(l) };
      },
      hexFromSig(t) {
        let { _tlv: e, _int: a } = Ct,
          n = e.encode(2, a.encode(t.r)),
          s = e.encode(2, a.encode(t.s)),
          o = n + s;
        return e.encode(48, o);
      },
    },
    ut = BigInt(0),
    Ma = BigInt(1),
    Ic = BigInt(2),
    ys = BigInt(3),
    Kf = BigInt(4);
  function Hc(t, e = {}) {
    let a = Mc("weierstrass", t, e),
      { Fp: n, Fn: s } = a,
      o = a.CURVE,
      { h: r, n: i } = o;
    bn(
      e,
      {},
      {
        allowInfinityPoint: "boolean",
        clearCofactor: "function",
        isTorsionFree: "function",
        fromBytes: "function",
        toBytes: "function",
        endo: "object",
      },
    );
    let { endo: c } = e;
    if (
      c &&
      (!n.is0(o.a) || typeof c.beta != "bigint" || !Array.isArray(c.basises))
    )
      throw new Error(
        'invalid endo: expected "beta": bigint and "basises": array',
      );
    let l = Dc(n, s);
    function u() {
      if (!n.isOdd)
        throw new Error(
          "compression is not supported: Field does not have .isOdd()",
        );
    }
    function d(R, E, B) {
      let { x: S, y: D } = E.toAffine(),
        I = n.toBytes(S);
      if ((pn(B, "isCompressed"), B)) {
        u();
        let j = !n.isOdd(D);
        return De(Uc(j), I);
      } else return De(Uint8Array.of(4), I, n.toBytes(D));
    }
    function f(R) {
      ce(R, void 0, "Point");
      let { publicKey: E, publicKeyUncompressed: B } = l,
        S = R.length,
        D = R[0],
        I = R.subarray(1);
      if (S === E && (D === 2 || D === 3)) {
        let j = n.fromBytes(I);
        if (!n.isValid(j))
          throw new Error("bad point: is not on curve, wrong x");
        let U = k(j),
          G;
        try {
          G = n.sqrt(U);
        } catch (te) {
          let re = te instanceof Error ? ": " + te.message : "";
          throw new Error("bad point: is not on curve, sqrt error" + re);
        }
        u();
        let W = n.isOdd(G);
        return ((D & 1) === 1) !== W && (G = n.neg(G)), { x: j, y: G };
      } else if (S === B && D === 4) {
        let j = n.BYTES,
          U = n.fromBytes(I.subarray(0, j)),
          G = n.fromBytes(I.subarray(j, j * 2));
        if (!L(U, G)) throw new Error("bad point: is not on curve");
        return { x: U, y: G };
      } else
        throw new Error(
          `bad point: got length ${S}, expected compressed=${E} or uncompressed=${B}`,
        );
    }
    let p = e.toBytes || d,
      v = e.fromBytes || f;
    function k(R) {
      let E = n.sqr(R),
        B = n.mul(E, R);
      return n.add(n.add(B, n.mul(R, o.a)), o.b);
    }
    function L(R, E) {
      let B = n.sqr(E),
        S = k(R);
      return n.eql(B, S);
    }
    if (!L(o.Gx, o.Gy)) throw new Error("bad curve params: generator point");
    let N = n.mul(n.pow(o.a, ys), Kf),
      V = n.mul(n.sqr(o.b), BigInt(27));
    if (n.is0(n.add(N, V))) throw new Error("bad curve params: a or b");
    function O(R, E, B = !1) {
      if (!n.isValid(E) || (B && n.is0(E)))
        throw new Error(`bad point coordinate ${R}`);
      return E;
    }
    function q(R) {
      if (!(R instanceof m)) throw new Error("Weierstrass Point expected");
    }
    function K(R) {
      if (!c || !c.basises) throw new Error("no endo");
      return Gf(R, c.basises, s.ORDER);
    }
    let T = Vo((R, E) => {
        let { X: B, Y: S, Z: D } = R;
        if (n.eql(D, n.ONE)) return { x: B, y: S };
        let I = R.is0();
        E == null && (E = I ? n.ONE : n.inv(D));
        let j = n.mul(B, E),
          U = n.mul(S, E),
          G = n.mul(D, E);
        if (I) return { x: n.ZERO, y: n.ZERO };
        if (!n.eql(G, n.ONE)) throw new Error("invZ was invalid");
        return { x: j, y: U };
      }),
      g = Vo((R) => {
        if (R.is0()) {
          if (e.allowInfinityPoint && !n.is0(R.Y)) return;
          throw new Error("bad point: ZERO");
        }
        let { x: E, y: B } = R.toAffine();
        if (!n.isValid(E) || !n.isValid(B))
          throw new Error("bad point: x or y not field elements");
        if (!L(E, B)) throw new Error("bad point: equation left != right");
        if (!R.isTorsionFree())
          throw new Error("bad point: not in prime-order subgroup");
        return !0;
      });
    function w(R, E, B, S, D) {
      return (
        (B = new m(n.mul(B.X, R), B.Y, B.Z)),
        (E = gn(S, E)),
        (B = gn(D, B)),
        E.add(B)
      );
    }
    class m {
      static BASE = new m(o.Gx, o.Gy, n.ONE);
      static ZERO = new m(n.ZERO, n.ONE, n.ZERO);
      static Fp = n;
      static Fn = s;
      X;
      Y;
      Z;
      constructor(E, B, S) {
        (this.X = O("x", E)),
          (this.Y = O("y", B, !0)),
          (this.Z = O("z", S)),
          Object.freeze(this);
      }
      static CURVE() {
        return o;
      }
      static fromAffine(E) {
        let { x: B, y: S } = E || {};
        if (!E || !n.isValid(B) || !n.isValid(S))
          throw new Error("invalid affine point");
        if (E instanceof m) throw new Error("projective point not allowed");
        return n.is0(B) && n.is0(S) ? m.ZERO : new m(B, S, n.ONE);
      }
      static fromBytes(E) {
        let B = m.fromAffine(v(ce(E, void 0, "point")));
        return B.assertValidity(), B;
      }
      static fromHex(E) {
        return m.fromBytes(ta(E));
      }
      get x() {
        return this.toAffine().x;
      }
      get y() {
        return this.toAffine().y;
      }
      precompute(E = 8, B = !0) {
        return A.createCache(this, E), B || this.multiply(ys), this;
      }
      assertValidity() {
        g(this);
      }
      hasEvenY() {
        let { y: E } = this.toAffine();
        if (!n.isOdd) throw new Error("Field doesn't support isOdd");
        return !n.isOdd(E);
      }
      equals(E) {
        q(E);
        let { X: B, Y: S, Z: D } = this,
          { X: I, Y: j, Z: U } = E,
          G = n.eql(n.mul(B, U), n.mul(I, D)),
          W = n.eql(n.mul(S, U), n.mul(j, D));
        return G && W;
      }
      negate() {
        return new m(this.X, n.neg(this.Y), this.Z);
      }
      double() {
        let { a: E, b: B } = o,
          S = n.mul(B, ys),
          { X: D, Y: I, Z: j } = this,
          U = n.ZERO,
          G = n.ZERO,
          W = n.ZERO,
          Y = n.mul(D, D),
          te = n.mul(I, I),
          re = n.mul(j, j),
          ae = n.mul(D, I);
        return (
          (ae = n.add(ae, ae)),
          (W = n.mul(D, j)),
          (W = n.add(W, W)),
          (U = n.mul(E, W)),
          (G = n.mul(S, re)),
          (G = n.add(U, G)),
          (U = n.sub(te, G)),
          (G = n.add(te, G)),
          (G = n.mul(U, G)),
          (U = n.mul(ae, U)),
          (W = n.mul(S, W)),
          (re = n.mul(E, re)),
          (ae = n.sub(Y, re)),
          (ae = n.mul(E, ae)),
          (ae = n.add(ae, W)),
          (W = n.add(Y, Y)),
          (Y = n.add(W, Y)),
          (Y = n.add(Y, re)),
          (Y = n.mul(Y, ae)),
          (G = n.add(G, Y)),
          (re = n.mul(I, j)),
          (re = n.add(re, re)),
          (Y = n.mul(re, ae)),
          (U = n.sub(U, Y)),
          (W = n.mul(re, te)),
          (W = n.add(W, W)),
          (W = n.add(W, W)),
          new m(U, G, W)
        );
      }
      add(E) {
        q(E);
        let { X: B, Y: S, Z: D } = this,
          { X: I, Y: j, Z: U } = E,
          G = n.ZERO,
          W = n.ZERO,
          Y = n.ZERO,
          te = o.a,
          re = n.mul(o.b, ys),
          ae = n.mul(B, I),
          de = n.mul(S, j),
          he = n.mul(D, U),
          we = n.add(B, S),
          ue = n.add(I, j);
        (we = n.mul(we, ue)),
          (ue = n.add(ae, de)),
          (we = n.sub(we, ue)),
          (ue = n.add(B, D));
        let Se = n.add(I, U);
        return (
          (ue = n.mul(ue, Se)),
          (Se = n.add(ae, he)),
          (ue = n.sub(ue, Se)),
          (Se = n.add(S, D)),
          (G = n.add(j, U)),
          (Se = n.mul(Se, G)),
          (G = n.add(de, he)),
          (Se = n.sub(Se, G)),
          (Y = n.mul(te, ue)),
          (G = n.mul(re, he)),
          (Y = n.add(G, Y)),
          (G = n.sub(de, Y)),
          (Y = n.add(de, Y)),
          (W = n.mul(G, Y)),
          (de = n.add(ae, ae)),
          (de = n.add(de, ae)),
          (he = n.mul(te, he)),
          (ue = n.mul(re, ue)),
          (de = n.add(de, he)),
          (he = n.sub(ae, he)),
          (he = n.mul(te, he)),
          (ue = n.add(ue, he)),
          (ae = n.mul(de, ue)),
          (W = n.add(W, ae)),
          (ae = n.mul(Se, ue)),
          (G = n.mul(we, G)),
          (G = n.sub(G, ae)),
          (ae = n.mul(we, de)),
          (Y = n.mul(Se, Y)),
          (Y = n.add(Y, ae)),
          new m(G, W, Y)
        );
      }
      subtract(E) {
        return this.add(E.negate());
      }
      is0() {
        return this.equals(m.ZERO);
      }
      multiply(E) {
        let { endo: B } = e;
        if (!s.isValidNot0(E)) throw new Error("invalid scalar: out of range");
        let S,
          D,
          I = (j) => A.cached(this, j, (U) => Jo(m, U));
        if (B) {
          let { k1neg: j, k1: U, k2neg: G, k2: W } = K(E),
            { p: Y, f: te } = I(U),
            { p: re, f: ae } = I(W);
          (D = te.add(ae)), (S = w(B.beta, Y, re, j, G));
        } else {
          let { p: j, f: U } = I(E);
          (S = j), (D = U);
        }
        return Jo(m, [S, D])[0];
      }
      multiplyUnsafe(E) {
        let { endo: B } = e,
          S = this;
        if (!s.isValid(E)) throw new Error("invalid scalar: out of range");
        if (E === ut || S.is0()) return m.ZERO;
        if (E === Ma) return S;
        if (A.hasCache(this)) return this.multiply(E);
        if (B) {
          let { k1neg: D, k1: I, k2neg: j, k2: U } = K(E),
            { p1: G, p2: W } = Bc(m, S, I, U);
          return w(B.beta, G, W, D, j);
        } else return A.unsafe(S, E);
      }
      toAffine(E) {
        return T(this, E);
      }
      isTorsionFree() {
        let { isTorsionFree: E } = e;
        return r === Ma ? !0 : E ? E(m, this) : A.unsafe(this, i).is0();
      }
      clearCofactor() {
        let { clearCofactor: E } = e;
        return r === Ma ? this : E ? E(m, this) : this.multiplyUnsafe(r);
      }
      isSmallOrder() {
        return this.multiplyUnsafe(r).is0();
      }
      toBytes(E = !0) {
        return pn(E, "isCompressed"), this.assertValidity(), p(m, this, E);
      }
      toHex(E = !0) {
        return ea(this.toBytes(E));
      }
      toString() {
        return `<Point ${this.is0() ? "ZERO" : this.toHex()}>`;
      }
    }
    let $ = s.BITS,
      A = new bs(m, e.endo ? Math.ceil($ / 2) : $);
    return m.BASE.precompute(8), m;
  }
  function Uc(t) {
    return Uint8Array.of(t ? 2 : 3);
  }
  function Dc(t, e) {
    return {
      secretKey: e.BYTES,
      publicKey: 1 + t.BYTES,
      publicKeyUncompressed: 1 + 2 * t.BYTES,
      publicKeyHasPrefix: !0,
      signature: 2 * e.BYTES,
    };
  }
  function Ff(t, e = {}) {
    let { Fn: a } = t,
      n = e.randomBytes || aa,
      s = Object.assign(Dc(t.Fp, a), { seed: Fo(a.ORDER) });
    function o(p) {
      try {
        let v = a.fromBytes(p);
        return a.isValidNot0(v);
      } catch {
        return !1;
      }
    }
    function r(p, v) {
      let { publicKey: k, publicKeyUncompressed: L } = s;
      try {
        let N = p.length;
        return (v === !0 && N !== k) || (v === !1 && N !== L)
          ? !1
          : !!t.fromBytes(p);
      } catch {
        return !1;
      }
    }
    function i(p = n(s.seed)) {
      return ms(ce(p, s.seed, "seed"), a.ORDER);
    }
    function c(p, v = !0) {
      return t.BASE.multiply(a.fromBytes(p)).toBytes(v);
    }
    function l(p) {
      let { secretKey: v, publicKey: k, publicKeyUncompressed: L } = s;
      if (!fn(p) || ("_lengths" in a && a._lengths) || v === k) return;
      let N = ce(p, void 0, "key").length;
      return N === k || N === L;
    }
    function u(p, v, k = !0) {
      if (l(p) === !0) throw new Error("first arg must be private key");
      if (l(v) === !1) throw new Error("second arg must be public key");
      let L = a.fromBytes(p);
      return t.fromBytes(v).multiply(L).toBytes(k);
    }
    let d = { isValidSecretKey: o, isValidPublicKey: r, randomSecretKey: i },
      f = gs(i, c);
    return Object.freeze({
      getPublicKey: c,
      getSharedSecret: u,
      keygen: f,
      Point: t,
      utils: d,
      lengths: s,
    });
  }
  function Nc(t, e, a = {}) {
    cs(e),
      bn(
        a,
        {},
        {
          hmac: "function",
          lowS: "boolean",
          randomBytes: "function",
          bits2int: "function",
          bits2int_modN: "function",
        },
      ),
      (a = Object.assign({}, a));
    let n = a.randomBytes || aa,
      s = a.hmac || ((B, S) => Xo(e, B, S)),
      { Fp: o, Fn: r } = t,
      { ORDER: i, BITS: c } = r,
      {
        keygen: l,
        getPublicKey: u,
        getSharedSecret: d,
        utils: f,
        lengths: p,
      } = Ff(t, a),
      v = {
        prehash: !0,
        lowS: typeof a.lowS == "boolean" ? a.lowS : !0,
        format: "compact",
        extraEntropy: !1,
      },
      k = i * Ic < o.ORDER;
    function L(B) {
      let S = i >> Ma;
      return B > S;
    }
    function N(B, S) {
      if (!r.isValidNot0(S))
        throw new Error(
          `invalid signature ${B}: out of range 1..Point.Fn.ORDER`,
        );
      return S;
    }
    function V() {
      if (k)
        throw new Error(
          '"recovered" sig type is not supported for cofactor >2 curves',
        );
    }
    function O(B, S) {
      er(S);
      let D = p.signature,
        I = S === "compact" ? D : S === "recovered" ? D + 1 : void 0;
      return ce(B, I);
    }
    class q {
      r;
      s;
      recovery;
      constructor(S, D, I) {
        if (((this.r = N("r", S)), (this.s = N("s", D)), I != null)) {
          if ((V(), ![0, 1, 2, 3].includes(I)))
            throw new Error("invalid recovery id");
          this.recovery = I;
        }
        Object.freeze(this);
      }
      static fromBytes(S, D = v.format) {
        O(S, D);
        let I;
        if (D === "der") {
          let { r: W, s: Y } = Ct.toSig(ce(S));
          return new q(W, Y);
        }
        D === "recovered" && ((I = S[0]), (D = "compact"), (S = S.subarray(1)));
        let j = p.signature / 2,
          U = S.subarray(0, j),
          G = S.subarray(j, j * 2);
        return new q(r.fromBytes(U), r.fromBytes(G), I);
      }
      static fromHex(S, D) {
        return this.fromBytes(ta(S), D);
      }
      assertRecovery() {
        let { recovery: S } = this;
        if (S == null) throw new Error("invalid recovery id: must be present");
        return S;
      }
      addRecoveryBit(S) {
        return new q(this.r, this.s, S);
      }
      recoverPublicKey(S) {
        let { r: D, s: I } = this,
          j = this.assertRecovery(),
          U = j === 2 || j === 3 ? D + i : D;
        if (!o.isValid(U))
          throw new Error("invalid recovery id: sig.r+curve.n != R.x");
        let G = o.toBytes(U),
          W = t.fromBytes(De(Uc((j & 1) === 0), G)),
          Y = r.inv(U),
          te = T(ce(S, void 0, "msgHash")),
          re = r.create(-te * Y),
          ae = r.create(I * Y),
          de = t.BASE.multiplyUnsafe(re).add(W.multiplyUnsafe(ae));
        if (de.is0()) throw new Error("invalid recovery: point at infinify");
        return de.assertValidity(), de;
      }
      hasHighS() {
        return L(this.s);
      }
      toBytes(S = v.format) {
        if ((er(S), S === "der")) return ta(Ct.hexFromSig(this));
        let { r: D, s: I } = this,
          j = r.toBytes(D),
          U = r.toBytes(I);
        return S === "recovered"
          ? (V(), De(Uint8Array.of(this.assertRecovery()), j, U))
          : De(j, U);
      }
      toHex(S) {
        return ea(this.toBytes(S));
      }
    }
    let K =
        a.bits2int ||
        function (S) {
          if (S.length > 8192) throw new Error("input is too large");
          let D = St(S),
            I = S.length * 8 - c;
          return I > 0 ? D >> BigInt(I) : D;
        },
      T =
        a.bits2int_modN ||
        function (S) {
          return r.create(K(S));
        },
      g = mn(c);
    function w(B) {
      return pc("num < 2^" + c, B, ut, g), r.toBytes(B);
    }
    function m(B, S) {
      return (
        ce(B, void 0, "message"), S ? ce(e(B), void 0, "prehashed message") : B
      );
    }
    function $(B, S, D) {
      let { lowS: I, prehash: j, extraEntropy: U } = Qo(D, v);
      B = m(B, j);
      let G = T(B),
        W = r.fromBytes(S);
      if (!r.isValidNot0(W)) throw new Error("invalid private key");
      let Y = [w(W), w(G)];
      if (U != null && U !== !1) {
        let de = U === !0 ? n(p.secretKey) : U;
        Y.push(ce(de, void 0, "extraEntropy"));
      }
      let te = De(...Y),
        re = G;
      function ae(de) {
        let he = K(de);
        if (!r.isValidNot0(he)) return;
        let we = r.inv(he),
          ue = t.BASE.multiply(he).toAffine(),
          Se = r.create(ue.x);
        if (Se === ut) return;
        let is = r.create(we * r.create(re + Se * W));
        if (is === ut) return;
        let tc = (ue.x === Se ? 0 : 2) | Number(ue.y & Ma),
          ac = is;
        return (
          I && L(is) && ((ac = r.neg(is)), (tc ^= 1)),
          new q(Se, ac, k ? void 0 : tc)
        );
      }
      return { seed: te, k2sig: ae };
    }
    function A(B, S, D = {}) {
      let { seed: I, k2sig: j } = $(B, S, D);
      return hc(e.outputLen, r.BYTES, s)(I, j).toBytes(D.format);
    }
    function R(B, S, D, I = {}) {
      let { lowS: j, prehash: U, format: G } = Qo(I, v);
      if (((D = ce(D, void 0, "publicKey")), (S = m(S, U)), !fn(B))) {
        let W = B instanceof q ? ", use sig.toBytes()" : "";
        throw new Error("verify expects Uint8Array signature" + W);
      }
      O(B, G);
      try {
        let W = q.fromBytes(B, G),
          Y = t.fromBytes(D);
        if (j && W.hasHighS()) return !1;
        let { r: te, s: re } = W,
          ae = T(S),
          de = r.inv(re),
          he = r.create(ae * de),
          we = r.create(te * de),
          ue = t.BASE.multiplyUnsafe(he).add(Y.multiplyUnsafe(we));
        return ue.is0() ? !1 : r.create(ue.x) === te;
      } catch {
        return !1;
      }
    }
    function E(B, S, D = {}) {
      let { prehash: I } = Qo(D, v);
      return (
        (S = m(S, I)), q.fromBytes(B, "recovered").recoverPublicKey(S).toBytes()
      );
    }
    return Object.freeze({
      keygen: l,
      getPublicKey: u,
      getSharedSecret: d,
      utils: f,
      lengths: p,
      Point: t,
      sign: A,
      verify: R,
      recoverPublicKey: E,
      Signature: q,
      hash: e,
    });
  }
  var ks = {
      p: BigInt(
        "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
      ),
      n: BigInt(
        "0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
      ),
      h: BigInt(1),
      a: BigInt(0),
      b: BigInt(7),
      Gx: BigInt(
        "0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
      ),
      Gy: BigInt(
        "0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8",
      ),
    },
    Wf = {
      beta: BigInt(
        "0x7ae96a2b657c07106e64479eac3434e99cf0497512f58995c1396c28719501ee",
      ),
      basises: [
        [
          BigInt("0x3086d221a7d46bcde86c90e49284eb15"),
          -BigInt("0xe4437ed6010e88286f547fa90abfe4c3"),
        ],
        [
          BigInt("0x114ca50f7a8e2f3f657c1108d9d44cfd8"),
          BigInt("0x3086d221a7d46bcde86c90e49284eb15"),
        ],
      ],
    },
    Yf = BigInt(0),
    ar = BigInt(2);
  function Zf(t) {
    let e = ks.p,
      a = BigInt(3),
      n = BigInt(6),
      s = BigInt(11),
      o = BigInt(22),
      r = BigInt(23),
      i = BigInt(44),
      c = BigInt(88),
      l = (t * t * t) % e,
      u = (l * l * t) % e,
      d = (Ne(u, a, e) * u) % e,
      f = (Ne(d, a, e) * u) % e,
      p = (Ne(f, ar, e) * l) % e,
      v = (Ne(p, s, e) * p) % e,
      k = (Ne(v, o, e) * v) % e,
      L = (Ne(k, i, e) * k) % e,
      N = (Ne(L, c, e) * L) % e,
      V = (Ne(N, i, e) * k) % e,
      O = (Ne(V, a, e) * u) % e,
      q = (Ne(O, r, e) * v) % e,
      K = (Ne(q, n, e) * l) % e,
      T = Ne(K, ar, e);
    if (!vs.eql(vs.sqr(T), t)) throw new Error("Cannot find square root");
    return T;
  }
  var vs = Ta(ks.p, { sqrt: Zf }),
    oa = Hc(ks, { Fp: vs, endo: Wf }),
    ft = Nc(oa, us),
    _c = {};
  function ws(t, ...e) {
    let a = _c[t];
    if (a === void 0) {
      let n = us(fc(t));
      (a = De(n, n)), (_c[t] = a);
    }
    return us(De(a, ...e));
  }
  var sr = (t) => t.toBytes(!0).slice(1),
    or = (t) => t % ar === Yf;
  function nr(t) {
    let { Fn: e, BASE: a } = oa,
      n = e.fromBytes(t),
      s = a.multiply(n);
    return { scalar: or(s.y) ? n : e.neg(n), bytes: sr(s) };
  }
  function jc(t) {
    let e = vs;
    if (!e.isValidNot0(t)) throw new Error("invalid x: Fail if x \u2265 p");
    let a = e.create(t * t),
      n = e.create(a * t + BigInt(7)),
      s = e.sqrt(n);
    or(s) || (s = e.neg(s));
    let o = oa.fromAffine({ x: t, y: s });
    return o.assertValidity(), o;
  }
  var xn = St;
  function qc(...t) {
    return oa.Fn.create(xn(ws("BIP0340/challenge", ...t)));
  }
  function Oc(t) {
    return nr(t).bytes;
  }
  function Jf(t, e, a = aa(32)) {
    let { Fn: n } = oa,
      s = ce(t, void 0, "message"),
      { bytes: o, scalar: r } = nr(e),
      i = ce(a, 32, "auxRand"),
      c = n.toBytes(r ^ xn(ws("BIP0340/aux", i))),
      l = ws("BIP0340/nonce", c, o, s),
      { bytes: u, scalar: d } = nr(l),
      f = qc(u, o, s),
      p = new Uint8Array(64);
    if ((p.set(u, 0), p.set(n.toBytes(n.create(d + f * r)), 32), !Pc(p, s, o)))
      throw new Error("sign: Invalid signature produced");
    return p;
  }
  function Pc(t, e, a) {
    let { Fp: n, Fn: s, BASE: o } = oa,
      r = ce(t, 64, "signature"),
      i = ce(e, void 0, "message"),
      c = ce(a, 32, "publicKey");
    try {
      let l = jc(xn(c)),
        u = xn(r.subarray(0, 32));
      if (!n.isValidNot0(u)) return !1;
      let d = xn(r.subarray(32, 64));
      if (!s.isValidNot0(d)) return !1;
      let f = qc(s.toBytes(u), sr(l), i),
        p = o.multiplyUnsafe(d).add(l.multiplyUnsafe(s.neg(f))),
        { x: v, y: k } = p.toAffine();
      return !(p.is0() || !or(k) || v !== u);
    } catch {
      return !1;
    }
  }
  var Ra = (() => {
    let a = (n = aa(48)) => ms(n, ks.n);
    return {
      keygen: gs(a, Oc),
      getPublicKey: Oc,
      sign: Jf,
      verify: Pc,
      Point: oa,
      utils: {
        randomSecretKey: a,
        taggedHash: ws,
        lift_x: jc,
        pointToBytes: sr,
      },
      lengths: {
        secretKey: 32,
        publicKey: 32,
        publicKeyHasPrefix: !1,
        signature: 64,
        seed: 48,
      },
    };
  })();
  function Xf(t) {
    return (
      t instanceof Uint8Array ||
      (ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array")
    );
  }
  function Lt(t, e = "") {
    if (!Number.isSafeInteger(t) || t < 0) {
      let a = e && `"${e}" `;
      throw new Error(`${a}expected integer >= 0, got ${t}`);
    }
  }
  function Ze(t, e, a = "") {
    let n = Xf(t),
      s = t?.length,
      o = e !== void 0;
    if (!n || (o && s !== e)) {
      let r = a && `"${a}" `,
        i = o ? ` of length ${e}` : "",
        c = n ? `length=${s}` : `type=${typeof t}`;
      throw new Error(r + "expected Uint8Array" + i + ", got " + c);
    }
    return t;
  }
  function ra(t) {
    if (typeof t != "function" || typeof t.create != "function")
      throw new Error("Hash must wrapped by utils.createHasher");
    Lt(t.outputLen), Lt(t.blockLen);
  }
  function Ia(t, e = !0) {
    if (t.destroyed) throw new Error("Hash instance has been destroyed");
    if (e && t.finished)
      throw new Error("Hash#digest() has already been called");
  }
  function zc(t, e) {
    Ze(t, void 0, "digestInto() output");
    let a = e.outputLen;
    if (t.length < a)
      throw new Error('"digestInto() output" expected to be of length >=' + a);
  }
  function _e(...t) {
    for (let e = 0; e < t.length; e++) t[e].fill(0);
  }
  function Ha(t) {
    return new DataView(t.buffer, t.byteOffset, t.byteLength);
  }
  function Je(t, e) {
    return (t << (32 - e)) | (t >>> e);
  }
  var Gc =
      typeof Uint8Array.from([]).toHex == "function" &&
      typeof Uint8Array.fromHex == "function",
    Qf = Array.from({ length: 256 }, (t, e) => e.toString(16).padStart(2, "0"));
  function Oe(t) {
    if ((Ze(t), Gc)) return t.toHex();
    let e = "";
    for (let a = 0; a < t.length; a++) e += Qf[t[a]];
    return e;
  }
  var pt = { _0: 48, _9: 57, A: 65, F: 70, a: 97, f: 102 };
  function Vc(t) {
    if (t >= pt._0 && t <= pt._9) return t - pt._0;
    if (t >= pt.A && t <= pt.F) return t - (pt.A - 10);
    if (t >= pt.a && t <= pt.f) return t - (pt.a - 10);
  }
  function ht(t) {
    if (typeof t != "string")
      throw new Error("hex string expected, got " + typeof t);
    if (Gc) return Uint8Array.fromHex(t);
    let e = t.length,
      a = e / 2;
    if (e % 2)
      throw new Error("hex string expected, got unpadded hex of length " + e);
    let n = new Uint8Array(a);
    for (let s = 0, o = 0; s < a; s++, o += 2) {
      let r = Vc(t.charCodeAt(o)),
        i = Vc(t.charCodeAt(o + 1));
      if (r === void 0 || i === void 0) {
        let c = t[o] + t[o + 1];
        throw new Error(
          'hex string expected, got non-hex character "' +
            c +
            '" at index ' +
            o,
        );
      }
      n[s] = r * 16 + i;
    }
    return n;
  }
  function ep(t) {
    if (typeof t != "string") throw new Error("string expected");
    return new Uint8Array(new TextEncoder().encode(t));
  }
  function rr(t, e = "") {
    return typeof t == "string" ? ep(t) : Ze(t, void 0, e);
  }
  function Ua(...t) {
    let e = 0;
    for (let n = 0; n < t.length; n++) {
      let s = t[n];
      Ze(s), (e += s.length);
    }
    let a = new Uint8Array(e);
    for (let n = 0, s = 0; n < t.length; n++) {
      let o = t[n];
      a.set(o, s), (s += o.length);
    }
    return a;
  }
  function Kc(t, e) {
    if (e !== void 0 && {}.toString.call(e) !== "[object Object]")
      throw new Error("options must be object or undefined");
    return Object.assign(t, e);
  }
  function ir(t, e = {}) {
    let a = (s, o) => t(o).update(s).digest(),
      n = t(void 0);
    return (
      (a.outputLen = n.outputLen),
      (a.blockLen = n.blockLen),
      (a.create = (s) => t(s)),
      Object.assign(a, e),
      Object.freeze(a)
    );
  }
  function Fc(t = 32) {
    let e = typeof globalThis == "object" ? globalThis.crypto : null;
    if (typeof e?.getRandomValues != "function")
      throw new Error("crypto.getRandomValues must be defined");
    return e.getRandomValues(new Uint8Array(t));
  }
  var cr = (t) => ({
    oid: Uint8Array.from([6, 9, 96, 134, 72, 1, 101, 3, 4, 2, t]),
  });
  function Wc(t, e, a) {
    return (t & e) ^ (~t & a);
  }
  function Yc(t, e, a) {
    return (t & e) ^ (t & a) ^ (e & a);
  }
  var yn = class {
      blockLen;
      outputLen;
      padOffset;
      isLE;
      buffer;
      view;
      finished = !1;
      length = 0;
      pos = 0;
      destroyed = !1;
      constructor(e, a, n, s) {
        (this.blockLen = e),
          (this.outputLen = a),
          (this.padOffset = n),
          (this.isLE = s),
          (this.buffer = new Uint8Array(e)),
          (this.view = Ha(this.buffer));
      }
      update(e) {
        Ia(this), Ze(e);
        let { view: a, buffer: n, blockLen: s } = this,
          o = e.length;
        for (let r = 0; r < o; ) {
          let i = Math.min(s - this.pos, o - r);
          if (i === s) {
            let c = Ha(e);
            for (; s <= o - r; r += s) this.process(c, r);
            continue;
          }
          n.set(e.subarray(r, r + i), this.pos),
            (this.pos += i),
            (r += i),
            this.pos === s && (this.process(a, 0), (this.pos = 0));
        }
        return (this.length += e.length), this.roundClean(), this;
      }
      digestInto(e) {
        Ia(this), zc(e, this), (this.finished = !0);
        let { buffer: a, view: n, blockLen: s, isLE: o } = this,
          { pos: r } = this;
        (a[r++] = 128),
          _e(this.buffer.subarray(r)),
          this.padOffset > s - r && (this.process(n, 0), (r = 0));
        for (let d = r; d < s; d++) a[d] = 0;
        n.setBigUint64(s - 8, BigInt(this.length * 8), o), this.process(n, 0);
        let i = Ha(e),
          c = this.outputLen;
        if (c % 4) throw new Error("_sha2: outputLen must be aligned to 32bit");
        let l = c / 4,
          u = this.get();
        if (l > u.length) throw new Error("_sha2: outputLen bigger than state");
        for (let d = 0; d < l; d++) i.setUint32(4 * d, u[d], o);
      }
      digest() {
        let { buffer: e, outputLen: a } = this;
        this.digestInto(e);
        let n = e.slice(0, a);
        return this.destroy(), n;
      }
      _cloneInto(e) {
        (e ||= new this.constructor()), e.set(...this.get());
        let {
          blockLen: a,
          buffer: n,
          length: s,
          finished: o,
          destroyed: r,
          pos: i,
        } = this;
        return (
          (e.destroyed = r),
          (e.finished = o),
          (e.length = s),
          (e.pos = i),
          s % a && e.buffer.set(n),
          e
        );
      }
      clone() {
        return this._cloneInto();
      }
    },
    mt = Uint32Array.from([
      1779033703, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924,
      528734635, 1541459225,
    ]);
  var ke = Uint32Array.from([
    1779033703, 4089235720, 3144134277, 2227873595, 1013904242, 4271175723,
    2773480762, 1595750129, 1359893119, 2917565137, 2600822924, 725511199,
    528734635, 4215389547, 1541459225, 327033209,
  ]);
  var Es = BigInt(4294967295),
    Zc = BigInt(32);
  function tp(t, e = !1) {
    return e
      ? { h: Number(t & Es), l: Number((t >> Zc) & Es) }
      : { h: Number((t >> Zc) & Es) | 0, l: Number(t & Es) | 0 };
  }
  function Jc(t, e = !1) {
    let a = t.length,
      n = new Uint32Array(a),
      s = new Uint32Array(a);
    for (let o = 0; o < a; o++) {
      let { h: r, l: i } = tp(t[o], e);
      [n[o], s[o]] = [r, i];
    }
    return [n, s];
  }
  var lr = (t, e, a) => t >>> a,
    dr = (t, e, a) => (t << (32 - a)) | (e >>> a),
    ia = (t, e, a) => (t >>> a) | (e << (32 - a)),
    ca = (t, e, a) => (t << (32 - a)) | (e >>> a),
    vn = (t, e, a) => (t << (64 - a)) | (e >>> (a - 32)),
    wn = (t, e, a) => (t >>> (a - 32)) | (e << (64 - a));
  function st(t, e, a, n) {
    let s = (e >>> 0) + (n >>> 0);
    return { h: (t + a + ((s / 2 ** 32) | 0)) | 0, l: s | 0 };
  }
  var Xc = (t, e, a) => (t >>> 0) + (e >>> 0) + (a >>> 0),
    Qc = (t, e, a, n) => (e + a + n + ((t / 2 ** 32) | 0)) | 0,
    el = (t, e, a, n) => (t >>> 0) + (e >>> 0) + (a >>> 0) + (n >>> 0),
    tl = (t, e, a, n, s) => (e + a + n + s + ((t / 2 ** 32) | 0)) | 0,
    al = (t, e, a, n, s) =>
      (t >>> 0) + (e >>> 0) + (a >>> 0) + (n >>> 0) + (s >>> 0),
    nl = (t, e, a, n, s, o) => (e + a + n + s + o + ((t / 2 ** 32) | 0)) | 0;
  var np = Uint32Array.from([
      1116352408, 1899447441, 3049323471, 3921009573, 961987163, 1508970993,
      2453635748, 2870763221, 3624381080, 310598401, 607225278, 1426881987,
      1925078388, 2162078206, 2614888103, 3248222580, 3835390401, 4022224774,
      264347078, 604807628, 770255983, 1249150122, 1555081692, 1996064986,
      2554220882, 2821834349, 2952996808, 3210313671, 3336571891, 3584528711,
      113926993, 338241895, 666307205, 773529912, 1294757372, 1396182291,
      1695183700, 1986661051, 2177026350, 2456956037, 2730485921, 2820302411,
      3259730800, 3345764771, 3516065817, 3600352804, 4094571909, 275423344,
      430227734, 506948616, 659060556, 883997877, 958139571, 1322822218,
      1537002063, 1747873779, 1955562222, 2024104815, 2227730452, 2361852424,
      2428436474, 2756734187, 3204031479, 3329325298,
    ]),
    Tt = new Uint32Array(64),
    ur = class extends yn {
      constructor(e) {
        super(64, e, 8, !1);
      }
      get() {
        let { A: e, B: a, C: n, D: s, E: o, F: r, G: i, H: c } = this;
        return [e, a, n, s, o, r, i, c];
      }
      set(e, a, n, s, o, r, i, c) {
        (this.A = e | 0),
          (this.B = a | 0),
          (this.C = n | 0),
          (this.D = s | 0),
          (this.E = o | 0),
          (this.F = r | 0),
          (this.G = i | 0),
          (this.H = c | 0);
      }
      process(e, a) {
        for (let d = 0; d < 16; d++, a += 4) Tt[d] = e.getUint32(a, !1);
        for (let d = 16; d < 64; d++) {
          let f = Tt[d - 15],
            p = Tt[d - 2],
            v = Je(f, 7) ^ Je(f, 18) ^ (f >>> 3),
            k = Je(p, 17) ^ Je(p, 19) ^ (p >>> 10);
          Tt[d] = (k + Tt[d - 7] + v + Tt[d - 16]) | 0;
        }
        let { A: n, B: s, C: o, D: r, E: i, F: c, G: l, H: u } = this;
        for (let d = 0; d < 64; d++) {
          let f = Je(i, 6) ^ Je(i, 11) ^ Je(i, 25),
            p = (u + f + Wc(i, c, l) + np[d] + Tt[d]) | 0,
            k = ((Je(n, 2) ^ Je(n, 13) ^ Je(n, 22)) + Yc(n, s, o)) | 0;
          (u = l),
            (l = c),
            (c = i),
            (i = (r + p) | 0),
            (r = o),
            (o = s),
            (s = n),
            (n = (p + k) | 0);
        }
        (n = (n + this.A) | 0),
          (s = (s + this.B) | 0),
          (o = (o + this.C) | 0),
          (r = (r + this.D) | 0),
          (i = (i + this.E) | 0),
          (c = (c + this.F) | 0),
          (l = (l + this.G) | 0),
          (u = (u + this.H) | 0),
          this.set(n, s, o, r, i, c, l, u);
      }
      roundClean() {
        _e(Tt);
      }
      destroy() {
        this.set(0, 0, 0, 0, 0, 0, 0, 0), _e(this.buffer);
      }
    },
    fr = class extends ur {
      A = mt[0] | 0;
      B = mt[1] | 0;
      C = mt[2] | 0;
      D = mt[3] | 0;
      E = mt[4] | 0;
      F = mt[5] | 0;
      G = mt[6] | 0;
      H = mt[7] | 0;
      constructor() {
        super(32);
      }
    };
  var sl = Jc(
      [
        "0x428a2f98d728ae22",
        "0x7137449123ef65cd",
        "0xb5c0fbcfec4d3b2f",
        "0xe9b5dba58189dbbc",
        "0x3956c25bf348b538",
        "0x59f111f1b605d019",
        "0x923f82a4af194f9b",
        "0xab1c5ed5da6d8118",
        "0xd807aa98a3030242",
        "0x12835b0145706fbe",
        "0x243185be4ee4b28c",
        "0x550c7dc3d5ffb4e2",
        "0x72be5d74f27b896f",
        "0x80deb1fe3b1696b1",
        "0x9bdc06a725c71235",
        "0xc19bf174cf692694",
        "0xe49b69c19ef14ad2",
        "0xefbe4786384f25e3",
        "0x0fc19dc68b8cd5b5",
        "0x240ca1cc77ac9c65",
        "0x2de92c6f592b0275",
        "0x4a7484aa6ea6e483",
        "0x5cb0a9dcbd41fbd4",
        "0x76f988da831153b5",
        "0x983e5152ee66dfab",
        "0xa831c66d2db43210",
        "0xb00327c898fb213f",
        "0xbf597fc7beef0ee4",
        "0xc6e00bf33da88fc2",
        "0xd5a79147930aa725",
        "0x06ca6351e003826f",
        "0x142929670a0e6e70",
        "0x27b70a8546d22ffc",
        "0x2e1b21385c26c926",
        "0x4d2c6dfc5ac42aed",
        "0x53380d139d95b3df",
        "0x650a73548baf63de",
        "0x766a0abb3c77b2a8",
        "0x81c2c92e47edaee6",
        "0x92722c851482353b",
        "0xa2bfe8a14cf10364",
        "0xa81a664bbc423001",
        "0xc24b8b70d0f89791",
        "0xc76c51a30654be30",
        "0xd192e819d6ef5218",
        "0xd69906245565a910",
        "0xf40e35855771202a",
        "0x106aa07032bbd1b8",
        "0x19a4c116b8d2d0c8",
        "0x1e376c085141ab53",
        "0x2748774cdf8eeb99",
        "0x34b0bcb5e19b48a8",
        "0x391c0cb3c5c95a63",
        "0x4ed8aa4ae3418acb",
        "0x5b9cca4f7763e373",
        "0x682e6ff3d6b2b8a3",
        "0x748f82ee5defb2fc",
        "0x78a5636f43172f60",
        "0x84c87814a1f0ab72",
        "0x8cc702081a6439ec",
        "0x90befffa23631e28",
        "0xa4506cebde82bde9",
        "0xbef9a3f7b2c67915",
        "0xc67178f2e372532b",
        "0xca273eceea26619c",
        "0xd186b8c721c0c207",
        "0xeada7dd6cde0eb1e",
        "0xf57d4f7fee6ed178",
        "0x06f067aa72176fba",
        "0x0a637dc5a2c898a6",
        "0x113f9804bef90dae",
        "0x1b710b35131c471b",
        "0x28db77f523047d84",
        "0x32caab7b40c72493",
        "0x3c9ebe0a15c9bebc",
        "0x431d67c49c100d4c",
        "0x4cc5d4becb3e42b6",
        "0x597f299cfc657e2a",
        "0x5fcb6fab3ad6faec",
        "0x6c44198c4a475817",
      ].map((t) => BigInt(t)),
    ),
    sp = sl[0],
    op = sl[1],
    Bt = new Uint32Array(80),
    Mt = new Uint32Array(80),
    pr = class extends yn {
      constructor(e) {
        super(128, e, 16, !1);
      }
      get() {
        let {
          Ah: e,
          Al: a,
          Bh: n,
          Bl: s,
          Ch: o,
          Cl: r,
          Dh: i,
          Dl: c,
          Eh: l,
          El: u,
          Fh: d,
          Fl: f,
          Gh: p,
          Gl: v,
          Hh: k,
          Hl: L,
        } = this;
        return [e, a, n, s, o, r, i, c, l, u, d, f, p, v, k, L];
      }
      set(e, a, n, s, o, r, i, c, l, u, d, f, p, v, k, L) {
        (this.Ah = e | 0),
          (this.Al = a | 0),
          (this.Bh = n | 0),
          (this.Bl = s | 0),
          (this.Ch = o | 0),
          (this.Cl = r | 0),
          (this.Dh = i | 0),
          (this.Dl = c | 0),
          (this.Eh = l | 0),
          (this.El = u | 0),
          (this.Fh = d | 0),
          (this.Fl = f | 0),
          (this.Gh = p | 0),
          (this.Gl = v | 0),
          (this.Hh = k | 0),
          (this.Hl = L | 0);
      }
      process(e, a) {
        for (let O = 0; O < 16; O++, a += 4)
          (Bt[O] = e.getUint32(a)), (Mt[O] = e.getUint32((a += 4)));
        for (let O = 16; O < 80; O++) {
          let q = Bt[O - 15] | 0,
            K = Mt[O - 15] | 0,
            T = ia(q, K, 1) ^ ia(q, K, 8) ^ lr(q, K, 7),
            g = ca(q, K, 1) ^ ca(q, K, 8) ^ dr(q, K, 7),
            w = Bt[O - 2] | 0,
            m = Mt[O - 2] | 0,
            $ = ia(w, m, 19) ^ vn(w, m, 61) ^ lr(w, m, 6),
            A = ca(w, m, 19) ^ wn(w, m, 61) ^ dr(w, m, 6),
            R = el(g, A, Mt[O - 7], Mt[O - 16]),
            E = tl(R, T, $, Bt[O - 7], Bt[O - 16]);
          (Bt[O] = E | 0), (Mt[O] = R | 0);
        }
        let {
          Ah: n,
          Al: s,
          Bh: o,
          Bl: r,
          Ch: i,
          Cl: c,
          Dh: l,
          Dl: u,
          Eh: d,
          El: f,
          Fh: p,
          Fl: v,
          Gh: k,
          Gl: L,
          Hh: N,
          Hl: V,
        } = this;
        for (let O = 0; O < 80; O++) {
          let q = ia(d, f, 14) ^ ia(d, f, 18) ^ vn(d, f, 41),
            K = ca(d, f, 14) ^ ca(d, f, 18) ^ wn(d, f, 41),
            T = (d & p) ^ (~d & k),
            g = (f & v) ^ (~f & L),
            w = al(V, K, g, op[O], Mt[O]),
            m = nl(w, N, q, T, sp[O], Bt[O]),
            $ = w | 0,
            A = ia(n, s, 28) ^ vn(n, s, 34) ^ vn(n, s, 39),
            R = ca(n, s, 28) ^ wn(n, s, 34) ^ wn(n, s, 39),
            E = (n & o) ^ (n & i) ^ (o & i),
            B = (s & r) ^ (s & c) ^ (r & c);
          (N = k | 0),
            (V = L | 0),
            (k = p | 0),
            (L = v | 0),
            (p = d | 0),
            (v = f | 0),
            ({ h: d, l: f } = st(l | 0, u | 0, m | 0, $ | 0)),
            (l = i | 0),
            (u = c | 0),
            (i = o | 0),
            (c = r | 0),
            (o = n | 0),
            (r = s | 0);
          let S = Xc($, R, B);
          (n = Qc(S, m, A, E)), (s = S | 0);
        }
        ({ h: n, l: s } = st(this.Ah | 0, this.Al | 0, n | 0, s | 0)),
          ({ h: o, l: r } = st(this.Bh | 0, this.Bl | 0, o | 0, r | 0)),
          ({ h: i, l: c } = st(this.Ch | 0, this.Cl | 0, i | 0, c | 0)),
          ({ h: l, l: u } = st(this.Dh | 0, this.Dl | 0, l | 0, u | 0)),
          ({ h: d, l: f } = st(this.Eh | 0, this.El | 0, d | 0, f | 0)),
          ({ h: p, l: v } = st(this.Fh | 0, this.Fl | 0, p | 0, v | 0)),
          ({ h: k, l: L } = st(this.Gh | 0, this.Gl | 0, k | 0, L | 0)),
          ({ h: N, l: V } = st(this.Hh | 0, this.Hl | 0, N | 0, V | 0)),
          this.set(n, s, o, r, i, c, l, u, d, f, p, v, k, L, N, V);
      }
      roundClean() {
        _e(Bt, Mt);
      }
      destroy() {
        _e(this.buffer),
          this.set(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
      }
    },
    hr = class extends pr {
      Ah = ke[0] | 0;
      Al = ke[1] | 0;
      Bh = ke[2] | 0;
      Bl = ke[3] | 0;
      Ch = ke[4] | 0;
      Cl = ke[5] | 0;
      Dh = ke[6] | 0;
      Dl = ke[7] | 0;
      Eh = ke[8] | 0;
      El = ke[9] | 0;
      Fh = ke[10] | 0;
      Fl = ke[11] | 0;
      Gh = ke[12] | 0;
      Gl = ke[13] | 0;
      Hh = ke[14] | 0;
      Hl = ke[15] | 0;
      constructor() {
        super(64);
      }
    };
  var la = ir(() => new fr(), cr(1));
  var ol = ir(() => new hr(), cr(3));
  var Pm = new TextDecoder("utf-8"),
    rp = new TextEncoder();
  function ip(t) {
    if (t.length !== 64) return !1;
    for (let e = 0; e < 64; e++) {
      let a = t.charCodeAt(e);
      if (isNaN(a) || a < 48 || a > 102 || (a > 57 && a < 97)) return !1;
    }
    return !0;
  }
  var Da = Symbol("verified"),
    cp = (t) => t instanceof Object;
  function lp(t) {
    if (
      !cp(t) ||
      typeof t.kind != "number" ||
      typeof t.content != "string" ||
      typeof t.created_at != "number" ||
      typeof t.pubkey != "string" ||
      !ip(t.pubkey) ||
      !Array.isArray(t.tags)
    )
      return !1;
    for (let e = 0; e < t.tags.length; e++) {
      let a = t.tags[e];
      if (!Array.isArray(a)) return !1;
      for (let n = 0; n < a.length; n++) if (typeof a[n] != "string") return !1;
    }
    return !0;
  }
  var dp = class {
    generateSecretKey() {
      return Ra.utils.randomSecretKey();
    }
    getPublicKey(t) {
      return Oe(Ra.getPublicKey(t));
    }
    finalizeEvent(t, e) {
      let a = t;
      return (
        (a.pubkey = Oe(Ra.getPublicKey(e))),
        (a.id = Na(a)),
        (a.sig = Oe(Ra.sign(ht(Na(a)), e))),
        (a[Da] = !0),
        a
      );
    }
    verifyEvent(t) {
      if (typeof t[Da] == "boolean") return t[Da];
      try {
        let e = Na(t);
        if (e !== t.id) return (t[Da] = !1), !1;
        let a = Ra.verify(ht(t.sig), ht(e), ht(t.pubkey));
        return (t[Da] = a), a;
      } catch {
        return (t[Da] = !1), !1;
      }
    }
  };
  function up(t) {
    if (!lp(t))
      throw new Error("can't serialize event with wrong or missing properties");
    return JSON.stringify([
      0,
      t.pubkey,
      t.created_at,
      t.kind,
      t.tags,
      t.content,
    ]);
  }
  function Na(t) {
    let e = la(rp.encode(up(t)));
    return Oe(e);
  }
  var $s = new dp(),
    kn = $s.generateSecretKey,
    Ke = $s.getPublicKey,
    As = $s.finalizeEvent,
    mr = $s.verifyEvent;
  function _a(t) {
    return (
      t instanceof Uint8Array ||
      (ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array")
    );
  }
  function fp(t) {
    if (!_a(t)) throw new Error("Uint8Array expected");
  }
  function dl(t, e) {
    return Array.isArray(e)
      ? e.length === 0
        ? !0
        : t
          ? e.every((a) => typeof a == "string")
          : e.every((a) => Number.isSafeInteger(a))
      : !1;
  }
  function ul(t) {
    if (typeof t != "function") throw new Error("function expected");
    return !0;
  }
  function da(t, e) {
    if (typeof e != "string") throw new Error(`${t}: string expected`);
    return !0;
  }
  function Oa(t) {
    if (!Number.isSafeInteger(t)) throw new Error(`invalid integer: ${t}`);
  }
  function Cs(t) {
    if (!Array.isArray(t)) throw new Error("array expected");
  }
  function Ls(t, e) {
    if (!dl(!0, e)) throw new Error(`${t}: array of strings expected`);
  }
  function xr(t, e) {
    if (!dl(!1, e)) throw new Error(`${t}: array of numbers expected`);
  }
  function Bs(...t) {
    let e = (o) => o,
      a = (o, r) => (i) => o(r(i)),
      n = t.map((o) => o.encode).reduceRight(a, e),
      s = t.map((o) => o.decode).reduce(a, e);
    return { encode: n, decode: s };
  }
  function yr(t) {
    let e = typeof t == "string" ? t.split("") : t,
      a = e.length;
    Ls("alphabet", e);
    let n = new Map(e.map((s, o) => [s, o]));
    return {
      encode: (s) => (
        Cs(s),
        s.map((o) => {
          if (!Number.isSafeInteger(o) || o < 0 || o >= a)
            throw new Error(
              `alphabet.encode: digit index outside alphabet "${o}". Allowed: ${t}`,
            );
          return e[o];
        })
      ),
      decode: (s) => (
        Cs(s),
        s.map((o) => {
          da("alphabet.decode", o);
          let r = n.get(o);
          if (r === void 0)
            throw new Error(`Unknown letter: "${o}". Allowed: ${t}`);
          return r;
        })
      ),
    };
  }
  function vr(t = "") {
    return (
      da("join", t),
      {
        encode: (e) => (Ls("join.decode", e), e.join(t)),
        decode: (e) => (da("join.decode", e), e.split(t)),
      }
    );
  }
  function pp(t, e = "=") {
    return (
      Oa(t),
      da("padding", e),
      {
        encode(a) {
          for (Ls("padding.encode", a); (a.length * t) % 8; ) a.push(e);
          return a;
        },
        decode(a) {
          Ls("padding.decode", a);
          let n = a.length;
          if ((n * t) % 8)
            throw new Error(
              "padding: invalid, string should have whole number of bytes",
            );
          for (; n > 0 && a[n - 1] === e; n--)
            if (((n - 1) * t) % 8 === 0)
              throw new Error("padding: invalid, string has too much padding");
          return a.slice(0, n);
        },
      }
    );
  }
  function rl(t, e, a) {
    if (e < 2)
      throw new Error(
        `convertRadix: invalid from=${e}, base cannot be less than 2`,
      );
    if (a < 2)
      throw new Error(
        `convertRadix: invalid to=${a}, base cannot be less than 2`,
      );
    if ((Cs(t), !t.length)) return [];
    let n = 0,
      s = [],
      o = Array.from(t, (i) => {
        if ((Oa(i), i < 0 || i >= e)) throw new Error(`invalid integer: ${i}`);
        return i;
      }),
      r = o.length;
    for (;;) {
      let i = 0,
        c = !0;
      for (let l = n; l < r; l++) {
        let u = o[l],
          d = e * i,
          f = d + u;
        if (!Number.isSafeInteger(f) || d / e !== i || f - u !== d)
          throw new Error("convertRadix: carry overflow");
        let p = f / a;
        i = f % a;
        let v = Math.floor(p);
        if (((o[l] = v), !Number.isSafeInteger(v) || v * a + i !== f))
          throw new Error("convertRadix: carry overflow");
        if (c) v ? (c = !1) : (n = l);
        else continue;
      }
      if ((s.push(i), c)) break;
    }
    for (let i = 0; i < t.length - 1 && t[i] === 0; i++) s.push(0);
    return s.reverse();
  }
  var fl = (t, e) => (e === 0 ? t : fl(e, t % e)),
    Ts = (t, e) => t + (e - fl(t, e)),
    Ss = (() => {
      let t = [];
      for (let e = 0; e < 40; e++) t.push(2 ** e);
      return t;
    })();
  function br(t, e, a, n) {
    if ((Cs(t), e <= 0 || e > 32))
      throw new Error(`convertRadix2: wrong from=${e}`);
    if (a <= 0 || a > 32) throw new Error(`convertRadix2: wrong to=${a}`);
    if (Ts(e, a) > 32)
      throw new Error(
        `convertRadix2: carry overflow from=${e} to=${a} carryBits=${Ts(e, a)}`,
      );
    let s = 0,
      o = 0,
      r = Ss[e],
      i = Ss[a] - 1,
      c = [];
    for (let l of t) {
      if ((Oa(l), l >= r))
        throw new Error(`convertRadix2: invalid data word=${l} from=${e}`);
      if (((s = (s << e) | l), o + e > 32))
        throw new Error(`convertRadix2: carry overflow pos=${o} from=${e}`);
      for (o += e; o >= a; o -= a) c.push(((s >> (o - a)) & i) >>> 0);
      let u = Ss[o];
      if (u === void 0) throw new Error("invalid carry");
      s &= u - 1;
    }
    if (((s = (s << (a - o)) & i), !n && o >= e))
      throw new Error("Excess padding");
    if (!n && s > 0) throw new Error(`Non-zero padding: ${s}`);
    return n && o > 0 && c.push(s >>> 0), c;
  }
  function hp(t) {
    Oa(t);
    let e = 2 ** 8;
    return {
      encode: (a) => {
        if (!_a(a)) throw new Error("radix.encode input should be Uint8Array");
        return rl(Array.from(a), e, t);
      },
      decode: (a) => (xr("radix.decode", a), Uint8Array.from(rl(a, t, e))),
    };
  }
  function pl(t, e = !1) {
    if ((Oa(t), t <= 0 || t > 32))
      throw new Error("radix2: bits should be in (0..32]");
    if (Ts(8, t) > 32 || Ts(t, 8) > 32)
      throw new Error("radix2: carry overflow");
    return {
      encode: (a) => {
        if (!_a(a)) throw new Error("radix2.encode input should be Uint8Array");
        return br(Array.from(a), 8, t, !e);
      },
      decode: (a) => (xr("radix2.decode", a), Uint8Array.from(br(a, t, 8, e))),
    };
  }
  function il(t) {
    return (
      ul(t),
      function (...e) {
        try {
          return t.apply(null, e);
        } catch {}
      }
    );
  }
  function mp(t, e) {
    return (
      Oa(t),
      ul(e),
      {
        encode(a) {
          if (!_a(a))
            throw new Error("checksum.encode: input should be Uint8Array");
          let n = e(a).slice(0, t),
            s = new Uint8Array(a.length + t);
          return s.set(a), s.set(n, a.length), s;
        },
        decode(a) {
          if (!_a(a))
            throw new Error("checksum.decode: input should be Uint8Array");
          let n = a.slice(0, -t),
            s = a.slice(-t),
            o = e(n).slice(0, t);
          for (let r = 0; r < t; r++)
            if (o[r] !== s[r]) throw new Error("Invalid checksum");
          return n;
        },
      }
    );
  }
  var bp =
      typeof Uint8Array.from([]).toBase64 == "function" &&
      typeof Uint8Array.fromBase64 == "function",
    gp = (t, e) => {
      da("base64", t);
      let a = e ? /^[A-Za-z0-9=_-]+$/ : /^[A-Za-z0-9=+/]+$/,
        n = e ? "base64url" : "base64";
      if (t.length > 0 && !a.test(t)) throw new Error("invalid base64");
      return Uint8Array.fromBase64(t, {
        alphabet: n,
        lastChunkHandling: "strict",
      });
    },
    ja = bp
      ? {
          encode(t) {
            return fp(t), t.toBase64();
          },
          decode(t) {
            return gp(t, !1);
          },
        }
      : Bs(
          pl(6),
          yr(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
          ),
          pp(6),
          vr(""),
        );
  var xp = (t) => Bs(hp(58), yr(t), vr("")),
    yp = xp("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz");
  var hl = (t) =>
    Bs(
      mp(4, (e) => t(t(e))),
      yp,
    );
  var gr = Bs(yr("qpzry9x8gf2tvdw0s3jn54khce6mua7l"), vr("")),
    cl = [996825010, 642813549, 513874426, 1027748829, 705979059];
  function En(t) {
    let e = t >> 25,
      a = (t & 33554431) << 5;
    for (let n = 0; n < cl.length; n++) ((e >> n) & 1) === 1 && (a ^= cl[n]);
    return a;
  }
  function ll(t, e, a = 1) {
    let n = t.length,
      s = 1;
    for (let o = 0; o < n; o++) {
      let r = t.charCodeAt(o);
      if (r < 33 || r > 126) throw new Error(`Invalid prefix (${t})`);
      s = En(s) ^ (r >> 5);
    }
    s = En(s);
    for (let o = 0; o < n; o++) s = En(s) ^ (t.charCodeAt(o) & 31);
    for (let o of e) s = En(s) ^ o;
    for (let o = 0; o < 6; o++) s = En(s);
    return (s ^= a), gr.encode(br([s % Ss[30]], 30, 5, !1));
  }
  function vp(t) {
    let e = t === "bech32" ? 1 : 734539939,
      a = pl(5),
      n = a.decode,
      s = a.encode,
      o = il(n);
    function r(d, f, p = 90) {
      da("bech32.encode prefix", d),
        _a(f) && (f = Array.from(f)),
        xr("bech32.encode", f);
      let v = d.length;
      if (v === 0) throw new TypeError(`Invalid prefix length ${v}`);
      let k = v + 7 + f.length;
      if (p !== !1 && k > p)
        throw new TypeError(`Length ${k} exceeds limit ${p}`);
      let L = d.toLowerCase(),
        N = ll(L, f, e);
      return `${L}1${gr.encode(f)}${N}`;
    }
    function i(d, f = 90) {
      da("bech32.decode input", d);
      let p = d.length;
      if (p < 8 || (f !== !1 && p > f))
        throw new TypeError(
          `invalid string length: ${p} (${d}). Expected (8..${f})`,
        );
      let v = d.toLowerCase();
      if (d !== v && d !== d.toUpperCase())
        throw new Error("String must be lowercase or uppercase");
      let k = v.lastIndexOf("1");
      if (k === 0 || k === -1)
        throw new Error(
          'Letter "1" must be present between prefix and data only',
        );
      let L = v.slice(0, k),
        N = v.slice(k + 1);
      if (N.length < 6)
        throw new Error("Data must be at least 6 characters long");
      let V = gr.decode(N).slice(0, -6),
        O = ll(L, V, e);
      if (!N.endsWith(O))
        throw new Error(`Invalid checksum in ${d}: expected "${O}"`);
      return { prefix: L, words: V };
    }
    let c = il(i);
    function l(d) {
      let { prefix: f, words: p } = i(d, !1);
      return { prefix: f, words: p, bytes: n(p) };
    }
    function u(d, f) {
      return r(d, s(f));
    }
    return {
      encode: r,
      decode: i,
      encodeFromBytes: u,
      decodeToBytes: l,
      decodeUnsafe: c,
      fromWords: n,
      fromWordsUnsafe: o,
      toWords: s,
    };
  }
  var $n = vp("bech32");
  var Ms = new TextDecoder("utf-8"),
    Ym = new TextEncoder();
  var ml = 5e3;
  function kr(t) {
    let { prefix: e, words: a } = $n.decode(t, ml),
      n = new Uint8Array($n.fromWords(a));
    switch (e) {
      case "nprofile": {
        let s = wr(n);
        if (!s[0]?.[0]) throw new Error("missing TLV 0 for nprofile");
        if (s[0][0].length !== 32) throw new Error("TLV 0 should be 32 bytes");
        return {
          type: "nprofile",
          data: {
            pubkey: Oe(s[0][0]),
            relays: s[1] ? s[1].map((o) => Ms.decode(o)) : [],
          },
        };
      }
      case "nevent": {
        let s = wr(n);
        if (!s[0]?.[0]) throw new Error("missing TLV 0 for nevent");
        if (s[0][0].length !== 32) throw new Error("TLV 0 should be 32 bytes");
        if (s[2] && s[2][0].length !== 32)
          throw new Error("TLV 2 should be 32 bytes");
        if (s[3] && s[3][0].length !== 4)
          throw new Error("TLV 3 should be 4 bytes");
        return {
          type: "nevent",
          data: {
            id: Oe(s[0][0]),
            relays: s[1] ? s[1].map((o) => Ms.decode(o)) : [],
            author: s[2]?.[0] ? Oe(s[2][0]) : void 0,
            kind: s[3]?.[0] ? parseInt(Oe(s[3][0]), 16) : void 0,
          },
        };
      }
      case "naddr": {
        let s = wr(n);
        if (!s[0]?.[0]) throw new Error("missing TLV 0 for naddr");
        if (!s[2]?.[0]) throw new Error("missing TLV 2 for naddr");
        if (s[2][0].length !== 32) throw new Error("TLV 2 should be 32 bytes");
        if (!s[3]?.[0]) throw new Error("missing TLV 3 for naddr");
        if (s[3][0].length !== 4) throw new Error("TLV 3 should be 4 bytes");
        return {
          type: "naddr",
          data: {
            identifier: Ms.decode(s[0][0]),
            pubkey: Oe(s[2][0]),
            kind: parseInt(Oe(s[3][0]), 16),
            relays: s[1] ? s[1].map((o) => Ms.decode(o)) : [],
          },
        };
      }
      case "nsec":
        return { type: e, data: n };
      case "npub":
      case "note":
        return { type: e, data: Oe(n) };
      default:
        throw new Error(`unknown prefix ${e}`);
    }
  }
  function wr(t) {
    let e = {},
      a = t;
    for (; a.length > 0; ) {
      if (a.length < 2) throw new Error("not enough data to read TLV");
      let n = a[0],
        s = a[1],
        o = a.slice(2, 2 + s);
      if (((a = a.slice(2 + s)), o.length < s))
        throw new Error(`not enough data to read on TLV ${n}`);
      (e[n] = e[n] || []), e[n].push(o);
    }
    return e;
  }
  function bl(t) {
    return gl("nsec", t);
  }
  function An(t) {
    return gl("npub", ht(t));
  }
  function wp(t, e) {
    let a = $n.toWords(e);
    return $n.encode(t, a, ml);
  }
  function gl(t, e) {
    return wp(t, e);
  }
  var Rs = class {
      oHash;
      iHash;
      blockLen;
      outputLen;
      finished = !1;
      destroyed = !1;
      constructor(e, a) {
        if (
          (ra(e),
          Ze(a, void 0, "key"),
          (this.iHash = e.create()),
          typeof this.iHash.update != "function")
        )
          throw new Error(
            "Expected instance of class which extends utils.Hash",
          );
        (this.blockLen = this.iHash.blockLen),
          (this.outputLen = this.iHash.outputLen);
        let n = this.blockLen,
          s = new Uint8Array(n);
        s.set(a.length > n ? e.create().update(a).digest() : a);
        for (let o = 0; o < s.length; o++) s[o] ^= 54;
        this.iHash.update(s), (this.oHash = e.create());
        for (let o = 0; o < s.length; o++) s[o] ^= 106;
        this.oHash.update(s), _e(s);
      }
      update(e) {
        return Ia(this), this.iHash.update(e), this;
      }
      digestInto(e) {
        Ia(this),
          Ze(e, this.outputLen, "output"),
          (this.finished = !0),
          this.iHash.digestInto(e),
          this.oHash.update(e),
          this.oHash.digestInto(e),
          this.destroy();
      }
      digest() {
        let e = new Uint8Array(this.oHash.outputLen);
        return this.digestInto(e), e;
      }
      _cloneInto(e) {
        e ||= Object.create(Object.getPrototypeOf(this), {});
        let {
          oHash: a,
          iHash: n,
          finished: s,
          destroyed: o,
          blockLen: r,
          outputLen: i,
        } = this;
        return (
          (e = e),
          (e.finished = s),
          (e.destroyed = o),
          (e.blockLen = r),
          (e.outputLen = i),
          (e.oHash = a._cloneInto(e.oHash)),
          (e.iHash = n._cloneInto(e.iHash)),
          e
        );
      }
      clone() {
        return this._cloneInto();
      }
      destroy() {
        (this.destroyed = !0), this.oHash.destroy(), this.iHash.destroy();
      }
    },
    Rt = (t, e, a) => new Rs(t, e).update(a).digest();
  Rt.create = (t, e) => new Rs(t, e);
  function kp(t, e, a, n) {
    ra(t);
    let s = Kc({ dkLen: 32, asyncTick: 10 }, n),
      { c: o, dkLen: r, asyncTick: i } = s;
    if ((Lt(o, "c"), Lt(r, "dkLen"), Lt(i, "asyncTick"), o < 1))
      throw new Error("iterations (c) must be >= 1");
    let c = rr(e, "password"),
      l = rr(a, "salt"),
      u = new Uint8Array(r),
      d = Rt.create(t, c),
      f = d._cloneInto().update(l);
    return { c: o, dkLen: r, asyncTick: i, DK: u, PRF: d, PRFSalt: f };
  }
  function Ep(t, e, a, n, s) {
    return t.destroy(), e.destroy(), n && n.destroy(), _e(s), a;
  }
  function xl(t, e, a, n) {
    let { c: s, dkLen: o, DK: r, PRF: i, PRFSalt: c } = kp(t, e, a, n),
      l,
      u = new Uint8Array(4),
      d = Ha(u),
      f = new Uint8Array(i.outputLen);
    for (let p = 1, v = 0; v < o; p++, v += i.outputLen) {
      let k = r.subarray(v, v + i.outputLen);
      d.setInt32(0, p, !1),
        (l = c._cloneInto(l)).update(u).digestInto(f),
        k.set(f.subarray(0, k.length));
      for (let L = 1; L < s; L++) {
        i._cloneInto(l).update(f).digestInto(f);
        for (let N = 0; N < k.length; N++) k[N] ^= f[N];
      }
    }
    return Ep(i, c, r, l, f);
  }
  function yl(t) {
    if (typeof t != "string")
      throw new TypeError("invalid mnemonic type: " + typeof t);
    return t.normalize("NFKD");
  }
  function $p(t) {
    let e = yl(t),
      a = e.split(" ");
    if (![12, 15, 18, 21, 24].includes(a.length))
      throw new Error("Invalid mnemonic");
    return { nfkd: e, words: a };
  }
  var Ap = (t) => yl("mnemonic" + t);
  function vl(t, e = "") {
    return xl(ol, $p(t).nfkd, Ap(e), { c: 2048, dkLen: 64 });
  }
  function Sp(t) {
    return (
      t instanceof Uint8Array ||
      (ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array")
    );
  }
  function wl(t, e = "") {
    if (!Number.isSafeInteger(t) || t < 0) {
      let a = e && `"${e}" `;
      throw new Error(`${a}expected integer >= 0, got ${t}`);
    }
  }
  function je(t, e, a = "") {
    let n = Sp(t),
      s = t?.length,
      o = e !== void 0;
    if (!n || (o && s !== e)) {
      let r = a && `"${a}" `,
        i = o ? ` of length ${e}` : "",
        c = n ? `length=${s}` : `type=${typeof t}`;
      throw new Error(r + "expected Uint8Array" + i + ", got " + c);
    }
    return t;
  }
  function kl(t) {
    if (typeof t != "function" || typeof t.create != "function")
      throw new Error("Hash must wrapped by utils.createHasher");
    wl(t.outputLen), wl(t.blockLen);
  }
  function qa(t, e = !0) {
    if (t.destroyed) throw new Error("Hash instance has been destroyed");
    if (e && t.finished)
      throw new Error("Hash#digest() has already been called");
  }
  function El(t, e) {
    je(t, void 0, "digestInto() output");
    let a = e.outputLen;
    if (t.length < a)
      throw new Error('"digestInto() output" expected to be of length >=' + a);
  }
  function Fe(...t) {
    for (let e = 0; e < t.length; e++) t[e].fill(0);
  }
  function It(t) {
    return new DataView(t.buffer, t.byteOffset, t.byteLength);
  }
  function Xe(t, e) {
    return (t << (32 - e)) | (t >>> e);
  }
  function Sn(t, e) {
    return (t << e) | ((t >>> (32 - e)) >>> 0);
  }
  function Cn(...t) {
    let e = 0;
    for (let n = 0; n < t.length; n++) {
      let s = t[n];
      je(s), (e += s.length);
    }
    let a = new Uint8Array(e);
    for (let n = 0, s = 0; n < t.length; n++) {
      let o = t[n];
      a.set(o, s), (s += o.length);
    }
    return a;
  }
  function Ln(t, e = {}) {
    let a = (s, o) => t(o).update(s).digest(),
      n = t(void 0);
    return (
      (a.outputLen = n.outputLen),
      (a.blockLen = n.blockLen),
      (a.create = (s) => t(s)),
      Object.assign(a, e),
      Object.freeze(a)
    );
  }
  var Er = (t) => ({
    oid: Uint8Array.from([6, 9, 96, 134, 72, 1, 101, 3, 4, 2, t]),
  });
  var Is = class {
      oHash;
      iHash;
      blockLen;
      outputLen;
      finished = !1;
      destroyed = !1;
      constructor(e, a) {
        if (
          (kl(e),
          je(a, void 0, "key"),
          (this.iHash = e.create()),
          typeof this.iHash.update != "function")
        )
          throw new Error(
            "Expected instance of class which extends utils.Hash",
          );
        (this.blockLen = this.iHash.blockLen),
          (this.outputLen = this.iHash.outputLen);
        let n = this.blockLen,
          s = new Uint8Array(n);
        s.set(a.length > n ? e.create().update(a).digest() : a);
        for (let o = 0; o < s.length; o++) s[o] ^= 54;
        this.iHash.update(s), (this.oHash = e.create());
        for (let o = 0; o < s.length; o++) s[o] ^= 106;
        this.oHash.update(s), Fe(s);
      }
      update(e) {
        return qa(this), this.iHash.update(e), this;
      }
      digestInto(e) {
        qa(this),
          je(e, this.outputLen, "output"),
          (this.finished = !0),
          this.iHash.digestInto(e),
          this.oHash.update(e),
          this.oHash.digestInto(e),
          this.destroy();
      }
      digest() {
        let e = new Uint8Array(this.oHash.outputLen);
        return this.digestInto(e), e;
      }
      _cloneInto(e) {
        e ||= Object.create(Object.getPrototypeOf(this), {});
        let {
          oHash: a,
          iHash: n,
          finished: s,
          destroyed: o,
          blockLen: r,
          outputLen: i,
        } = this;
        return (
          (e = e),
          (e.finished = s),
          (e.destroyed = o),
          (e.blockLen = r),
          (e.outputLen = i),
          (e.oHash = a._cloneInto(e.oHash)),
          (e.iHash = n._cloneInto(e.iHash)),
          e
        );
      }
      clone() {
        return this._cloneInto();
      }
      destroy() {
        (this.destroyed = !0), this.oHash.destroy(), this.iHash.destroy();
      }
    },
    Hs = (t, e, a) => new Is(t, e).update(a).digest();
  Hs.create = (t, e) => new Is(t, e);
  function $r(t, e, a) {
    return (t & e) ^ (~t & a);
  }
  function Ar(t, e, a) {
    return (t & e) ^ (t & a) ^ (e & a);
  }
  var ua = class {
      blockLen;
      outputLen;
      padOffset;
      isLE;
      buffer;
      view;
      finished = !1;
      length = 0;
      pos = 0;
      destroyed = !1;
      constructor(e, a, n, s) {
        (this.blockLen = e),
          (this.outputLen = a),
          (this.padOffset = n),
          (this.isLE = s),
          (this.buffer = new Uint8Array(e)),
          (this.view = It(this.buffer));
      }
      update(e) {
        qa(this), je(e);
        let { view: a, buffer: n, blockLen: s } = this,
          o = e.length;
        for (let r = 0; r < o; ) {
          let i = Math.min(s - this.pos, o - r);
          if (i === s) {
            let c = It(e);
            for (; s <= o - r; r += s) this.process(c, r);
            continue;
          }
          n.set(e.subarray(r, r + i), this.pos),
            (this.pos += i),
            (r += i),
            this.pos === s && (this.process(a, 0), (this.pos = 0));
        }
        return (this.length += e.length), this.roundClean(), this;
      }
      digestInto(e) {
        qa(this), El(e, this), (this.finished = !0);
        let { buffer: a, view: n, blockLen: s, isLE: o } = this,
          { pos: r } = this;
        (a[r++] = 128),
          Fe(this.buffer.subarray(r)),
          this.padOffset > s - r && (this.process(n, 0), (r = 0));
        for (let d = r; d < s; d++) a[d] = 0;
        n.setBigUint64(s - 8, BigInt(this.length * 8), o), this.process(n, 0);
        let i = It(e),
          c = this.outputLen;
        if (c % 4) throw new Error("_sha2: outputLen must be aligned to 32bit");
        let l = c / 4,
          u = this.get();
        if (l > u.length) throw new Error("_sha2: outputLen bigger than state");
        for (let d = 0; d < l; d++) i.setUint32(4 * d, u[d], o);
      }
      digest() {
        let { buffer: e, outputLen: a } = this;
        this.digestInto(e);
        let n = e.slice(0, a);
        return this.destroy(), n;
      }
      _cloneInto(e) {
        (e ||= new this.constructor()), e.set(...this.get());
        let {
          blockLen: a,
          buffer: n,
          length: s,
          finished: o,
          destroyed: r,
          pos: i,
        } = this;
        return (
          (e.destroyed = r),
          (e.finished = o),
          (e.length = s),
          (e.pos = i),
          s % a && e.buffer.set(n),
          e
        );
      }
      clone() {
        return this._cloneInto();
      }
    },
    bt = Uint32Array.from([
      1779033703, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924,
      528734635, 1541459225,
    ]);
  var Ee = Uint32Array.from([
    1779033703, 4089235720, 3144134277, 2227873595, 1013904242, 4271175723,
    2773480762, 1595750129, 1359893119, 2917565137, 2600822924, 725511199,
    528734635, 4215389547, 1541459225, 327033209,
  ]);
  var Cp = Uint8Array.from([
      7, 4, 13, 1, 10, 6, 15, 3, 12, 0, 9, 5, 2, 14, 11, 8,
    ]),
    Al = Uint8Array.from(new Array(16).fill(0).map((t, e) => e)),
    Lp = Al.map((t) => (9 * t + 5) % 16),
    Sl = (() => {
      let a = [[Al], [Lp]];
      for (let n = 0; n < 4; n++)
        for (let s of a) s.push(s[n].map((o) => Cp[o]));
      return a;
    })(),
    Cl = Sl[0],
    Ll = Sl[1],
    Tl = [
      [11, 14, 15, 12, 5, 8, 7, 9, 11, 13, 14, 15, 6, 7, 9, 8],
      [12, 13, 11, 15, 6, 9, 9, 7, 12, 15, 11, 13, 7, 8, 7, 7],
      [13, 15, 14, 11, 7, 7, 6, 8, 13, 14, 13, 12, 5, 5, 6, 9],
      [14, 11, 12, 14, 8, 6, 5, 5, 15, 12, 15, 14, 9, 9, 8, 6],
      [15, 12, 13, 13, 9, 5, 8, 6, 14, 11, 12, 11, 8, 6, 5, 5],
    ].map((t) => Uint8Array.from(t)),
    Tp = Cl.map((t, e) => t.map((a) => Tl[e][a])),
    Bp = Ll.map((t, e) => t.map((a) => Tl[e][a])),
    Mp = Uint32Array.from([0, 1518500249, 1859775393, 2400959708, 2840853838]),
    Rp = Uint32Array.from([1352829926, 1548603684, 1836072691, 2053994217, 0]);
  function $l(t, e, a, n) {
    return t === 0
      ? e ^ a ^ n
      : t === 1
        ? (e & a) | (~e & n)
        : t === 2
          ? (e | ~a) ^ n
          : t === 3
            ? (e & n) | (a & ~n)
            : e ^ (a | ~n);
  }
  var Us = new Uint32Array(16),
    Sr = class extends ua {
      h0 = 1732584193;
      h1 = -271733879;
      h2 = -1732584194;
      h3 = 271733878;
      h4 = -1009589776;
      constructor() {
        super(64, 20, 8, !0);
      }
      get() {
        let { h0: e, h1: a, h2: n, h3: s, h4: o } = this;
        return [e, a, n, s, o];
      }
      set(e, a, n, s, o) {
        (this.h0 = e | 0),
          (this.h1 = a | 0),
          (this.h2 = n | 0),
          (this.h3 = s | 0),
          (this.h4 = o | 0);
      }
      process(e, a) {
        for (let p = 0; p < 16; p++, a += 4) Us[p] = e.getUint32(a, !0);
        let n = this.h0 | 0,
          s = n,
          o = this.h1 | 0,
          r = o,
          i = this.h2 | 0,
          c = i,
          l = this.h3 | 0,
          u = l,
          d = this.h4 | 0,
          f = d;
        for (let p = 0; p < 5; p++) {
          let v = 4 - p,
            k = Mp[p],
            L = Rp[p],
            N = Cl[p],
            V = Ll[p],
            O = Tp[p],
            q = Bp[p];
          for (let K = 0; K < 16; K++) {
            let T = (Sn(n + $l(p, o, i, l) + Us[N[K]] + k, O[K]) + d) | 0;
            (n = d), (d = l), (l = Sn(i, 10) | 0), (i = o), (o = T);
          }
          for (let K = 0; K < 16; K++) {
            let T = (Sn(s + $l(v, r, c, u) + Us[V[K]] + L, q[K]) + f) | 0;
            (s = f), (f = u), (u = Sn(c, 10) | 0), (c = r), (r = T);
          }
        }
        this.set(
          (this.h1 + i + u) | 0,
          (this.h2 + l + f) | 0,
          (this.h3 + d + s) | 0,
          (this.h4 + n + r) | 0,
          (this.h0 + o + c) | 0,
        );
      }
      roundClean() {
        Fe(Us);
      }
      destroy() {
        (this.destroyed = !0), Fe(this.buffer), this.set(0, 0, 0, 0, 0);
      }
    },
    Bl = Ln(() => new Sr());
  var Ds = BigInt(4294967295),
    Ml = BigInt(32);
  function Ip(t, e = !1) {
    return e
      ? { h: Number(t & Ds), l: Number((t >> Ml) & Ds) }
      : { h: Number((t >> Ml) & Ds) | 0, l: Number(t & Ds) | 0 };
  }
  function Rl(t, e = !1) {
    let a = t.length,
      n = new Uint32Array(a),
      s = new Uint32Array(a);
    for (let o = 0; o < a; o++) {
      let { h: r, l: i } = Ip(t[o], e);
      [n[o], s[o]] = [r, i];
    }
    return [n, s];
  }
  var Cr = (t, e, a) => t >>> a,
    Lr = (t, e, a) => (t << (32 - a)) | (e >>> a),
    fa = (t, e, a) => (t >>> a) | (e << (32 - a)),
    pa = (t, e, a) => (t << (32 - a)) | (e >>> a),
    Tn = (t, e, a) => (t << (64 - a)) | (e >>> (a - 32)),
    Bn = (t, e, a) => (t >>> (a - 32)) | (e << (64 - a));
  function ot(t, e, a, n) {
    let s = (e >>> 0) + (n >>> 0);
    return { h: (t + a + ((s / 2 ** 32) | 0)) | 0, l: s | 0 };
  }
  var Il = (t, e, a) => (t >>> 0) + (e >>> 0) + (a >>> 0),
    Hl = (t, e, a, n) => (e + a + n + ((t / 2 ** 32) | 0)) | 0,
    Ul = (t, e, a, n) => (t >>> 0) + (e >>> 0) + (a >>> 0) + (n >>> 0),
    Dl = (t, e, a, n, s) => (e + a + n + s + ((t / 2 ** 32) | 0)) | 0,
    Nl = (t, e, a, n, s) =>
      (t >>> 0) + (e >>> 0) + (a >>> 0) + (n >>> 0) + (s >>> 0),
    _l = (t, e, a, n, s, o) => (e + a + n + s + o + ((t / 2 ** 32) | 0)) | 0;
  var Up = Uint32Array.from([
      1116352408, 1899447441, 3049323471, 3921009573, 961987163, 1508970993,
      2453635748, 2870763221, 3624381080, 310598401, 607225278, 1426881987,
      1925078388, 2162078206, 2614888103, 3248222580, 3835390401, 4022224774,
      264347078, 604807628, 770255983, 1249150122, 1555081692, 1996064986,
      2554220882, 2821834349, 2952996808, 3210313671, 3336571891, 3584528711,
      113926993, 338241895, 666307205, 773529912, 1294757372, 1396182291,
      1695183700, 1986661051, 2177026350, 2456956037, 2730485921, 2820302411,
      3259730800, 3345764771, 3516065817, 3600352804, 4094571909, 275423344,
      430227734, 506948616, 659060556, 883997877, 958139571, 1322822218,
      1537002063, 1747873779, 1955562222, 2024104815, 2227730452, 2361852424,
      2428436474, 2756734187, 3204031479, 3329325298,
    ]),
    Ht = new Uint32Array(64),
    Tr = class extends ua {
      constructor(e) {
        super(64, e, 8, !1);
      }
      get() {
        let { A: e, B: a, C: n, D: s, E: o, F: r, G: i, H: c } = this;
        return [e, a, n, s, o, r, i, c];
      }
      set(e, a, n, s, o, r, i, c) {
        (this.A = e | 0),
          (this.B = a | 0),
          (this.C = n | 0),
          (this.D = s | 0),
          (this.E = o | 0),
          (this.F = r | 0),
          (this.G = i | 0),
          (this.H = c | 0);
      }
      process(e, a) {
        for (let d = 0; d < 16; d++, a += 4) Ht[d] = e.getUint32(a, !1);
        for (let d = 16; d < 64; d++) {
          let f = Ht[d - 15],
            p = Ht[d - 2],
            v = Xe(f, 7) ^ Xe(f, 18) ^ (f >>> 3),
            k = Xe(p, 17) ^ Xe(p, 19) ^ (p >>> 10);
          Ht[d] = (k + Ht[d - 7] + v + Ht[d - 16]) | 0;
        }
        let { A: n, B: s, C: o, D: r, E: i, F: c, G: l, H: u } = this;
        for (let d = 0; d < 64; d++) {
          let f = Xe(i, 6) ^ Xe(i, 11) ^ Xe(i, 25),
            p = (u + f + $r(i, c, l) + Up[d] + Ht[d]) | 0,
            k = ((Xe(n, 2) ^ Xe(n, 13) ^ Xe(n, 22)) + Ar(n, s, o)) | 0;
          (u = l),
            (l = c),
            (c = i),
            (i = (r + p) | 0),
            (r = o),
            (o = s),
            (s = n),
            (n = (p + k) | 0);
        }
        (n = (n + this.A) | 0),
          (s = (s + this.B) | 0),
          (o = (o + this.C) | 0),
          (r = (r + this.D) | 0),
          (i = (i + this.E) | 0),
          (c = (c + this.F) | 0),
          (l = (l + this.G) | 0),
          (u = (u + this.H) | 0),
          this.set(n, s, o, r, i, c, l, u);
      }
      roundClean() {
        Fe(Ht);
      }
      destroy() {
        this.set(0, 0, 0, 0, 0, 0, 0, 0), Fe(this.buffer);
      }
    },
    Br = class extends Tr {
      A = bt[0] | 0;
      B = bt[1] | 0;
      C = bt[2] | 0;
      D = bt[3] | 0;
      E = bt[4] | 0;
      F = bt[5] | 0;
      G = bt[6] | 0;
      H = bt[7] | 0;
      constructor() {
        super(32);
      }
    };
  var Ol = Rl(
      [
        "0x428a2f98d728ae22",
        "0x7137449123ef65cd",
        "0xb5c0fbcfec4d3b2f",
        "0xe9b5dba58189dbbc",
        "0x3956c25bf348b538",
        "0x59f111f1b605d019",
        "0x923f82a4af194f9b",
        "0xab1c5ed5da6d8118",
        "0xd807aa98a3030242",
        "0x12835b0145706fbe",
        "0x243185be4ee4b28c",
        "0x550c7dc3d5ffb4e2",
        "0x72be5d74f27b896f",
        "0x80deb1fe3b1696b1",
        "0x9bdc06a725c71235",
        "0xc19bf174cf692694",
        "0xe49b69c19ef14ad2",
        "0xefbe4786384f25e3",
        "0x0fc19dc68b8cd5b5",
        "0x240ca1cc77ac9c65",
        "0x2de92c6f592b0275",
        "0x4a7484aa6ea6e483",
        "0x5cb0a9dcbd41fbd4",
        "0x76f988da831153b5",
        "0x983e5152ee66dfab",
        "0xa831c66d2db43210",
        "0xb00327c898fb213f",
        "0xbf597fc7beef0ee4",
        "0xc6e00bf33da88fc2",
        "0xd5a79147930aa725",
        "0x06ca6351e003826f",
        "0x142929670a0e6e70",
        "0x27b70a8546d22ffc",
        "0x2e1b21385c26c926",
        "0x4d2c6dfc5ac42aed",
        "0x53380d139d95b3df",
        "0x650a73548baf63de",
        "0x766a0abb3c77b2a8",
        "0x81c2c92e47edaee6",
        "0x92722c851482353b",
        "0xa2bfe8a14cf10364",
        "0xa81a664bbc423001",
        "0xc24b8b70d0f89791",
        "0xc76c51a30654be30",
        "0xd192e819d6ef5218",
        "0xd69906245565a910",
        "0xf40e35855771202a",
        "0x106aa07032bbd1b8",
        "0x19a4c116b8d2d0c8",
        "0x1e376c085141ab53",
        "0x2748774cdf8eeb99",
        "0x34b0bcb5e19b48a8",
        "0x391c0cb3c5c95a63",
        "0x4ed8aa4ae3418acb",
        "0x5b9cca4f7763e373",
        "0x682e6ff3d6b2b8a3",
        "0x748f82ee5defb2fc",
        "0x78a5636f43172f60",
        "0x84c87814a1f0ab72",
        "0x8cc702081a6439ec",
        "0x90befffa23631e28",
        "0xa4506cebde82bde9",
        "0xbef9a3f7b2c67915",
        "0xc67178f2e372532b",
        "0xca273eceea26619c",
        "0xd186b8c721c0c207",
        "0xeada7dd6cde0eb1e",
        "0xf57d4f7fee6ed178",
        "0x06f067aa72176fba",
        "0x0a637dc5a2c898a6",
        "0x113f9804bef90dae",
        "0x1b710b35131c471b",
        "0x28db77f523047d84",
        "0x32caab7b40c72493",
        "0x3c9ebe0a15c9bebc",
        "0x431d67c49c100d4c",
        "0x4cc5d4becb3e42b6",
        "0x597f299cfc657e2a",
        "0x5fcb6fab3ad6faec",
        "0x6c44198c4a475817",
      ].map((t) => BigInt(t)),
    ),
    Dp = Ol[0],
    Np = Ol[1],
    Ut = new Uint32Array(80),
    Dt = new Uint32Array(80),
    Mr = class extends ua {
      constructor(e) {
        super(128, e, 16, !1);
      }
      get() {
        let {
          Ah: e,
          Al: a,
          Bh: n,
          Bl: s,
          Ch: o,
          Cl: r,
          Dh: i,
          Dl: c,
          Eh: l,
          El: u,
          Fh: d,
          Fl: f,
          Gh: p,
          Gl: v,
          Hh: k,
          Hl: L,
        } = this;
        return [e, a, n, s, o, r, i, c, l, u, d, f, p, v, k, L];
      }
      set(e, a, n, s, o, r, i, c, l, u, d, f, p, v, k, L) {
        (this.Ah = e | 0),
          (this.Al = a | 0),
          (this.Bh = n | 0),
          (this.Bl = s | 0),
          (this.Ch = o | 0),
          (this.Cl = r | 0),
          (this.Dh = i | 0),
          (this.Dl = c | 0),
          (this.Eh = l | 0),
          (this.El = u | 0),
          (this.Fh = d | 0),
          (this.Fl = f | 0),
          (this.Gh = p | 0),
          (this.Gl = v | 0),
          (this.Hh = k | 0),
          (this.Hl = L | 0);
      }
      process(e, a) {
        for (let O = 0; O < 16; O++, a += 4)
          (Ut[O] = e.getUint32(a)), (Dt[O] = e.getUint32((a += 4)));
        for (let O = 16; O < 80; O++) {
          let q = Ut[O - 15] | 0,
            K = Dt[O - 15] | 0,
            T = fa(q, K, 1) ^ fa(q, K, 8) ^ Cr(q, K, 7),
            g = pa(q, K, 1) ^ pa(q, K, 8) ^ Lr(q, K, 7),
            w = Ut[O - 2] | 0,
            m = Dt[O - 2] | 0,
            $ = fa(w, m, 19) ^ Tn(w, m, 61) ^ Cr(w, m, 6),
            A = pa(w, m, 19) ^ Bn(w, m, 61) ^ Lr(w, m, 6),
            R = Ul(g, A, Dt[O - 7], Dt[O - 16]),
            E = Dl(R, T, $, Ut[O - 7], Ut[O - 16]);
          (Ut[O] = E | 0), (Dt[O] = R | 0);
        }
        let {
          Ah: n,
          Al: s,
          Bh: o,
          Bl: r,
          Ch: i,
          Cl: c,
          Dh: l,
          Dl: u,
          Eh: d,
          El: f,
          Fh: p,
          Fl: v,
          Gh: k,
          Gl: L,
          Hh: N,
          Hl: V,
        } = this;
        for (let O = 0; O < 80; O++) {
          let q = fa(d, f, 14) ^ fa(d, f, 18) ^ Tn(d, f, 41),
            K = pa(d, f, 14) ^ pa(d, f, 18) ^ Bn(d, f, 41),
            T = (d & p) ^ (~d & k),
            g = (f & v) ^ (~f & L),
            w = Nl(V, K, g, Np[O], Dt[O]),
            m = _l(w, N, q, T, Dp[O], Ut[O]),
            $ = w | 0,
            A = fa(n, s, 28) ^ Tn(n, s, 34) ^ Tn(n, s, 39),
            R = pa(n, s, 28) ^ Bn(n, s, 34) ^ Bn(n, s, 39),
            E = (n & o) ^ (n & i) ^ (o & i),
            B = (s & r) ^ (s & c) ^ (r & c);
          (N = k | 0),
            (V = L | 0),
            (k = p | 0),
            (L = v | 0),
            (p = d | 0),
            (v = f | 0),
            ({ h: d, l: f } = ot(l | 0, u | 0, m | 0, $ | 0)),
            (l = i | 0),
            (u = c | 0),
            (i = o | 0),
            (c = r | 0),
            (o = n | 0),
            (r = s | 0);
          let S = Il($, R, B);
          (n = Hl(S, m, A, E)), (s = S | 0);
        }
        ({ h: n, l: s } = ot(this.Ah | 0, this.Al | 0, n | 0, s | 0)),
          ({ h: o, l: r } = ot(this.Bh | 0, this.Bl | 0, o | 0, r | 0)),
          ({ h: i, l: c } = ot(this.Ch | 0, this.Cl | 0, i | 0, c | 0)),
          ({ h: l, l: u } = ot(this.Dh | 0, this.Dl | 0, l | 0, u | 0)),
          ({ h: d, l: f } = ot(this.Eh | 0, this.El | 0, d | 0, f | 0)),
          ({ h: p, l: v } = ot(this.Fh | 0, this.Fl | 0, p | 0, v | 0)),
          ({ h: k, l: L } = ot(this.Gh | 0, this.Gl | 0, k | 0, L | 0)),
          ({ h: N, l: V } = ot(this.Hh | 0, this.Hl | 0, N | 0, V | 0)),
          this.set(n, s, o, r, i, c, l, u, d, f, p, v, k, L, N, V);
      }
      roundClean() {
        Fe(Ut, Dt);
      }
      destroy() {
        Fe(this.buffer),
          this.set(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
      }
    },
    Rr = class extends Mr {
      Ah = Ee[0] | 0;
      Al = Ee[1] | 0;
      Bh = Ee[2] | 0;
      Bl = Ee[3] | 0;
      Ch = Ee[4] | 0;
      Cl = Ee[5] | 0;
      Dh = Ee[6] | 0;
      Dl = Ee[7] | 0;
      Eh = Ee[8] | 0;
      El = Ee[9] | 0;
      Fh = Ee[10] | 0;
      Fl = Ee[11] | 0;
      Gh = Ee[12] | 0;
      Gl = Ee[13] | 0;
      Hh = Ee[14] | 0;
      Hl = Ee[15] | 0;
      constructor() {
        super(64);
      }
    };
  var Ir = Ln(() => new Br(), Er(1));
  var Hr = Ln(() => new Rr(), Er(3));
  var Rn = ft.Point,
    { Fn: Mn } = Rn,
    Ur = hl(Ir),
    _p = Uint8Array.from("Bitcoin seed".split(""), (t) => t.charCodeAt(0)),
    Dr = { private: 76066276, public: 76067358 },
    Nr = 2147483648,
    Op = (t) => Bl(Ir(t)),
    jp = (t) => It(t).getUint32(0, !1),
    Ns = (t) => {
      if (!Number.isSafeInteger(t) || t < 0 || t > 2 ** 32 - 1)
        throw new Error(
          "invalid number, should be from 0 to 2**32-1, got " + t,
        );
      let e = new Uint8Array(4);
      return It(e).setUint32(0, t, !1), e;
    },
    _s = class t {
      get fingerprint() {
        if (!this.pubHash) throw new Error("No publicKey set!");
        return jp(this.pubHash);
      }
      get identifier() {
        return this.pubHash;
      }
      get pubKeyHash() {
        return this.pubHash;
      }
      get privateKey() {
        return this._privateKey || null;
      }
      get publicKey() {
        return this._publicKey || null;
      }
      get privateExtendedKey() {
        let e = this._privateKey;
        if (!e) throw new Error("No private key");
        return Ur.encode(
          this.serialize(this.versions.private, Cn(Uint8Array.of(0), e)),
        );
      }
      get publicExtendedKey() {
        if (!this._publicKey) throw new Error("No public key");
        return Ur.encode(this.serialize(this.versions.public, this._publicKey));
      }
      static fromMasterSeed(e, a = Dr) {
        if ((je(e), 8 * e.length < 128 || 8 * e.length > 512))
          throw new Error(
            "HDKey: seed length must be between 128 and 512 bits; 256 bits is advised, got " +
              e.length,
          );
        let n = Hs(Hr, _p, e),
          s = n.slice(0, 32),
          o = n.slice(32);
        return new t({ versions: a, chainCode: o, privateKey: s });
      }
      static fromExtendedKey(e, a = Dr) {
        let n = Ur.decode(e),
          s = It(n),
          o = s.getUint32(0, !1),
          r = {
            versions: a,
            depth: n[4],
            parentFingerprint: s.getUint32(5, !1),
            index: s.getUint32(9, !1),
            chainCode: n.slice(13, 45),
          },
          i = n.slice(45),
          c = i[0] === 0;
        if (o !== a[c ? "private" : "public"])
          throw new Error("Version mismatch");
        return c
          ? new t({ ...r, privateKey: i.slice(1) })
          : new t({ ...r, publicKey: i });
      }
      static fromJSON(e) {
        return t.fromExtendedKey(e.xpriv);
      }
      versions;
      depth = 0;
      index = 0;
      chainCode = null;
      parentFingerprint = 0;
      _privateKey;
      _publicKey;
      pubHash;
      constructor(e) {
        if (!e || typeof e != "object")
          throw new Error("HDKey.constructor must not be called directly");
        if (
          ((this.versions = e.versions || Dr),
          (this.depth = e.depth || 0),
          (this.chainCode = e.chainCode || null),
          (this.index = e.index || 0),
          (this.parentFingerprint = e.parentFingerprint || 0),
          !this.depth && (this.parentFingerprint || this.index))
        )
          throw new Error(
            "HDKey: zero depth with non-zero index/parent fingerprint",
          );
        if (this.depth > 255)
          throw new Error("HDKey: depth exceeds the serializable value 255");
        if (e.publicKey && e.privateKey)
          throw new Error("HDKey: publicKey and privateKey at same time.");
        if (e.privateKey) {
          if (!ft.utils.isValidSecretKey(e.privateKey))
            throw new Error("Invalid private key");
          (this._privateKey = e.privateKey),
            (this._publicKey = ft.getPublicKey(e.privateKey, !0));
        } else if (e.publicKey)
          this._publicKey = Rn.fromBytes(e.publicKey).toBytes(!0);
        else throw new Error("HDKey: no public or private key provided");
        this.pubHash = Op(this._publicKey);
      }
      derive(e) {
        if (!/^[mM]'?/.test(e))
          throw new Error('Path must start with "m" or "M"');
        if (/^[mM]'?$/.test(e)) return this;
        let a = e.replace(/^[mM]'?\//, "").split("/"),
          n = this;
        for (let s of a) {
          let o = /^(\d+)('?)$/.exec(s),
            r = o && o[1];
          if (!o || o.length !== 3 || typeof r != "string")
            throw new Error("invalid child index: " + s);
          let i = +r;
          if (!Number.isSafeInteger(i) || i >= Nr)
            throw new Error("Invalid index");
          o[2] === "'" && (i += Nr), (n = n.deriveChild(i));
        }
        return n;
      }
      deriveChild(e) {
        if (!this._publicKey || !this.chainCode)
          throw new Error("No publicKey or chainCode set");
        let a = Ns(e);
        if (e >= Nr) {
          let c = this._privateKey;
          if (!c) throw new Error("Could not derive hardened child key");
          a = Cn(Uint8Array.of(0), c, a);
        } else a = Cn(this._publicKey, a);
        let n = Hs(Hr, this.chainCode, a),
          s = n.slice(0, 32),
          o = n.slice(32);
        if (!ft.utils.isValidSecretKey(s))
          throw new Error("Tweak bigger than curve order");
        let r = {
            versions: this.versions,
            chainCode: o,
            depth: this.depth + 1,
            parentFingerprint: this.fingerprint,
            index: e,
          },
          i = Mn.fromBytes(s);
        try {
          if (this._privateKey) {
            let c = Mn.create(Mn.fromBytes(this._privateKey) + i);
            if (!Mn.isValidNot0(c))
              throw new Error(
                "The tweak was out of range or the resulted private key is invalid",
              );
            r.privateKey = Mn.toBytes(c);
          } else {
            let c = Rn.fromBytes(this._publicKey).add(Rn.BASE.multiply(i));
            if (c.equals(Rn.ZERO))
              throw new Error(
                "The tweak was equal to negative P, which made the result key invalid",
              );
            r.publicKey = c.toBytes(!0);
          }
          return new t(r);
        } catch {
          return this.deriveChild(e + 1);
        }
      }
      sign(e) {
        if (!this._privateKey) throw new Error("No privateKey set!");
        return je(e, 32), ft.sign(e, this._privateKey, { prehash: !1 });
      }
      verify(e, a) {
        if ((je(e, 32), je(a, 64), !this._publicKey))
          throw new Error("No publicKey set!");
        return ft.verify(a, e, this._publicKey, { prehash: !1 });
      }
      wipePrivateData() {
        return (
          this._privateKey &&
            (this._privateKey.fill(0), (this._privateKey = void 0)),
          this
        );
      }
      toJSON() {
        return { xpriv: this.privateExtendedKey, xpub: this.publicExtendedKey };
      }
      serialize(e, a) {
        if (!this.chainCode) throw new Error("No chainCode set");
        return (
          je(a, 33),
          Cn(
            Ns(e),
            new Uint8Array([this.depth]),
            Ns(this.parentFingerprint),
            Ns(this.index),
            this.chainCode,
            a,
          )
        );
      }
    };
  var qp = "m/44'/1237'";
  function _r(t, e, a = 0) {
    let s = _s.fromMasterSeed(vl(t, e)).derive(`${qp}/${a}'/0/0`).privateKey;
    if (!s) throw new Error("could not derive private key");
    return s;
  }
  function jl(t) {
    return (
      t instanceof Uint8Array ||
      (ArrayBuffer.isView(t) &&
        t.constructor.name === "Uint8Array" &&
        "BYTES_PER_ELEMENT" in t &&
        t.BYTES_PER_ELEMENT === 1)
    );
  }
  var Or = (t) => (t ? `"${t}" ` : "");
  function Os(t, e = "") {
    if (typeof t != "number")
      throw new TypeError(Or(e) + "expected number, got " + typeof t);
    if (!Number.isSafeInteger(t) || t < 0)
      throw new RangeError(Or(e) + "expected integer >= 0, got " + t);
    return t;
  }
  function Pa(t, e, a = "") {
    if (jl(t) && (e === void 0 || t.length === e)) return t;
    e !== void 0 && Os(e, "length");
    let n = jl(t),
      s = e !== void 0 ? ` of length ${e}` : "",
      o = n ? `length=${t.length}` : `type=${typeof t}`,
      r = Or(a) + "expected Uint8Array" + s + ", got " + o;
    throw n ? new RangeError(r) : new TypeError(r);
  }
  var Pp = (t, e) => {
      if (t === null || typeof t != "object" || Array.isArray(t))
        throw new TypeError(
          (e === "object" ? "" : `"${e}" `) +
            "expected object, got type=" +
            typeof t,
        );
    },
    ql = (t, e) => {
      Pp(t, e);
      let a = Object.getPrototypeOf(t);
      if (a !== Object.prototype && a !== null)
        throw new TypeError(`"${e}" expected plain object`);
      if (Object.hasOwn(t, "__proto__"))
        throw new TypeError(`"${e}.__proto__" is not allowed`);
    };
  function jr(t, e = !0) {
    if (t.destroyed) throw new Error("hash was destroyed");
    if (e && t.finished) throw new Error("digest() was already called");
  }
  function Vl(t, e) {
    Pa(t, void 0, "output");
    let a = e.outputLen;
    if (!(t.length >= a))
      throw new RangeError('"output" expected length >= ' + a);
  }
  function qr(...t) {
    for (let e = 0; e < t.length; e++) t[e].fill(0);
  }
  function js(t) {
    return new DataView(t.buffer, t.byteOffset, t.byteLength);
  }
  function Qe(t, e) {
    return (t << (32 - e)) | (t >>> e);
  }
  var zl =
      typeof Uint8Array.from([]).toHex == "function" &&
      typeof Uint8Array.fromHex == "function",
    Vp = Array.from({ length: 256 }, (t, e) => e.toString(16).padStart(2, "0"));
  function Nt(t) {
    if ((Pa(t), zl)) return t.toHex();
    let e = "";
    for (let a = 0; a < t.length; a++) e += Vp[t[a]];
    return e;
  }
  function Pl(t) {
    return t >= 48 && t <= 57
      ? t - 48
      : t >= 65 && t <= 70
        ? t - 55
        : t >= 97 && t <= 102
          ? t - 87
          : void 0;
  }
  function _t(t) {
    if (typeof t != "string")
      throw new TypeError("hex string expected, got " + typeof t);
    if (zl)
      try {
        return Uint8Array.fromHex(t);
      } catch (s) {
        throw s instanceof SyntaxError ? new RangeError(s.message) : s;
      }
    let e = t.length,
      a = e / 2;
    if (e % 2)
      throw new RangeError(
        "hex string expected, got unpadded hex of length " + e,
      );
    let n = new Uint8Array(a);
    for (let s = 0, o = 0; s < a; s++, o += 2) {
      let r = Pl(t.charCodeAt(o)),
        i = Pl(t.charCodeAt(o + 1));
      if (r === void 0 || i === void 0) {
        let c = t[o] + t[o + 1];
        throw new RangeError(
          'hex string expected, got non-hex character "' +
            c +
            '" at index ' +
            o,
        );
      }
      n[s] = r * 16 + i;
    }
    return n;
  }
  function zp(t, e, a = "opts") {
    return (
      ql(t, "defaults"),
      e !== void 0 && ql(e, a),
      Object.assign(Object.create(null), t, e)
    );
  }
  function Gl(t, e = {}) {
    if (typeof t != "function")
      throw new TypeError('"hashCons" expected function, got type=' + typeof t);
    e = zp({}, e, "info");
    let a = (s, o) => t(o).update(s).digest(),
      n = t(void 0);
    return (
      (a.outputLen = n.outputLen),
      (a.blockLen = n.blockLen),
      (a.canXOF = n.canXOF),
      (a.create = (s) => t(s)),
      Object.assign(a, e),
      Object.freeze(a)
    );
  }
  function Kl(t = 32) {
    Os(t, "bytesLength");
    let e = typeof globalThis == "object" ? globalThis.crypto : null;
    if (typeof e?.getRandomValues != "function")
      throw new Error("crypto.getRandomValues must be defined");
    if (t > 65536)
      throw new RangeError(`"bytesLength" expected <= 65536, got ${t}`);
    return e.getRandomValues(new Uint8Array(t));
  }
  var Fl = (t) => ({
    oid: Uint8Array.from([6, 9, 96, 134, 72, 1, 101, 3, 4, 2, t]),
  });
  var Gp = (t) => (t / 2 ** 32) | 0,
    Kp = (t) => t >>> 0;
  function Wl(t, e, a, n) {
    let s = Gp(a),
      o = Kp(a);
    t.setUint32(e, n ? o : s, n), t.setUint32(e + 4, n ? s : o, n);
  }
  function Yl(t, e, a) {
    return (t & e) ^ (~t & a);
  }
  function Zl(t, e, a) {
    return (t & e) ^ (t & a) ^ (e & a);
  }
  var qs = class {
      blockLen;
      outputLen;
      canXOF = !1;
      padOffset;
      isLE;
      buffer;
      view;
      finished = !1;
      length = 0;
      pos = 0;
      destroyed = !1;
      constructor(e, a, n, s) {
        (this.blockLen = e),
          (this.outputLen = a),
          (this.padOffset = n),
          (this.isLE = s),
          (this.buffer = new Uint8Array(e)),
          (this.view = js(this.buffer));
      }
      update(e) {
        jr(this), Pa(e);
        let { view: a, buffer: n, blockLen: s } = this,
          o = e.length,
          r = !1;
        for (let i = 0; i < o; ) {
          let c = Math.min(s - this.pos, o - i);
          if (c === s) {
            let l = js(e);
            for (; s <= o - i; i += s) this.process(l, i);
            r = !0;
            continue;
          }
          n.set(i === 0 && c === o ? e : e.subarray(i, i + c), this.pos),
            (this.pos += c),
            (i += c),
            this.pos === s && (this.process(a, 0), (this.pos = 0), (r = !0));
        }
        return (this.length += e.length), r && this.roundClean(), this;
      }
      digestInto(e) {
        jr(this), Vl(e, this), (this.finished = !0);
        let { buffer: a, view: n, blockLen: s, isLE: o } = this,
          { pos: r } = this;
        (a[r++] = 128),
          a.fill(0, r),
          this.padOffset > s - r && (this.process(n, 0), a.fill(0)),
          Wl(n, s - 8, this.length * 8, o),
          this.process(n, 0),
          this.roundClean();
        let i = e === a ? n : js(e),
          c = this.outputLen,
          l = c / 4,
          u = this.get();
        if (c % 4 || l > u.length) throw new Error("invalid outputLen");
        for (let d = 0; d < l; d++) i.setUint32(4 * d, u[d], o);
      }
      digest() {
        let { buffer: e, outputLen: a } = this;
        this.digestInto(e);
        let n = e.slice(0, a);
        return this.destroy(), n;
      }
      _cloneIntoMeta(e) {
        let { buffer: a, length: n, finished: s, destroyed: o, pos: r } = this;
        return (
          (e.destroyed = o),
          (e.finished = s),
          (e.length = n),
          (e.pos = r),
          r && e.buffer.set(a),
          e
        );
      }
      clone() {
        return this._cloneInto();
      }
    },
    Jl = Uint32Array.from([
      1779033703, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924,
      528734635, 1541459225,
    ]);
  var Fp = Uint32Array.from([
      1116352408, 1899447441, 3049323471, 3921009573, 961987163, 1508970993,
      2453635748, 2870763221, 3624381080, 310598401, 607225278, 1426881987,
      1925078388, 2162078206, 2614888103, 3248222580, 3835390401, 4022224774,
      264347078, 604807628, 770255983, 1249150122, 1555081692, 1996064986,
      2554220882, 2821834349, 2952996808, 3210313671, 3336571891, 3584528711,
      113926993, 338241895, 666307205, 773529912, 1294757372, 1396182291,
      1695183700, 1986661051, 2177026350, 2456956037, 2730485921, 2820302411,
      3259730800, 3345764771, 3516065817, 3600352804, 4094571909, 275423344,
      430227734, 506948616, 659060556, 883997877, 958139571, 1322822218,
      1537002063, 1747873779, 1955562222, 2024104815, 2227730452, 2361852424,
      2428436474, 2756734187, 3204031479, 3329325298,
    ]),
    Ot = new Uint32Array(64),
    Pr = class extends qs {
      A = 0;
      B = 0;
      C = 0;
      D = 0;
      E = 0;
      F = 0;
      G = 0;
      H = 0;
      constructor(e, a) {
        super(64, e, 8, !1),
          (this.A = a[0] | 0),
          (this.B = a[1] | 0),
          (this.C = a[2] | 0),
          (this.D = a[3] | 0),
          (this.E = a[4] | 0),
          (this.F = a[5] | 0),
          (this.G = a[6] | 0),
          (this.H = a[7] | 0);
      }
      get() {
        let { A: e, B: a, C: n, D: s, E: o, F: r, G: i, H: c } = this;
        return [e, a, n, s, o, r, i, c];
      }
      set(e, a, n, s, o, r, i, c) {
        (this.A = e | 0),
          (this.B = a | 0),
          (this.C = n | 0),
          (this.D = s | 0),
          (this.E = o | 0),
          (this.F = r | 0),
          (this.G = i | 0),
          (this.H = c | 0);
      }
      _cloneInto(e) {
        return (
          (e ||= new this.constructor()).set(...this.get()),
          this._cloneIntoMeta(e)
        );
      }
      process(e, a) {
        for (let d = 0; d < 16; d++, a += 4) Ot[d] = e.getUint32(a, !1);
        for (let d = 16; d < 64; d++) {
          let f = Ot[d - 15],
            p = Ot[d - 2],
            v = Qe(f, 7) ^ Qe(f, 18) ^ (f >>> 3),
            k = Qe(p, 17) ^ Qe(p, 19) ^ (p >>> 10);
          Ot[d] = (k + Ot[d - 7] + v + Ot[d - 16]) | 0;
        }
        let { A: n, B: s, C: o, D: r, E: i, F: c, G: l, H: u } = this;
        for (let d = 0; d < 64; d++) {
          let f = Qe(i, 6) ^ Qe(i, 11) ^ Qe(i, 25),
            p = (u + f + Yl(i, c, l) + Fp[d] + Ot[d]) | 0,
            k = ((Qe(n, 2) ^ Qe(n, 13) ^ Qe(n, 22)) + Zl(n, s, o)) | 0;
          (u = l),
            (l = c),
            (c = i),
            (i = (r + p) | 0),
            (r = o),
            (o = s),
            (s = n),
            (n = (p + k) | 0);
        }
        (n = (n + this.A) | 0),
          (s = (s + this.B) | 0),
          (o = (o + this.C) | 0),
          (r = (r + this.D) | 0),
          (i = (i + this.E) | 0),
          (c = (c + this.F) | 0),
          (l = (l + this.G) | 0),
          (u = (u + this.H) | 0),
          this.set(n, s, o, r, i, c, l, u);
      }
      roundClean() {
        qr(Ot);
      }
      destroy() {
        (this.destroyed = !0),
          this.set(0, 0, 0, 0, 0, 0, 0, 0),
          qr(this.buffer);
      }
    },
    Vr = class extends Pr {
      constructor() {
        super(32, Jl);
      }
    };
  var Va = Gl(() => new Vr(), Fl(1));
  var Wp = (t) => t[0] === "\u3042\u3044\u3053\u304F\u3057\u3093";
  function Xl(t) {
    for (let e = 0; e < t.length; e++) {
      let a = t.charCodeAt(e);
      if (a >= 55296 && a <= 56319) {
        if (e + 1 >= t.length) return !1;
        let n = t.charCodeAt(++e);
        if (n < 56320 || n > 57343) return !1;
      } else if (a >= 56320 && a <= 57343) return !1;
    }
    return !0;
  }
  function Yp(t) {
    if (typeof t != "string")
      throw new TypeError("invalid mnemonic type: " + typeof t);
    if (!Xl(t)) throw new TypeError("expected well-formed Unicode string");
    return t.normalize("NFKD");
  }
  function Zp(t) {
    let e = Yp(t),
      a = e.split(" ");
    if (![12, 15, 18, 21, 24].includes(a.length))
      throw new Error("Invalid mnemonic");
    return { nfkd: e, words: a };
  }
  function Ql(t) {
    if ((Pa(t), ![16, 20, 24, 28, 32].includes(t.length)))
      throw new RangeError("invalid entropy length");
  }
  function ed(t, e = 128) {
    if ((Os(e), e % 32 !== 0 || e > 256))
      throw new RangeError("Invalid entropy");
    return eh(Kl(e / 8), t);
  }
  var td = (t) => {
    let e = 8 - t.length / 4;
    return (Va(t)[0] >> e) << e;
  };
  function ad(t) {
    if (!Array.isArray(t) || t.length !== 2048 || typeof t[0] != "string")
      throw new TypeError("Wordlist: expected array of 2048 strings");
    t.forEach((e) => {
      if (typeof e != "string")
        throw new TypeError("wordlist: non-string element: " + e);
      if (!Xl(e))
        throw new TypeError("wordlist: expected well-formed Unicode string");
    });
  }
  function Jp(t, e) {
    ad(e);
    let a = new Uint8Array(t.length + 1);
    a.set(t), (a[t.length] = td(t));
    let n = [],
      s = 0,
      o = 0;
    for (let r of a)
      (s = (s << 8) | r),
        (o += 8),
        o >= 11 &&
          ((o -= 11), n.push(e[(s >>> o) & 2047]), (s &= (1 << o) - 1));
    return n;
  }
  function Xp(t, e) {
    ad(e);
    let a = (t.length / 3) * 4,
      n = new Uint8Array(a + 1),
      s = 0,
      o = 0,
      r = 0;
    for (let c of t) {
      let l = e.indexOf(c);
      if (l === -1) throw new Error("Unknown word: " + c);
      for (s = (s << 11) | l, o += 11; o >= 8; )
        (o -= 8), (n[r++] = (s >>> o) & 255);
      s &= (1 << o) - 1;
    }
    o > 0 && (n[r] = s << (8 - o));
    let i = n.subarray(0, a);
    if (n[a] !== td(i)) throw new Error("Invalid checksum");
    return Uint8Array.from(i);
  }
  function Qp(t, e) {
    let { words: a } = Zp(t),
      n = Xp(a, e);
    return Ql(n), n;
  }
  function eh(t, e) {
    return Ql(t), Jp(t, e).join(Wp(e) ? "\u3000" : " ");
  }
  function zr(t, e) {
    try {
      Qp(t, e);
    } catch {
      return !1;
    }
    return !0;
  }
  var Gr = Object.freeze(
    `abacate
abaixo
abalar
abater
abduzir
abelha
aberto
abismo
abotoar
abranger
abreviar
abrigar
abrupto
absinto
absoluto
absurdo
abutre
acabado
acalmar
acampar
acanhar
acaso
aceitar
acelerar
acenar
acervo
acessar
acetona
achatar
acidez
acima
acionado
acirrar
aclamar
aclive
acolhida
acomodar
acoplar
acordar
acumular
acusador
adaptar
adega
adentro
adepto
adequar
aderente
adesivo
adeus
adiante
aditivo
adjetivo
adjunto
admirar
adorar
adquirir
adubo
adverso
advogado
aeronave
afastar
aferir
afetivo
afinador
afivelar
aflito
afluente
afrontar
agachar
agarrar
agasalho
agenciar
agilizar
agiota
agitado
agora
agradar
agreste
agrupar
aguardar
agulha
ajoelhar
ajudar
ajustar
alameda
alarme
alastrar
alavanca
albergue
albino
alcatra
aldeia
alecrim
alegria
alertar
alface
alfinete
algum
alheio
aliar
alicate
alienar
alinhar
aliviar
almofada
alocar
alpiste
alterar
altitude
alucinar
alugar
aluno
alusivo
alvo
amaciar
amador
amarelo
amassar
ambas
ambiente
ameixa
amenizar
amido
amistoso
amizade
amolador
amontoar
amoroso
amostra
amparar
ampliar
ampola
anagrama
analisar
anarquia
anatomia
andaime
anel
anexo
angular
animar
anjo
anomalia
anotado
ansioso
anterior
anuidade
anunciar
anzol
apagador
apalpar
apanhado
apego
apelido
apertada
apesar
apetite
apito
aplauso
aplicada
apoio
apontar
aposta
aprendiz
aprovar
aquecer
arame
aranha
arara
arcada
ardente
areia
arejar
arenito
aresta
argiloso
argola
arma
arquivo
arraial
arrebate
arriscar
arroba
arrumar
arsenal
arterial
artigo
arvoredo
asfaltar
asilado
aspirar
assador
assinar
assoalho
assunto
astral
atacado
atadura
atalho
atarefar
atear
atender
aterro
ateu
atingir
atirador
ativo
atoleiro
atracar
atrevido
atriz
atual
atum
auditor
aumentar
aura
aurora
autismo
autoria
autuar
avaliar
avante
avaria
avental
avesso
aviador
avisar
avulso
axila
azarar
azedo
azeite
azulejo
babar
babosa
bacalhau
bacharel
bacia
bagagem
baiano
bailar
baioneta
bairro
baixista
bajular
baleia
baliza
balsa
banal
bandeira
banho
banir
banquete
barato
barbado
baronesa
barraca
barulho
baseado
bastante
batata
batedor
batida
batom
batucar
baunilha
beber
beijo
beirada
beisebol
beldade
beleza
belga
beliscar
bendito
bengala
benzer
berimbau
berlinda
berro
besouro
bexiga
bezerro
bico
bicudo
bienal
bifocal
bifurcar
bigorna
bilhete
bimestre
bimotor
biologia
biombo
biosfera
bipolar
birrento
biscoito
bisneto
bispo
bissexto
bitola
bizarro
blindado
bloco
bloquear
boato
bobagem
bocado
bocejo
bochecha
boicotar
bolada
boletim
bolha
bolo
bombeiro
bonde
boneco
bonita
borbulha
borda
boreal
borracha
bovino
boxeador
branco
brasa
braveza
breu
briga
brilho
brincar
broa
brochura
bronzear
broto
bruxo
bucha
budismo
bufar
bule
buraco
busca
busto
buzina
cabana
cabelo
cabide
cabo
cabrito
cacau
cacetada
cachorro
cacique
cadastro
cadeado
cafezal
caiaque
caipira
caixote
cajado
caju
calafrio
calcular
caldeira
calibrar
calmante
calota
camada
cambista
camisa
camomila
campanha
camuflar
canavial
cancelar
caneta
canguru
canhoto
canivete
canoa
cansado
cantar
canudo
capacho
capela
capinar
capotar
capricho
captador
capuz
caracol
carbono
cardeal
careca
carimbar
carneiro
carpete
carreira
cartaz
carvalho
casaco
casca
casebre
castelo
casulo
catarata
cativar
caule
causador
cautelar
cavalo
caverna
cebola
cedilha
cegonha
celebrar
celular
cenoura
censo
centeio
cercar
cerrado
certeiro
cerveja
cetim
cevada
chacota
chaleira
chamado
chapada
charme
chatice
chave
chefe
chegada
cheiro
cheque
chicote
chifre
chinelo
chocalho
chover
chumbo
chutar
chuva
cicatriz
ciclone
cidade
cidreira
ciente
cigana
cimento
cinto
cinza
ciranda
circuito
cirurgia
citar
clareza
clero
clicar
clone
clube
coado
coagir
cobaia
cobertor
cobrar
cocada
coelho
coentro
coeso
cogumelo
coibir
coifa
coiote
colar
coleira
colher
colidir
colmeia
colono
coluna
comando
combinar
comentar
comitiva
comover
complexo
comum
concha
condor
conectar
confuso
congelar
conhecer
conjugar
consumir
contrato
convite
cooperar
copeiro
copiador
copo
coquetel
coragem
cordial
corneta
coronha
corporal
correio
cortejo
coruja
corvo
cosseno
costela
cotonete
couro
couve
covil
cozinha
cratera
cravo
creche
credor
creme
crer
crespo
criada
criminal
crioulo
crise
criticar
crosta
crua
cruzeiro
cubano
cueca
cuidado
cujo
culatra
culminar
culpar
cultura
cumprir
cunhado
cupido
curativo
curral
cursar
curto
cuspir
custear
cutelo
damasco
datar
debater
debitar
deboche
debulhar
decalque
decimal
declive
decote
decretar
dedal
dedicado
deduzir
defesa
defumar
degelo
degrau
degustar
deitado
deixar
delator
delegado
delinear
delonga
demanda
demitir
demolido
dentista
depenado
depilar
depois
depressa
depurar
deriva
derramar
desafio
desbotar
descanso
desenho
desfiado
desgaste
desigual
deslize
desmamar
desova
despesa
destaque
desviar
detalhar
detentor
detonar
detrito
deusa
dever
devido
devotado
dezena
diagrama
dialeto
didata
difuso
digitar
dilatado
diluente
diminuir
dinastia
dinheiro
diocese
direto
discreta
disfarce
disparo
disquete
dissipar
distante
ditador
diurno
diverso
divisor
divulgar
dizer
dobrador
dolorido
domador
dominado
donativo
donzela
dormente
dorsal
dosagem
dourado
doutor
drenagem
drible
drogaria
duelar
duende
dueto
duplo
duquesa
durante
duvidoso
eclodir
ecoar
ecologia
edificar
edital
educado
efeito
efetivar
ejetar
elaborar
eleger
eleitor
elenco
elevador
eliminar
elogiar
embargo
embolado
embrulho
embutido
emenda
emergir
emissor
empatia
empenho
empinado
empolgar
emprego
empurrar
emulador
encaixe
encenado
enchente
encontro
endeusar
endossar
enfaixar
enfeite
enfim
engajado
engenho
englobar
engomado
engraxar
enguia
enjoar
enlatar
enquanto
enraizar
enrolado
enrugar
ensaio
enseada
ensino
ensopado
entanto
enteado
entidade
entortar
entrada
entulho
envergar
enviado
envolver
enxame
enxerto
enxofre
enxuto
epiderme
equipar
ereto
erguido
errata
erva
ervilha
esbanjar
esbelto
escama
escola
escrita
escuta
esfinge
esfolar
esfregar
esfumado
esgrima
esmalte
espanto
espelho
espiga
esponja
espreita
espumar
esquerda
estaca
esteira
esticar
estofado
estrela
estudo
esvaziar
etanol
etiqueta
euforia
europeu
evacuar
evaporar
evasivo
eventual
evidente
evoluir
exagero
exalar
examinar
exato
exausto
excesso
excitar
exclamar
executar
exemplo
exibir
exigente
exonerar
expandir
expelir
expirar
explanar
exposto
expresso
expulsar
externo
extinto
extrato
fabricar
fabuloso
faceta
facial
fada
fadiga
faixa
falar
falta
familiar
fandango
fanfarra
fantoche
fardado
farelo
farinha
farofa
farpa
fartura
fatia
fator
favorita
faxina
fazenda
fechado
feijoada
feirante
felino
feminino
fenda
feno
fera
feriado
ferrugem
ferver
festejar
fetal
feudal
fiapo
fibrose
ficar
ficheiro
figurado
fileira
filho
filme
filtrar
firmeza
fisgada
fissura
fita
fivela
fixador
fixo
flacidez
flamingo
flanela
flechada
flora
flutuar
fluxo
focal
focinho
fofocar
fogo
foguete
foice
folgado
folheto
forjar
formiga
forno
forte
fosco
fossa
fragata
fralda
frango
frasco
fraterno
freira
frente
fretar
frieza
friso
fritura
fronha
frustrar
fruteira
fugir
fulano
fuligem
fundar
fungo
funil
furador
furioso
futebol
gabarito
gabinete
gado
gaiato
gaiola
gaivota
galega
galho
galinha
galocha
ganhar
garagem
garfo
gargalo
garimpo
garoupa
garrafa
gasoduto
gasto
gata
gatilho
gaveta
gazela
gelado
geleia
gelo
gemada
gemer
gemido
generoso
gengiva
genial
genoma
genro
geologia
gerador
germinar
gesso
gestor
ginasta
gincana
gingado
girafa
girino
glacial
glicose
global
glorioso
goela
goiaba
golfe
golpear
gordura
gorjeta
gorro
gostoso
goteira
governar
gracejo
gradual
grafite
gralha
grampo
granada
gratuito
graveto
graxa
grego
grelhar
greve
grilo
grisalho
gritaria
grosso
grotesco
grudado
grunhido
gruta
guache
guarani
guaxinim
guerrear
guiar
guincho
guisado
gula
guloso
guru
habitar
harmonia
haste
haver
hectare
herdar
heresia
hesitar
hiato
hibernar
hidratar
hiena
hino
hipismo
hipnose
hipoteca
hoje
holofote
homem
honesto
honrado
hormonal
hospedar
humorado
iate
ideia
idoso
ignorado
igreja
iguana
ileso
ilha
iludido
iluminar
ilustrar
imagem
imediato
imenso
imersivo
iminente
imitador
imortal
impacto
impedir
implante
impor
imprensa
impune
imunizar
inalador
inapto
inativo
incenso
inchar
incidir
incluir
incolor
indeciso
indireto
indutor
ineficaz
inerente
infantil
infestar
infinito
inflamar
informal
infrator
ingerir
inibido
inicial
inimigo
injetar
inocente
inodoro
inovador
inox
inquieto
inscrito
inseto
insistir
inspetor
instalar
insulto
intacto
integral
intimar
intocado
intriga
invasor
inverno
invicto
invocar
iogurte
iraniano
ironizar
irreal
irritado
isca
isento
isolado
isqueiro
italiano
janeiro
jangada
janta
jararaca
jardim
jarro
jasmim
jato
javali
jazida
jejum
joaninha
joelhada
jogador
joia
jornal
jorrar
jovem
juba
judeu
judoca
juiz
julgador
julho
jurado
jurista
juro
justa
labareda
laboral
lacre
lactante
ladrilho
lagarta
lagoa
laje
lamber
lamentar
laminar
lampejo
lanche
lapidar
lapso
laranja
lareira
largura
lasanha
lastro
lateral
latido
lavanda
lavoura
lavrador
laxante
lazer
lealdade
lebre
legado
legendar
legista
leigo
leiloar
leitura
lembrete
leme
lenhador
lentilha
leoa
lesma
leste
letivo
letreiro
levar
leveza
levitar
liberal
libido
liderar
ligar
ligeiro
limitar
limoeiro
limpador
linda
linear
linhagem
liquidez
listagem
lisura
litoral
livro
lixa
lixeira
locador
locutor
lojista
lombo
lona
longe
lontra
lorde
lotado
loteria
loucura
lousa
louvar
luar
lucidez
lucro
luneta
lustre
lutador
luva
macaco
macete
machado
macio
madeira
madrinha
magnata
magreza
maior
mais
malandro
malha
malote
maluco
mamilo
mamoeiro
mamute
manada
mancha
mandato
manequim
manhoso
manivela
manobrar
mansa
manter
manusear
mapeado
maquinar
marcador
maresia
marfim
margem
marinho
marmita
maroto
marquise
marreco
martelo
marujo
mascote
masmorra
massagem
mastigar
matagal
materno
matinal
matutar
maxilar
medalha
medida
medusa
megafone
meiga
melancia
melhor
membro
memorial
menino
menos
mensagem
mental
merecer
mergulho
mesada
mesclar
mesmo
mesquita
mestre
metade
meteoro
metragem
mexer
mexicano
micro
migalha
migrar
milagre
milenar
milhar
mimado
minerar
minhoca
ministro
minoria
miolo
mirante
mirtilo
misturar
mocidade
moderno
modular
moeda
moer
moinho
moita
moldura
moleza
molho
molinete
molusco
montanha
moqueca
morango
morcego
mordomo
morena
mosaico
mosquete
mostarda
motel
motim
moto
motriz
muda
muito
mulata
mulher
multar
mundial
munido
muralha
murcho
muscular
museu
musical
nacional
nadador
naja
namoro
narina
narrado
nascer
nativa
natureza
navalha
navegar
navio
neblina
nebuloso
negativa
negociar
negrito
nervoso
neta
neural
nevasca
nevoeiro
ninar
ninho
nitidez
nivelar
nobreza
noite
noiva
nomear
nominal
nordeste
nortear
notar
noticiar
noturno
novelo
novilho
novo
nublado
nudez
numeral
nupcial
nutrir
nuvem
obcecado
obedecer
objetivo
obrigado
obscuro
obstetra
obter
obturar
ocidente
ocioso
ocorrer
oculista
ocupado
ofegante
ofensiva
oferenda
oficina
ofuscado
ogiva
olaria
oleoso
olhar
oliveira
ombro
omelete
omisso
omitir
ondulado
oneroso
ontem
opcional
operador
oponente
oportuno
oposto
orar
orbitar
ordem
ordinal
orfanato
orgasmo
orgulho
oriental
origem
oriundo
orla
ortodoxo
orvalho
oscilar
ossada
osso
ostentar
otimismo
ousadia
outono
outubro
ouvido
ovelha
ovular
oxidar
oxigenar
pacato
paciente
pacote
pactuar
padaria
padrinho
pagar
pagode
painel
pairar
paisagem
palavra
palestra
palheta
palito
palmada
palpitar
pancada
panela
panfleto
panqueca
pantanal
papagaio
papelada
papiro
parafina
parcial
pardal
parede
partida
pasmo
passado
pastel
patamar
patente
patinar
patrono
paulada
pausar
peculiar
pedalar
pedestre
pediatra
pedra
pegada
peitoral
peixe
pele
pelicano
penca
pendurar
peneira
penhasco
pensador
pente
perceber
perfeito
pergunta
perito
permitir
perna
perplexo
persiana
pertence
peruca
pescado
pesquisa
pessoa
petiscar
piada
picado
piedade
pigmento
pilastra
pilhado
pilotar
pimenta
pincel
pinguim
pinha
pinote
pintar
pioneiro
pipoca
piquete
piranha
pires
pirueta
piscar
pistola
pitanga
pivete
planta
plaqueta
platina
plebeu
plumagem
pluvial
pneu
poda
poeira
poetisa
polegada
policiar
poluente
polvilho
pomar
pomba
ponderar
pontaria
populoso
porta
possuir
postal
pote
poupar
pouso
povoar
praia
prancha
prato
praxe
prece
predador
prefeito
premiar
prensar
preparar
presilha
pretexto
prevenir
prezar
primata
princesa
prisma
privado
processo
produto
profeta
proibido
projeto
prometer
propagar
prosa
protetor
provador
publicar
pudim
pular
pulmonar
pulseira
punhal
punir
pupilo
pureza
puxador
quadra
quantia
quarto
quase
quebrar
queda
queijo
quente
querido
quimono
quina
quiosque
rabanada
rabisco
rachar
racionar
radial
raiar
rainha
raio
raiva
rajada
ralado
ramal
ranger
ranhura
rapadura
rapel
rapidez
raposa
raquete
raridade
rasante
rascunho
rasgar
raspador
rasteira
rasurar
ratazana
ratoeira
realeza
reanimar
reaver
rebaixar
rebelde
rebolar
recado
recente
recheio
recibo
recordar
recrutar
recuar
rede
redimir
redonda
reduzida
reenvio
refinar
refletir
refogar
refresco
refugiar
regalia
regime
regra
reinado
reitor
rejeitar
relativo
remador
remendo
remorso
renovado
reparo
repelir
repleto
repolho
represa
repudiar
requerer
resenha
resfriar
resgatar
residir
resolver
respeito
ressaca
restante
resumir
retalho
reter
retirar
retomada
retratar
revelar
revisor
revolta
riacho
rica
rigidez
rigoroso
rimar
ringue
risada
risco
risonho
robalo
rochedo
rodada
rodeio
rodovia
roedor
roleta
romano
roncar
rosado
roseira
rosto
rota
roteiro
rotina
rotular
rouco
roupa
roxo
rubro
rugido
rugoso
ruivo
rumo
rupestre
russo
sabor
saciar
sacola
sacudir
sadio
safira
saga
sagrada
saibro
salada
saleiro
salgado
saliva
salpicar
salsicha
saltar
salvador
sambar
samurai
sanar
sanfona
sangue
sanidade
sapato
sarda
sargento
sarjeta
saturar
saudade
saxofone
sazonal
secar
secular
seda
sedento
sediado
sedoso
sedutor
segmento
segredo
segundo
seiva
seleto
selvagem
semanal
semente
senador
senhor
sensual
sentado
separado
sereia
seringa
serra
servo
setembro
setor
sigilo
silhueta
silicone
simetria
simpatia
simular
sinal
sincero
singular
sinopse
sintonia
sirene
siri
situado
soberano
sobra
socorro
sogro
soja
solda
soletrar
solteiro
sombrio
sonata
sondar
sonegar
sonhador
sono
soprano
soquete
sorrir
sorteio
sossego
sotaque
soterrar
sovado
sozinho
suavizar
subida
submerso
subsolo
subtrair
sucata
sucesso
suco
sudeste
sufixo
sugador
sugerir
sujeito
sulfato
sumir
suor
superior
suplicar
suposto
suprimir
surdina
surfista
surpresa
surreal
surtir
suspiro
sustento
tabela
tablete
tabuada
tacho
tagarela
talher
talo
talvez
tamanho
tamborim
tampa
tangente
tanto
tapar
tapioca
tardio
tarefa
tarja
tarraxa
tatuagem
taurino
taxativo
taxista
teatral
tecer
tecido
teclado
tedioso
teia
teimar
telefone
telhado
tempero
tenente
tensor
tentar
termal
terno
terreno
tese
tesoura
testado
teto
textura
texugo
tiara
tigela
tijolo
timbrar
timidez
tingido
tinteiro
tiragem
titular
toalha
tocha
tolerar
tolice
tomada
tomilho
tonel
tontura
topete
tora
torcido
torneio
torque
torrada
torto
tostar
touca
toupeira
toxina
trabalho
tracejar
tradutor
trafegar
trajeto
trama
trancar
trapo
traseiro
tratador
travar
treino
tremer
trepidar
trevo
triagem
tribo
triciclo
tridente
trilogia
trindade
triplo
triturar
triunfal
trocar
trombeta
trova
trunfo
truque
tubular
tucano
tudo
tulipa
tupi
turbo
turma
turquesa
tutelar
tutorial
uivar
umbigo
unha
unidade
uniforme
urologia
urso
urtiga
urubu
usado
usina
usufruir
vacina
vadiar
vagaroso
vaidoso
vala
valente
validade
valores
vantagem
vaqueiro
varanda
vareta
varrer
vascular
vasilha
vassoura
vazar
vazio
veado
vedar
vegetar
veicular
veleiro
velhice
veludo
vencedor
vendaval
venerar
ventre
verbal
verdade
vereador
vergonha
vermelho
verniz
versar
vertente
vespa
vestido
vetorial
viaduto
viagem
viajar
viatura
vibrador
videira
vidraria
viela
viga
vigente
vigiar
vigorar
vilarejo
vinco
vinheta
vinil
violeta
virada
virtude
visitar
visto
vitral
viveiro
vizinho
voador
voar
vogal
volante
voleibol
voltagem
volumoso
vontade
vulto
vuvuzela
xadrez
xarope
xeque
xeretar
xerife
xingar
zangado
zarpar
zebu
zelador
zombar
zoologia
zumbido`.split(`
`),
  );
  var nd = Object.freeze(
    `abandon
ability
able
about
above
absent
absorb
abstract
absurd
abuse
access
accident
account
accuse
achieve
acid
acoustic
acquire
across
act
action
actor
actress
actual
adapt
add
addict
address
adjust
admit
adult
advance
advice
aerobic
affair
afford
afraid
again
age
agent
agree
ahead
aim
air
airport
aisle
alarm
album
alcohol
alert
alien
all
alley
allow
almost
alone
alpha
already
also
alter
always
amateur
amazing
among
amount
amused
analyst
anchor
ancient
anger
angle
angry
animal
ankle
announce
annual
another
answer
antenna
antique
anxiety
any
apart
apology
appear
apple
approve
april
arch
arctic
area
arena
argue
arm
armed
armor
army
around
arrange
arrest
arrive
arrow
art
artefact
artist
artwork
ask
aspect
assault
asset
assist
assume
asthma
athlete
atom
attack
attend
attitude
attract
auction
audit
august
aunt
author
auto
autumn
average
avocado
avoid
awake
aware
away
awesome
awful
awkward
axis
baby
bachelor
bacon
badge
bag
balance
balcony
ball
bamboo
banana
banner
bar
barely
bargain
barrel
base
basic
basket
battle
beach
bean
beauty
because
become
beef
before
begin
behave
behind
believe
below
belt
bench
benefit
best
betray
better
between
beyond
bicycle
bid
bike
bind
biology
bird
birth
bitter
black
blade
blame
blanket
blast
bleak
bless
blind
blood
blossom
blouse
blue
blur
blush
board
boat
body
boil
bomb
bone
bonus
book
boost
border
boring
borrow
boss
bottom
bounce
box
boy
bracket
brain
brand
brass
brave
bread
breeze
brick
bridge
brief
bright
bring
brisk
broccoli
broken
bronze
broom
brother
brown
brush
bubble
buddy
budget
buffalo
build
bulb
bulk
bullet
bundle
bunker
burden
burger
burst
bus
business
busy
butter
buyer
buzz
cabbage
cabin
cable
cactus
cage
cake
call
calm
camera
camp
can
canal
cancel
candy
cannon
canoe
canvas
canyon
capable
capital
captain
car
carbon
card
cargo
carpet
carry
cart
case
cash
casino
castle
casual
cat
catalog
catch
category
cattle
caught
cause
caution
cave
ceiling
celery
cement
census
century
cereal
certain
chair
chalk
champion
change
chaos
chapter
charge
chase
chat
cheap
check
cheese
chef
cherry
chest
chicken
chief
child
chimney
choice
choose
chronic
chuckle
chunk
churn
cigar
cinnamon
circle
citizen
city
civil
claim
clap
clarify
claw
clay
clean
clerk
clever
click
client
cliff
climb
clinic
clip
clock
clog
close
cloth
cloud
clown
club
clump
cluster
clutch
coach
coast
coconut
code
coffee
coil
coin
collect
color
column
combine
come
comfort
comic
common
company
concert
conduct
confirm
congress
connect
consider
control
convince
cook
cool
copper
copy
coral
core
corn
correct
cost
cotton
couch
country
couple
course
cousin
cover
coyote
crack
cradle
craft
cram
crane
crash
crater
crawl
crazy
cream
credit
creek
crew
cricket
crime
crisp
critic
crop
cross
crouch
crowd
crucial
cruel
cruise
crumble
crunch
crush
cry
crystal
cube
culture
cup
cupboard
curious
current
curtain
curve
cushion
custom
cute
cycle
dad
damage
damp
dance
danger
daring
dash
daughter
dawn
day
deal
debate
debris
decade
december
decide
decline
decorate
decrease
deer
defense
define
defy
degree
delay
deliver
demand
demise
denial
dentist
deny
depart
depend
deposit
depth
deputy
derive
describe
desert
design
desk
despair
destroy
detail
detect
develop
device
devote
diagram
dial
diamond
diary
dice
diesel
diet
differ
digital
dignity
dilemma
dinner
dinosaur
direct
dirt
disagree
discover
disease
dish
dismiss
disorder
display
distance
divert
divide
divorce
dizzy
doctor
document
dog
doll
dolphin
domain
donate
donkey
donor
door
dose
double
dove
draft
dragon
drama
drastic
draw
dream
dress
drift
drill
drink
drip
drive
drop
drum
dry
duck
dumb
dune
during
dust
dutch
duty
dwarf
dynamic
eager
eagle
early
earn
earth
easily
east
easy
echo
ecology
economy
edge
edit
educate
effort
egg
eight
either
elbow
elder
electric
elegant
element
elephant
elevator
elite
else
embark
embody
embrace
emerge
emotion
employ
empower
empty
enable
enact
end
endless
endorse
enemy
energy
enforce
engage
engine
enhance
enjoy
enlist
enough
enrich
enroll
ensure
enter
entire
entry
envelope
episode
equal
equip
era
erase
erode
erosion
error
erupt
escape
essay
essence
estate
eternal
ethics
evidence
evil
evoke
evolve
exact
example
excess
exchange
excite
exclude
excuse
execute
exercise
exhaust
exhibit
exile
exist
exit
exotic
expand
expect
expire
explain
expose
express
extend
extra
eye
eyebrow
fabric
face
faculty
fade
faint
faith
fall
false
fame
family
famous
fan
fancy
fantasy
farm
fashion
fat
fatal
father
fatigue
fault
favorite
feature
february
federal
fee
feed
feel
female
fence
festival
fetch
fever
few
fiber
fiction
field
figure
file
film
filter
final
find
fine
finger
finish
fire
firm
first
fiscal
fish
fit
fitness
fix
flag
flame
flash
flat
flavor
flee
flight
flip
float
flock
floor
flower
fluid
flush
fly
foam
focus
fog
foil
fold
follow
food
foot
force
forest
forget
fork
fortune
forum
forward
fossil
foster
found
fox
fragile
frame
frequent
fresh
friend
fringe
frog
front
frost
frown
frozen
fruit
fuel
fun
funny
furnace
fury
future
gadget
gain
galaxy
gallery
game
gap
garage
garbage
garden
garlic
garment
gas
gasp
gate
gather
gauge
gaze
general
genius
genre
gentle
genuine
gesture
ghost
giant
gift
giggle
ginger
giraffe
girl
give
glad
glance
glare
glass
glide
glimpse
globe
gloom
glory
glove
glow
glue
goat
goddess
gold
good
goose
gorilla
gospel
gossip
govern
gown
grab
grace
grain
grant
grape
grass
gravity
great
green
grid
grief
grit
grocery
group
grow
grunt
guard
guess
guide
guilt
guitar
gun
gym
habit
hair
half
hammer
hamster
hand
happy
harbor
hard
harsh
harvest
hat
have
hawk
hazard
head
health
heart
heavy
hedgehog
height
hello
helmet
help
hen
hero
hidden
high
hill
hint
hip
hire
history
hobby
hockey
hold
hole
holiday
hollow
home
honey
hood
hope
horn
horror
horse
hospital
host
hotel
hour
hover
hub
huge
human
humble
humor
hundred
hungry
hunt
hurdle
hurry
hurt
husband
hybrid
ice
icon
idea
identify
idle
ignore
ill
illegal
illness
image
imitate
immense
immune
impact
impose
improve
impulse
inch
include
income
increase
index
indicate
indoor
industry
infant
inflict
inform
inhale
inherit
initial
inject
injury
inmate
inner
innocent
input
inquiry
insane
insect
inside
inspire
install
intact
interest
into
invest
invite
involve
iron
island
isolate
issue
item
ivory
jacket
jaguar
jar
jazz
jealous
jeans
jelly
jewel
job
join
joke
journey
joy
judge
juice
jump
jungle
junior
junk
just
kangaroo
keen
keep
ketchup
key
kick
kid
kidney
kind
kingdom
kiss
kit
kitchen
kite
kitten
kiwi
knee
knife
knock
know
lab
label
labor
ladder
lady
lake
lamp
language
laptop
large
later
latin
laugh
laundry
lava
law
lawn
lawsuit
layer
lazy
leader
leaf
learn
leave
lecture
left
leg
legal
legend
leisure
lemon
lend
length
lens
leopard
lesson
letter
level
liar
liberty
library
license
life
lift
light
like
limb
limit
link
lion
liquid
list
little
live
lizard
load
loan
lobster
local
lock
logic
lonely
long
loop
lottery
loud
lounge
love
loyal
lucky
luggage
lumber
lunar
lunch
luxury
lyrics
machine
mad
magic
magnet
maid
mail
main
major
make
mammal
man
manage
mandate
mango
mansion
manual
maple
marble
march
margin
marine
market
marriage
mask
mass
master
match
material
math
matrix
matter
maximum
maze
meadow
mean
measure
meat
mechanic
medal
media
melody
melt
member
memory
mention
menu
mercy
merge
merit
merry
mesh
message
metal
method
middle
midnight
milk
million
mimic
mind
minimum
minor
minute
miracle
mirror
misery
miss
mistake
mix
mixed
mixture
mobile
model
modify
mom
moment
monitor
monkey
monster
month
moon
moral
more
morning
mosquito
mother
motion
motor
mountain
mouse
move
movie
much
muffin
mule
multiply
muscle
museum
mushroom
music
must
mutual
myself
mystery
myth
naive
name
napkin
narrow
nasty
nation
nature
near
neck
need
negative
neglect
neither
nephew
nerve
nest
net
network
neutral
never
news
next
nice
night
noble
noise
nominee
noodle
normal
north
nose
notable
note
nothing
notice
novel
now
nuclear
number
nurse
nut
oak
obey
object
oblige
obscure
observe
obtain
obvious
occur
ocean
october
odor
off
offer
office
often
oil
okay
old
olive
olympic
omit
once
one
onion
online
only
open
opera
opinion
oppose
option
orange
orbit
orchard
order
ordinary
organ
orient
original
orphan
ostrich
other
outdoor
outer
output
outside
oval
oven
over
own
owner
oxygen
oyster
ozone
pact
paddle
page
pair
palace
palm
panda
panel
panic
panther
paper
parade
parent
park
parrot
party
pass
patch
path
patient
patrol
pattern
pause
pave
payment
peace
peanut
pear
peasant
pelican
pen
penalty
pencil
people
pepper
perfect
permit
person
pet
phone
photo
phrase
physical
piano
picnic
picture
piece
pig
pigeon
pill
pilot
pink
pioneer
pipe
pistol
pitch
pizza
place
planet
plastic
plate
play
please
pledge
pluck
plug
plunge
poem
poet
point
polar
pole
police
pond
pony
pool
popular
portion
position
possible
post
potato
pottery
poverty
powder
power
practice
praise
predict
prefer
prepare
present
pretty
prevent
price
pride
primary
print
priority
prison
private
prize
problem
process
produce
profit
program
project
promote
proof
property
prosper
protect
proud
provide
public
pudding
pull
pulp
pulse
pumpkin
punch
pupil
puppy
purchase
purity
purpose
purse
push
put
puzzle
pyramid
quality
quantum
quarter
question
quick
quit
quiz
quote
rabbit
raccoon
race
rack
radar
radio
rail
rain
raise
rally
ramp
ranch
random
range
rapid
rare
rate
rather
raven
raw
razor
ready
real
reason
rebel
rebuild
recall
receive
recipe
record
recycle
reduce
reflect
reform
refuse
region
regret
regular
reject
relax
release
relief
rely
remain
remember
remind
remove
render
renew
rent
reopen
repair
repeat
replace
report
require
rescue
resemble
resist
resource
response
result
retire
retreat
return
reunion
reveal
review
reward
rhythm
rib
ribbon
rice
rich
ride
ridge
rifle
right
rigid
ring
riot
ripple
risk
ritual
rival
river
road
roast
robot
robust
rocket
romance
roof
rookie
room
rose
rotate
rough
round
route
royal
rubber
rude
rug
rule
run
runway
rural
sad
saddle
sadness
safe
sail
salad
salmon
salon
salt
salute
same
sample
sand
satisfy
satoshi
sauce
sausage
save
say
scale
scan
scare
scatter
scene
scheme
school
science
scissors
scorpion
scout
scrap
screen
script
scrub
sea
search
season
seat
second
secret
section
security
seed
seek
segment
select
sell
seminar
senior
sense
sentence
series
service
session
settle
setup
seven
shadow
shaft
shallow
share
shed
shell
sheriff
shield
shift
shine
ship
shiver
shock
shoe
shoot
shop
short
shoulder
shove
shrimp
shrug
shuffle
shy
sibling
sick
side
siege
sight
sign
silent
silk
silly
silver
similar
simple
since
sing
siren
sister
situate
six
size
skate
sketch
ski
skill
skin
skirt
skull
slab
slam
sleep
slender
slice
slide
slight
slim
slogan
slot
slow
slush
small
smart
smile
smoke
smooth
snack
snake
snap
sniff
snow
soap
soccer
social
sock
soda
soft
solar
soldier
solid
solution
solve
someone
song
soon
sorry
sort
soul
sound
soup
source
south
space
spare
spatial
spawn
speak
special
speed
spell
spend
sphere
spice
spider
spike
spin
spirit
split
spoil
sponsor
spoon
sport
spot
spray
spread
spring
spy
square
squeeze
squirrel
stable
stadium
staff
stage
stairs
stamp
stand
start
state
stay
steak
steel
stem
step
stereo
stick
still
sting
stock
stomach
stone
stool
story
stove
strategy
street
strike
strong
struggle
student
stuff
stumble
style
subject
submit
subway
success
such
sudden
suffer
sugar
suggest
suit
summer
sun
sunny
sunset
super
supply
supreme
sure
surface
surge
surprise
surround
survey
suspect
sustain
swallow
swamp
swap
swarm
swear
sweet
swift
swim
swing
switch
sword
symbol
symptom
syrup
system
table
tackle
tag
tail
talent
talk
tank
tape
target
task
taste
tattoo
taxi
teach
team
tell
ten
tenant
tennis
tent
term
test
text
thank
that
theme
then
theory
there
they
thing
this
thought
three
thrive
throw
thumb
thunder
ticket
tide
tiger
tilt
timber
time
tiny
tip
tired
tissue
title
toast
tobacco
today
toddler
toe
together
toilet
token
tomato
tomorrow
tone
tongue
tonight
tool
tooth
top
topic
topple
torch
tornado
tortoise
toss
total
tourist
toward
tower
town
toy
track
trade
traffic
tragic
train
transfer
trap
trash
travel
tray
treat
tree
trend
trial
tribe
trick
trigger
trim
trip
trophy
trouble
truck
true
truly
trumpet
trust
truth
try
tube
tuition
tumble
tuna
tunnel
turkey
turn
turtle
twelve
twenty
twice
twin
twist
two
type
typical
ugly
umbrella
unable
unaware
uncle
uncover
under
undo
unfair
unfold
unhappy
uniform
unique
unit
universe
unknown
unlock
until
unusual
unveil
update
upgrade
uphold
upon
upper
upset
urban
urge
usage
use
used
useful
useless
usual
utility
vacant
vacuum
vague
valid
valley
valve
van
vanish
vapor
various
vast
vault
vehicle
velvet
vendor
venture
venue
verb
verify
version
very
vessel
veteran
viable
vibrant
vicious
victory
video
view
village
vintage
violin
virtual
virus
visa
visit
visual
vital
vivid
vocal
voice
void
volcano
volume
vote
voyage
wage
wagon
wait
walk
wall
walnut
want
warfare
warm
warrior
wash
wasp
waste
water
wave
way
wealth
weapon
wear
weasel
weather
web
wedding
weekend
weird
welcome
west
wet
whale
what
wheat
wheel
when
where
whip
whisper
wide
width
wife
wild
will
win
window
wine
wing
wink
winner
winter
wire
wisdom
wise
wish
witness
wolf
woman
wonder
wood
wool
word
work
world
worry
worth
wrap
wreck
wrestle
wrist
write
wrong
yard
year
yellow
you
young
youth
zebra
zero
zone
zoo`.split(`
`),
  );
  function th(t) {
    return (
      t instanceof Uint8Array ||
      (ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array")
    );
  }
  function Ps(t) {
    if (typeof t != "boolean") throw new Error(`boolean expected, not ${t}`);
  }
  function Vs(t) {
    if (!Number.isSafeInteger(t) || t < 0)
      throw new Error("positive integer expected, got " + t);
  }
  function ye(t, e, a = "") {
    let n = th(t),
      s = t?.length,
      o = e !== void 0;
    if (!n || (o && s !== e)) {
      let r = a && `"${a}" `,
        i = o ? ` of length ${e}` : "",
        c = n ? `length=${s}` : `type=${typeof t}`;
      throw new Error(r + "expected Uint8Array" + i + ", got " + c);
    }
    return t;
  }
  function Kr(t, e = !0) {
    if (t.destroyed) throw new Error("Hash instance has been destroyed");
    if (e && t.finished)
      throw new Error("Hash#digest() has already been called");
  }
  function sd(t, e) {
    ye(t, void 0, "output");
    let a = e.outputLen;
    if (t.length < a)
      throw new Error(
        "digestInto() expects output buffer of length at least " + a,
      );
  }
  function gt(t) {
    return new Uint32Array(
      t.buffer,
      t.byteOffset,
      Math.floor(t.byteLength / 4),
    );
  }
  function xt(...t) {
    for (let e = 0; e < t.length; e++) t[e].fill(0);
  }
  function ah(t) {
    return new DataView(t.buffer, t.byteOffset, t.byteLength);
  }
  var nh = new Uint8Array(new Uint32Array([287454020]).buffer)[0] === 68;
  function od(t, e) {
    if (e == null || typeof e != "object")
      throw new Error("options must be defined");
    return Object.assign(t, e);
  }
  function zs(t, e) {
    if (t.length !== e.length) return !1;
    let a = 0;
    for (let n = 0; n < t.length; n++) a |= t[n] ^ e[n];
    return a === 0;
  }
  var Fr = (t, e) => {
    function a(n, ...s) {
      if ((ye(n, void 0, "key"), !nh))
        throw new Error("Non little-endian hardware is not yet supported");
      if (t.nonceLength !== void 0) {
        let u = s[0];
        ye(u, t.varSizeNonce ? void 0 : t.nonceLength, "nonce");
      }
      let o = t.tagLength;
      o && s[1] !== void 0 && ye(s[1], void 0, "AAD");
      let r = e(n, ...s),
        i = (u, d) => {
          if (d !== void 0) {
            if (u !== 2) throw new Error("cipher output not supported");
            ye(d, void 0, "output");
          }
        },
        c = !1;
      return {
        encrypt(u, d) {
          if (c)
            throw new Error("cannot encrypt() twice with same key + nonce");
          return (c = !0), ye(u), i(r.encrypt.length, d), r.encrypt(u, d);
        },
        decrypt(u, d) {
          if ((ye(u), o && u.length < o))
            throw new Error(
              '"ciphertext" expected length bigger than tagLength=' + o,
            );
          return i(r.decrypt.length, d), r.decrypt(u, d);
        },
      };
    }
    return Object.assign(a, t), a;
  };
  function Wr(t, e, a = !0) {
    if (e === void 0) return new Uint8Array(t);
    if (e.length !== t)
      throw new Error(
        '"output" expected Uint8Array of length ' + t + ", got: " + e.length,
      );
    if (a && !sh(e)) throw new Error("invalid output, must be aligned");
    return e;
  }
  function rd(t, e, a) {
    Ps(a);
    let n = new Uint8Array(16),
      s = ah(n);
    return s.setBigUint64(0, BigInt(e), a), s.setBigUint64(8, BigInt(t), a), n;
  }
  function sh(t) {
    return t.byteOffset % 4 === 0;
  }
  function za(t) {
    return Uint8Array.from(t);
  }
  var cd = (t) => Uint8Array.from(t.split(""), (e) => e.charCodeAt(0)),
    oh = cd("expand 16-byte k"),
    rh = cd("expand 32-byte k"),
    ih = gt(oh),
    ch = gt(rh);
  function Z(t, e) {
    return (t << e) | (t >>> (32 - e));
  }
  function Yr(t) {
    return t.byteOffset % 4 === 0;
  }
  var Gs = 64,
    lh = 16,
    ld = 2 ** 32 - 1,
    id = Uint32Array.of();
  function dh(t, e, a, n, s, o, r, i) {
    let c = s.length,
      l = new Uint8Array(Gs),
      u = gt(l),
      d = Yr(s) && Yr(o),
      f = d ? gt(s) : id,
      p = d ? gt(o) : id;
    for (let v = 0; v < c; r++) {
      if ((t(e, a, n, u, r, i), r >= ld))
        throw new Error("arx: counter overflow");
      let k = Math.min(Gs, c - v);
      if (d && k === Gs) {
        let L = v / 4;
        if (v % 4 !== 0) throw new Error("arx: invalid block position");
        for (let N = 0, V; N < lh; N++) (V = L + N), (p[V] = f[V] ^ u[N]);
        v += Gs;
        continue;
      }
      for (let L = 0, N; L < k; L++) (N = v + L), (o[N] = s[N] ^ l[L]);
      v += k;
    }
  }
  function Zr(t, e) {
    let {
      allowShortKeys: a,
      extendNonceFn: n,
      counterLength: s,
      counterRight: o,
      rounds: r,
    } = od(
      { allowShortKeys: !1, counterLength: 8, counterRight: !1, rounds: 20 },
      e,
    );
    if (typeof t != "function") throw new Error("core must be a function");
    return (
      Vs(s),
      Vs(r),
      Ps(o),
      Ps(a),
      (i, c, l, u, d = 0) => {
        ye(i, void 0, "key"), ye(c, void 0, "nonce"), ye(l, void 0, "data");
        let f = l.length;
        if (
          (u === void 0 && (u = new Uint8Array(f)),
          ye(u, void 0, "output"),
          Vs(d),
          d < 0 || d >= ld)
        )
          throw new Error("arx: counter overflow");
        if (u.length < f)
          throw new Error(
            `arx: output (${u.length}) is shorter than data (${f})`,
          );
        let p = [],
          v = i.length,
          k,
          L;
        if (v === 32) p.push((k = za(i))), (L = ch);
        else if (v === 16 && a)
          (k = new Uint8Array(32)), k.set(i), k.set(i, 16), (L = ih), p.push(k);
        else throw (ye(i, 32, "arx key"), new Error("invalid key size"));
        Yr(c) || p.push((c = za(c)));
        let N = gt(k);
        if (n) {
          if (c.length !== 24)
            throw new Error("arx: extended nonce must be 24 bytes");
          n(L, N, gt(c.subarray(0, 16)), N), (c = c.subarray(16));
        }
        let V = 16 - s;
        if (V !== c.length)
          throw new Error(`arx: nonce must be ${V} or 16 bytes`);
        if (V !== 12) {
          let q = new Uint8Array(12);
          q.set(c, o ? 0 : 12 - c.length), (c = q), p.push(c);
        }
        let O = gt(c);
        return dh(t, L, N, O, l, u, d, r), xt(...p), u;
      }
    );
  }
  function $e(t, e) {
    return (t[e++] & 255) | ((t[e++] & 255) << 8);
  }
  var Jr = class {
    blockLen = 16;
    outputLen = 16;
    buffer = new Uint8Array(16);
    r = new Uint16Array(10);
    h = new Uint16Array(10);
    pad = new Uint16Array(8);
    pos = 0;
    finished = !1;
    constructor(e) {
      e = za(ye(e, 32, "key"));
      let a = $e(e, 0),
        n = $e(e, 2),
        s = $e(e, 4),
        o = $e(e, 6),
        r = $e(e, 8),
        i = $e(e, 10),
        c = $e(e, 12),
        l = $e(e, 14);
      (this.r[0] = a & 8191),
        (this.r[1] = ((a >>> 13) | (n << 3)) & 8191),
        (this.r[2] = ((n >>> 10) | (s << 6)) & 7939),
        (this.r[3] = ((s >>> 7) | (o << 9)) & 8191),
        (this.r[4] = ((o >>> 4) | (r << 12)) & 255),
        (this.r[5] = (r >>> 1) & 8190),
        (this.r[6] = ((r >>> 14) | (i << 2)) & 8191),
        (this.r[7] = ((i >>> 11) | (c << 5)) & 8065),
        (this.r[8] = ((c >>> 8) | (l << 8)) & 8191),
        (this.r[9] = (l >>> 5) & 127);
      for (let u = 0; u < 8; u++) this.pad[u] = $e(e, 16 + 2 * u);
    }
    process(e, a, n = !1) {
      let s = n ? 0 : 2048,
        { h: o, r } = this,
        i = r[0],
        c = r[1],
        l = r[2],
        u = r[3],
        d = r[4],
        f = r[5],
        p = r[6],
        v = r[7],
        k = r[8],
        L = r[9],
        N = $e(e, a + 0),
        V = $e(e, a + 2),
        O = $e(e, a + 4),
        q = $e(e, a + 6),
        K = $e(e, a + 8),
        T = $e(e, a + 10),
        g = $e(e, a + 12),
        w = $e(e, a + 14),
        m = o[0] + (N & 8191),
        $ = o[1] + (((N >>> 13) | (V << 3)) & 8191),
        A = o[2] + (((V >>> 10) | (O << 6)) & 8191),
        R = o[3] + (((O >>> 7) | (q << 9)) & 8191),
        E = o[4] + (((q >>> 4) | (K << 12)) & 8191),
        B = o[5] + ((K >>> 1) & 8191),
        S = o[6] + (((K >>> 14) | (T << 2)) & 8191),
        D = o[7] + (((T >>> 11) | (g << 5)) & 8191),
        I = o[8] + (((g >>> 8) | (w << 8)) & 8191),
        j = o[9] + ((w >>> 5) | s),
        U = 0,
        G = U + m * i + $ * (5 * L) + A * (5 * k) + R * (5 * v) + E * (5 * p);
      (U = G >>> 13),
        (G &= 8191),
        (G +=
          B * (5 * f) + S * (5 * d) + D * (5 * u) + I * (5 * l) + j * (5 * c)),
        (U += G >>> 13),
        (G &= 8191);
      let W = U + m * c + $ * i + A * (5 * L) + R * (5 * k) + E * (5 * v);
      (U = W >>> 13),
        (W &= 8191),
        (W +=
          B * (5 * p) + S * (5 * f) + D * (5 * d) + I * (5 * u) + j * (5 * l)),
        (U += W >>> 13),
        (W &= 8191);
      let Y = U + m * l + $ * c + A * i + R * (5 * L) + E * (5 * k);
      (U = Y >>> 13),
        (Y &= 8191),
        (Y +=
          B * (5 * v) + S * (5 * p) + D * (5 * f) + I * (5 * d) + j * (5 * u)),
        (U += Y >>> 13),
        (Y &= 8191);
      let te = U + m * u + $ * l + A * c + R * i + E * (5 * L);
      (U = te >>> 13),
        (te &= 8191),
        (te +=
          B * (5 * k) + S * (5 * v) + D * (5 * p) + I * (5 * f) + j * (5 * d)),
        (U += te >>> 13),
        (te &= 8191);
      let re = U + m * d + $ * u + A * l + R * c + E * i;
      (U = re >>> 13),
        (re &= 8191),
        (re +=
          B * (5 * L) + S * (5 * k) + D * (5 * v) + I * (5 * p) + j * (5 * f)),
        (U += re >>> 13),
        (re &= 8191);
      let ae = U + m * f + $ * d + A * u + R * l + E * c;
      (U = ae >>> 13),
        (ae &= 8191),
        (ae += B * i + S * (5 * L) + D * (5 * k) + I * (5 * v) + j * (5 * p)),
        (U += ae >>> 13),
        (ae &= 8191);
      let de = U + m * p + $ * f + A * d + R * u + E * l;
      (U = de >>> 13),
        (de &= 8191),
        (de += B * c + S * i + D * (5 * L) + I * (5 * k) + j * (5 * v)),
        (U += de >>> 13),
        (de &= 8191);
      let he = U + m * v + $ * p + A * f + R * d + E * u;
      (U = he >>> 13),
        (he &= 8191),
        (he += B * l + S * c + D * i + I * (5 * L) + j * (5 * k)),
        (U += he >>> 13),
        (he &= 8191);
      let we = U + m * k + $ * v + A * p + R * f + E * d;
      (U = we >>> 13),
        (we &= 8191),
        (we += B * u + S * l + D * c + I * i + j * (5 * L)),
        (U += we >>> 13),
        (we &= 8191);
      let ue = U + m * L + $ * k + A * v + R * p + E * f;
      (U = ue >>> 13),
        (ue &= 8191),
        (ue += B * d + S * u + D * l + I * c + j * i),
        (U += ue >>> 13),
        (ue &= 8191),
        (U = ((U << 2) + U) | 0),
        (U = (U + G) | 0),
        (G = U & 8191),
        (U = U >>> 13),
        (W += U),
        (o[0] = G),
        (o[1] = W),
        (o[2] = Y),
        (o[3] = te),
        (o[4] = re),
        (o[5] = ae),
        (o[6] = de),
        (o[7] = he),
        (o[8] = we),
        (o[9] = ue);
    }
    finalize() {
      let { h: e, pad: a } = this,
        n = new Uint16Array(10),
        s = e[1] >>> 13;
      e[1] &= 8191;
      for (let i = 2; i < 10; i++)
        (e[i] += s), (s = e[i] >>> 13), (e[i] &= 8191);
      (e[0] += s * 5),
        (s = e[0] >>> 13),
        (e[0] &= 8191),
        (e[1] += s),
        (s = e[1] >>> 13),
        (e[1] &= 8191),
        (e[2] += s),
        (n[0] = e[0] + 5),
        (s = n[0] >>> 13),
        (n[0] &= 8191);
      for (let i = 1; i < 10; i++)
        (n[i] = e[i] + s), (s = n[i] >>> 13), (n[i] &= 8191);
      n[9] -= 8192;
      let o = (s ^ 1) - 1;
      for (let i = 0; i < 10; i++) n[i] &= o;
      o = ~o;
      for (let i = 0; i < 10; i++) e[i] = (e[i] & o) | n[i];
      (e[0] = (e[0] | (e[1] << 13)) & 65535),
        (e[1] = ((e[1] >>> 3) | (e[2] << 10)) & 65535),
        (e[2] = ((e[2] >>> 6) | (e[3] << 7)) & 65535),
        (e[3] = ((e[3] >>> 9) | (e[4] << 4)) & 65535),
        (e[4] = ((e[4] >>> 12) | (e[5] << 1) | (e[6] << 14)) & 65535),
        (e[5] = ((e[6] >>> 2) | (e[7] << 11)) & 65535),
        (e[6] = ((e[7] >>> 5) | (e[8] << 8)) & 65535),
        (e[7] = ((e[8] >>> 8) | (e[9] << 5)) & 65535);
      let r = e[0] + a[0];
      e[0] = r & 65535;
      for (let i = 1; i < 8; i++)
        (r = (((e[i] + a[i]) | 0) + (r >>> 16)) | 0), (e[i] = r & 65535);
      xt(n);
    }
    update(e) {
      Kr(this), ye(e), (e = za(e));
      let { buffer: a, blockLen: n } = this,
        s = e.length;
      for (let o = 0; o < s; ) {
        let r = Math.min(n - this.pos, s - o);
        if (r === n) {
          for (; n <= s - o; o += n) this.process(e, o);
          continue;
        }
        a.set(e.subarray(o, o + r), this.pos),
          (this.pos += r),
          (o += r),
          this.pos === n && (this.process(a, 0, !1), (this.pos = 0));
      }
      return this;
    }
    destroy() {
      xt(this.h, this.r, this.buffer, this.pad);
    }
    digestInto(e) {
      Kr(this), sd(e, this), (this.finished = !0);
      let { buffer: a, h: n } = this,
        { pos: s } = this;
      if (s) {
        for (a[s++] = 1; s < 16; s++) a[s] = 0;
        this.process(a, 0, !0);
      }
      this.finalize();
      let o = 0;
      for (let r = 0; r < 8; r++) (e[o++] = n[r] >>> 0), (e[o++] = n[r] >>> 8);
      return e;
    }
    digest() {
      let { buffer: e, outputLen: a } = this;
      this.digestInto(e);
      let n = e.slice(0, a);
      return this.destroy(), n;
    }
  };
  function uh(t) {
    let e = (n, s) => t(s).update(n).digest(),
      a = t(new Uint8Array(32));
    return (
      (e.outputLen = a.outputLen),
      (e.blockLen = a.blockLen),
      (e.create = (n) => t(n)),
      e
    );
  }
  var dd = uh((t) => new Jr(t));
  function pd(t, e, a, n, s, o = 20) {
    let r = t[0],
      i = t[1],
      c = t[2],
      l = t[3],
      u = e[0],
      d = e[1],
      f = e[2],
      p = e[3],
      v = e[4],
      k = e[5],
      L = e[6],
      N = e[7],
      V = s,
      O = a[0],
      q = a[1],
      K = a[2],
      T = r,
      g = i,
      w = c,
      m = l,
      $ = u,
      A = d,
      R = f,
      E = p,
      B = v,
      S = k,
      D = L,
      I = N,
      j = V,
      U = O,
      G = q,
      W = K;
    for (let te = 0; te < o; te += 2)
      (T = (T + $) | 0),
        (j = Z(j ^ T, 16)),
        (B = (B + j) | 0),
        ($ = Z($ ^ B, 12)),
        (T = (T + $) | 0),
        (j = Z(j ^ T, 8)),
        (B = (B + j) | 0),
        ($ = Z($ ^ B, 7)),
        (g = (g + A) | 0),
        (U = Z(U ^ g, 16)),
        (S = (S + U) | 0),
        (A = Z(A ^ S, 12)),
        (g = (g + A) | 0),
        (U = Z(U ^ g, 8)),
        (S = (S + U) | 0),
        (A = Z(A ^ S, 7)),
        (w = (w + R) | 0),
        (G = Z(G ^ w, 16)),
        (D = (D + G) | 0),
        (R = Z(R ^ D, 12)),
        (w = (w + R) | 0),
        (G = Z(G ^ w, 8)),
        (D = (D + G) | 0),
        (R = Z(R ^ D, 7)),
        (m = (m + E) | 0),
        (W = Z(W ^ m, 16)),
        (I = (I + W) | 0),
        (E = Z(E ^ I, 12)),
        (m = (m + E) | 0),
        (W = Z(W ^ m, 8)),
        (I = (I + W) | 0),
        (E = Z(E ^ I, 7)),
        (T = (T + A) | 0),
        (W = Z(W ^ T, 16)),
        (D = (D + W) | 0),
        (A = Z(A ^ D, 12)),
        (T = (T + A) | 0),
        (W = Z(W ^ T, 8)),
        (D = (D + W) | 0),
        (A = Z(A ^ D, 7)),
        (g = (g + R) | 0),
        (j = Z(j ^ g, 16)),
        (I = (I + j) | 0),
        (R = Z(R ^ I, 12)),
        (g = (g + R) | 0),
        (j = Z(j ^ g, 8)),
        (I = (I + j) | 0),
        (R = Z(R ^ I, 7)),
        (w = (w + E) | 0),
        (U = Z(U ^ w, 16)),
        (B = (B + U) | 0),
        (E = Z(E ^ B, 12)),
        (w = (w + E) | 0),
        (U = Z(U ^ w, 8)),
        (B = (B + U) | 0),
        (E = Z(E ^ B, 7)),
        (m = (m + $) | 0),
        (G = Z(G ^ m, 16)),
        (S = (S + G) | 0),
        ($ = Z($ ^ S, 12)),
        (m = (m + $) | 0),
        (G = Z(G ^ m, 8)),
        (S = (S + G) | 0),
        ($ = Z($ ^ S, 7));
    let Y = 0;
    (n[Y++] = (r + T) | 0),
      (n[Y++] = (i + g) | 0),
      (n[Y++] = (c + w) | 0),
      (n[Y++] = (l + m) | 0),
      (n[Y++] = (u + $) | 0),
      (n[Y++] = (d + A) | 0),
      (n[Y++] = (f + R) | 0),
      (n[Y++] = (p + E) | 0),
      (n[Y++] = (v + B) | 0),
      (n[Y++] = (k + S) | 0),
      (n[Y++] = (L + D) | 0),
      (n[Y++] = (N + I) | 0),
      (n[Y++] = (V + j) | 0),
      (n[Y++] = (O + U) | 0),
      (n[Y++] = (q + G) | 0),
      (n[Y++] = (K + W) | 0);
  }
  function fh(t, e, a, n) {
    let s = t[0],
      o = t[1],
      r = t[2],
      i = t[3],
      c = e[0],
      l = e[1],
      u = e[2],
      d = e[3],
      f = e[4],
      p = e[5],
      v = e[6],
      k = e[7],
      L = a[0],
      N = a[1],
      V = a[2],
      O = a[3];
    for (let K = 0; K < 20; K += 2)
      (s = (s + c) | 0),
        (L = Z(L ^ s, 16)),
        (f = (f + L) | 0),
        (c = Z(c ^ f, 12)),
        (s = (s + c) | 0),
        (L = Z(L ^ s, 8)),
        (f = (f + L) | 0),
        (c = Z(c ^ f, 7)),
        (o = (o + l) | 0),
        (N = Z(N ^ o, 16)),
        (p = (p + N) | 0),
        (l = Z(l ^ p, 12)),
        (o = (o + l) | 0),
        (N = Z(N ^ o, 8)),
        (p = (p + N) | 0),
        (l = Z(l ^ p, 7)),
        (r = (r + u) | 0),
        (V = Z(V ^ r, 16)),
        (v = (v + V) | 0),
        (u = Z(u ^ v, 12)),
        (r = (r + u) | 0),
        (V = Z(V ^ r, 8)),
        (v = (v + V) | 0),
        (u = Z(u ^ v, 7)),
        (i = (i + d) | 0),
        (O = Z(O ^ i, 16)),
        (k = (k + O) | 0),
        (d = Z(d ^ k, 12)),
        (i = (i + d) | 0),
        (O = Z(O ^ i, 8)),
        (k = (k + O) | 0),
        (d = Z(d ^ k, 7)),
        (s = (s + l) | 0),
        (O = Z(O ^ s, 16)),
        (v = (v + O) | 0),
        (l = Z(l ^ v, 12)),
        (s = (s + l) | 0),
        (O = Z(O ^ s, 8)),
        (v = (v + O) | 0),
        (l = Z(l ^ v, 7)),
        (o = (o + u) | 0),
        (L = Z(L ^ o, 16)),
        (k = (k + L) | 0),
        (u = Z(u ^ k, 12)),
        (o = (o + u) | 0),
        (L = Z(L ^ o, 8)),
        (k = (k + L) | 0),
        (u = Z(u ^ k, 7)),
        (r = (r + d) | 0),
        (N = Z(N ^ r, 16)),
        (f = (f + N) | 0),
        (d = Z(d ^ f, 12)),
        (r = (r + d) | 0),
        (N = Z(N ^ r, 8)),
        (f = (f + N) | 0),
        (d = Z(d ^ f, 7)),
        (i = (i + c) | 0),
        (V = Z(V ^ i, 16)),
        (p = (p + V) | 0),
        (c = Z(c ^ p, 12)),
        (i = (i + c) | 0),
        (V = Z(V ^ i, 8)),
        (p = (p + V) | 0),
        (c = Z(c ^ p, 7));
    let q = 0;
    (n[q++] = s),
      (n[q++] = o),
      (n[q++] = r),
      (n[q++] = i),
      (n[q++] = L),
      (n[q++] = N),
      (n[q++] = V),
      (n[q++] = O);
  }
  var Ks = Zr(pd, { counterRight: !1, counterLength: 4, allowShortKeys: !1 }),
    ph = Zr(pd, {
      counterRight: !1,
      counterLength: 8,
      extendNonceFn: fh,
      allowShortKeys: !1,
    });
  var hh = new Uint8Array(16),
    ud = (t, e) => {
      t.update(e);
      let a = e.length % 16;
      a && t.update(hh.subarray(a));
    },
    mh = new Uint8Array(32);
  function fd(t, e, a, n, s) {
    s !== void 0 && ye(s, void 0, "AAD");
    let o = t(e, a, mh),
      r = rd(n.length, s ? s.length : 0, !0),
      i = dd.create(o);
    s && ud(i, s), ud(i, n), i.update(r);
    let c = i.digest();
    return xt(o, r), c;
  }
  var hd = (t) => (e, a, n) => ({
      encrypt(o, r) {
        let i = o.length;
        (r = Wr(i + 16, r, !1)), r.set(o);
        let c = r.subarray(0, -16);
        t(e, a, c, c, 1);
        let l = fd(t, e, a, c, n);
        return r.set(l, i), xt(l), r;
      },
      decrypt(o, r) {
        r = Wr(o.length - 16, r, !1);
        let i = o.subarray(0, -16),
          c = o.subarray(-16),
          l = fd(t, e, a, i, n);
        if (!zs(c, l)) throw new Error("invalid tag");
        return r.set(o.subarray(0, -16)), t(e, a, r, r, 1), xt(l), r;
      },
    }),
    lg = Fr({ blockSize: 64, nonceLength: 12, tagLength: 16 }, hd(Ks)),
    dg = Fr({ blockSize: 64, nonceLength: 24, tagLength: 16 }, hd(ph));
  function bd(t, e, a) {
    return (
      ra(t), a === void 0 && (a = new Uint8Array(t.outputLen)), Rt(t, a, e)
    );
  }
  var Xr = Uint8Array.of(0),
    md = Uint8Array.of();
  function gd(t, e, a, n = 32) {
    ra(t), Lt(n, "length");
    let s = t.outputLen;
    if (n > 255 * s) throw new Error("Length must be <= 255*HashLen");
    let o = Math.ceil(n / s);
    a === void 0 ? (a = md) : Ze(a, void 0, "info");
    let r = new Uint8Array(o * s),
      i = Rt.create(t, e),
      c = i._cloneInto(),
      l = new Uint8Array(i.outputLen);
    for (let u = 0; u < o; u++)
      (Xr[0] = u + 1),
        c
          .update(u === 0 ? md : l)
          .update(a)
          .update(Xr)
          .digestInto(l),
        r.set(l, s * u),
        i._cloneInto(c);
    return i.destroy(), c.destroy(), _e(l, Xr), r.slice(0, n);
  }
  var bh = new TextDecoder("utf-8"),
    xd = new TextEncoder(),
    Qr = 1,
    ei = 4294967295,
    ti = 65536;
  function ai(t, e) {
    let a = ft.getSharedSecret(t, ht("02" + e)).subarray(1, 33);
    return bd(la, a, xd.encode("nip44-v2"));
  }
  function yd(t, e) {
    let a = gd(la, t, e, 76);
    return {
      chacha_key: a.subarray(0, 32),
      chacha_nonce: a.subarray(32, 44),
      hmac_key: a.subarray(44, 76),
    };
  }
  function vd(t) {
    if (!Number.isSafeInteger(t) || t < 1)
      throw new Error("expected positive integer");
    if (t <= 32) return 32;
    let e = 2 ** (Math.floor(Math.log2(t - 1)) + 1),
      a = e <= 256 ? 32 : e / 8;
    return a * (Math.floor((t - 1) / a) + 1);
  }
  function gh(t) {
    if (!Number.isSafeInteger(t) || t < Qr || t > 65535)
      throw new Error(
        "invalid plaintext size: must be between 1 and 65535 bytes",
      );
    let e = new Uint8Array(2);
    return new DataView(e.buffer).setUint16(0, t, !1), e;
  }
  function xh(t) {
    if (!Number.isSafeInteger(t) || t < ti || t > ei)
      throw new Error(
        "invalid plaintext size: must be between 65536 and 4294967295 bytes",
      );
    let e = new Uint8Array(4);
    return new DataView(e.buffer).setUint32(0, t, !1), e;
  }
  function yh(t) {
    let e = xd.encode(t),
      a = e.length;
    if (a < Qr || a > ei)
      throw new Error(
        "invalid plaintext size: must be between 1 and 4294967295 bytes",
      );
    let n = a >= ti ? Ua(new Uint8Array([0, 0]), xh(a)) : gh(a),
      s = new Uint8Array(vd(a) - a);
    return Ua(n, e, s);
  }
  function vh(t) {
    let e = new DataView(t.buffer, t.byteOffset, t.byteLength),
      a = e.getUint16(0),
      n,
      s;
    if (a === 0) {
      if (((n = e.getUint32(2)), n < ti)) throw new Error("invalid padding");
      s = 6;
    } else (n = a), (s = 2);
    let o = t.subarray(s, s + n);
    if (n < Qr || n > ei || o.length !== n || t.length !== s + vd(n))
      throw new Error("invalid padding");
    return bh.decode(o);
  }
  function wd(t, e, a) {
    if (a.length !== 32)
      throw new Error("AAD associated data must be 32 bytes");
    let n = Ua(a, e);
    return Rt(la, t, n);
  }
  function wh(t) {
    if (typeof t != "string") throw new Error("payload must be a valid string");
    let e = t.length;
    if (e < 132) throw new Error("invalid payload length: " + e);
    if (t[0] === "#") throw new Error("unknown encryption version");
    let a;
    try {
      a = ja.decode(t);
    } catch (o) {
      throw new Error("invalid base64: " + o.message);
    }
    let n = a.length;
    if (n < 99) throw new Error("invalid data length: " + n);
    let s = a[0];
    if (s !== 2) throw new Error("unknown encryption version " + s);
    return {
      nonce: a.subarray(1, 33),
      ciphertext: a.subarray(33, -32),
      mac: a.subarray(-32),
    };
  }
  function ni(t, e, a = Fc(32)) {
    let { chacha_key: n, chacha_nonce: s, hmac_key: o } = yd(e, a),
      r = yh(t),
      i = Ks(n, s, r),
      c = wd(o, i, a);
    return ja.encode(Ua(new Uint8Array([2]), a, i, c));
  }
  function si(t, e) {
    let { nonce: a, ciphertext: n, mac: s } = wh(t),
      { chacha_key: o, chacha_nonce: r, hmac_key: i } = yd(e, a),
      c = wd(i, n, a);
    if (!zs(c, s)) throw new Error("invalid MAC");
    let l = Ks(o, r, n);
    return vh(l);
  }
  var be = () => Math.floor(Date.now() / 1e3),
    ve = () => Date.now();
  function Ka(t = 16) {
    let e = new Uint8Array(t);
    return crypto.getRandomValues(e), Nt(e);
  }
  var Fa = { enc: (t) => ja.encode(t), dec: (t) => ja.decode(t) };
  function In(t) {
    return Nt(Va(t));
  }
  function kd(t, e) {
    let [a, n] = [t, e].sort(),
      s = Va(_t(a + n)),
      o = Va(s),
      r = new Uint8Array([...s, ...o]),
      i = [];
    for (let c = 0; c < 12; c++) {
      let l =
        ((r[c * 5] << 24) >>> 0) +
        (r[c * 5 + 1] << 16) +
        (r[c * 5 + 2] << 8) +
        r[c * 5 + 3];
      i.push(String(l % 1e5).padStart(5, "0"));
    }
    return i;
  }
  function ie(t, e) {
    return (
      (t = typeof t == "string" ? t : ""), t.length > e ? t.slice(0, e) : t
    );
  }
  function Wa(t, e) {
    let a = (t.tags || []).find((n) => n[0] === e);
    return a ? a[1] : void 0;
  }
  var Ga = class {
      constructor() {
        this._h = new Map();
      }
      on(e, a) {
        return (
          this._h.has(e) || this._h.set(e, new Set()),
          this._h.get(e).add(a),
          () => this._h.get(e).delete(a)
        );
      }
      emit(e, ...a) {
        let n = this._h.get(e);
        if (n)
          for (let s of [...n])
            try {
              s(...a);
            } catch (o) {
              console.error("[emit]", e, o);
            }
      }
    },
    Ed = (t) => new Promise((e) => setTimeout(e, t));
  var Ys = 14,
    Zs = 7,
    jt = 1400,
    Ad = 13,
    Hn = 1059,
    Ya = 21059,
    Eh = 48 * 3600,
    Fs = new Map();
  function Ws(t, e, a) {
    if (!a) return ai(t, e);
    let n = Ke(t) + e,
      s = Fs.get(n);
    return s || ((s = ai(t, e)), Fs.size > 500 && Fs.clear(), Fs.set(n, s)), s;
  }
  function $d() {
    return be() - Math.floor(Math.random() * Eh);
  }
  function Sd(t, e, a, n = [], s) {
    let o = {
      kind: e,
      content: a,
      tags: n,
      created_at: s || be(),
      pubkey: Ke(t),
    };
    return (o.id = Na(o)), o;
  }
  function oi(t, e, a, n = {}) {
    let s = !!n.ephemeral,
      o = As(
        {
          kind: Ad,
          content: ni(JSON.stringify(t), Ws(e, a, !0)),
          created_at: s ? be() : $d(),
          tags: [],
        },
        e,
      ),
      r = kn(),
      i = [["p", a]];
    return (
      n.expiration && i.push(["expiration", String(n.expiration)]),
      As(
        {
          kind: s ? Ya : Hn,
          content: ni(JSON.stringify(o), Ws(r, a, !1)),
          created_at: s ? be() : $d(),
          tags: i,
        },
        r,
      )
    );
  }
  function Cd(t, e) {
    if (t.kind !== Hn && t.kind !== Ya) throw new Error("kind");
    if (!mr(t)) throw new Error("wrap sig");
    let a = JSON.parse(si(t.content, Ws(e, t.pubkey, !1)));
    if (!a || a.kind !== Ad || !mr(a)) throw new Error("seal");
    let n = JSON.parse(si(a.content, Ws(e, a.pubkey, !0)));
    if (!n || n.pubkey !== a.pubkey) throw new Error("impersonation");
    let s = Na({
      kind: n.kind,
      content: n.content,
      tags: n.tags,
      created_at: n.created_at,
      pubkey: n.pubkey,
    });
    if (n.id !== s) throw new Error("rumor id");
    if (typeof n.content != "string" || !Array.isArray(n.tags))
      throw new Error("shape");
    return n;
  }
  function ri(t, e) {
    return As({ created_at: be(), tags: [], content: "", ...e }, t);
  }
  var $h = 1e4,
    ii = class {
      constructor(e, a) {
        (this.url = e),
          (this.pool = a),
          (this.ws = null),
          (this.status = "idle"),
          (this.queue = []),
          (this.pending = new Map()),
          (this.waitAuth = new Map()),
          (this.closedSubs = new Set()),
          (this.backoff = 1e3),
          (this.dead = !1),
          (this.authed = !1),
          (this.authId = null),
          (this.timer = null);
      }
      open() {
        if (
          this.dead ||
          (this.ws && (this.ws.readyState === 0 || this.ws.readyState === 1))
        )
          return;
        clearTimeout(this.timer), this.setStatus("connecting");
        let e;
        try {
          e = new this.pool.WS(this.url);
        } catch {
          this.scheduleReconnect();
          return;
        }
        (this.ws = e),
          (e.onopen = () => {
            if (e !== this.ws) return;
            (this.backoff = 1e3), (this.authed = !1), this.setStatus("open");
            let a = this.queue;
            this.queue = [];
            for (let n of a) this.rawSend(n);
            for (let [n, s] of this.pool.subs) this.rawSend(["REQ", n, ...s()]);
            this.pool.emit("open", this.url);
          }),
          (e.onmessage = (a) =>
            this.onMessage(
              typeof a.data == "string" ? a.data : String(a.data),
            )),
          (e.onclose = () => {
            e === this.ws &&
              ((this.ws = null),
              this.setStatus("closed"),
              this.scheduleReconnect());
          }),
          (e.onerror = () => {
            try {
              e.close();
            } catch {}
          });
      }
      scheduleReconnect() {
        if (this.dead) return;
        clearTimeout(this.timer);
        let e = this.backoff + Math.random() * 500;
        (this.backoff = Math.min(this.backoff * 2, 3e4)),
          (this.timer = setTimeout(() => this.open(), e));
      }
      kick() {
        this.dead ||
          ((!this.ws || this.ws.readyState > 1) &&
            ((this.backoff = 1e3), this.open()));
      }
      close() {
        if (((this.dead = !0), clearTimeout(this.timer), this.ws))
          try {
            this.ws.close();
          } catch {}
        (this.ws = null), this.setStatus("closed");
        for (let e of this.pending.values()) e(!1, "closed");
        this.pending.clear();
      }
      setStatus(e) {
        (this.status = e), this.pool.emit("status", this.url, e);
      }
      rawSend(e) {
        try {
          this.ws.send(JSON.stringify(e));
        } catch {
          this.queue.push(e);
        }
      }
      send(e) {
        this.ws && this.ws.readyState === 1
          ? this.rawSend(e)
          : (this.queue.push(e),
            this.queue.length > 400 && this.queue.shift(),
            this.open());
      }
      publish(e) {
        return new Promise((a) => {
          let n = setTimeout(() => {
            this.pending.delete(e.id), a({ ok: !1, reason: "timeout" });
          }, $h);
          this.pending.set(e.id, (s, o) => {
            clearTimeout(n), a({ ok: s, reason: o });
          }),
            this.send(["EVENT", e]);
        });
      }
      onMessage(e) {
        let a;
        try {
          a = JSON.parse(e);
        } catch {
          return;
        }
        if (!Array.isArray(a)) return;
        let n = a[0];
        if (n === "EVENT" && a[2] && typeof a[2] == "object")
          this.pool.emit("event", a[2], this.url, a[1]);
        else if (n === "OK") {
          let s = a[1],
            o = !!a[2],
            r = String(a[3] || "");
          if (s === this.authId) {
            if (o) {
              this.authed = !0;
              for (let c of this.closedSubs) {
                let l = this.pool.subs.get(c);
                l && this.rawSend(["REQ", c, ...l()]);
              }
              this.closedSubs.clear();
              for (let c of this.waitAuth.values()) this.rawSend(["EVENT", c]);
              this.waitAuth.clear();
            }
            return;
          }
          if (
            !o &&
            /^auth-required/i.test(r) &&
            this.pool.signAuth &&
            this.pending.get(s)
          ) {
            this.waitAuth.set(s, this.findQueued(s));
            return;
          }
          let i = this.pending.get(s);
          i && (this.pending.delete(s), i(o || /^duplicate/i.test(r), r));
        } else if (n === "EOSE") this.pool.emit("eose", this.url, a[1]);
        else if (n === "CLOSED")
          /^auth-required/i.test(String(a[2] || "")) &&
            this.closedSubs.add(a[1]);
        else if (n === "AUTH" && typeof a[1] == "string" && this.pool.signAuth)
          try {
            let s = this.pool.signAuth(this.url, a[1]);
            (this.authId = s.id), this.rawSend(["AUTH", s]);
          } catch {}
        else
          n === "NOTICE" &&
            this.pool.emit("notice", this.url, String(a[1] || ""));
      }
      findQueued(e) {
        return this.pool.lastEvents.get(e);
      }
    },
    Js = class extends Ga {
      constructor({ WebSocketImpl: e, signAuth: a } = {}) {
        super(),
          (this.WS = e || globalThis.WebSocket),
          (this.signAuth = a || null),
          (this.relays = new Map()),
          (this.urls = []),
          (this.subs = new Map()),
          (this.lastEvents = new Map()),
          (this.extra = new Set());
      }
      setRelays(e) {
        let a = [...new Set(e.map(ha).filter(Boolean))];
        for (let [n, s] of this.relays)
          !a.includes(n) &&
            !this.extra.has(n) &&
            (s.close(), this.relays.delete(n));
        this.urls = a;
        for (let n of a) this.ensure(n);
      }
      ensure(e) {
        if (((e = ha(e)), !e)) return null;
        let a = this.relays.get(e);
        return a || ((a = new ii(e, this)), this.relays.set(e, a), a.open()), a;
      }
      kickAll() {
        for (let e of this.relays.values()) e.kick();
      }
      closeAll() {
        for (let e of this.relays.values()) e.close();
        this.relays.clear();
      }
      subscribe(e, a) {
        this.subs.set(e, a);
        for (let n of this.urls) {
          let s = this.relays.get(n);
          s && s.send(["REQ", e, ...a()]);
        }
      }
      refresh(e) {
        let a = this.subs.get(e);
        if (a)
          for (let n of this.urls) {
            let s = this.relays.get(n);
            s && s.status === "open" && s.send(["REQ", e, ...a()]);
          }
      }
      unsubscribe(e) {
        this.subs.delete(e);
        for (let a of this.relays.values())
          a.status === "open" && a.send(["CLOSE", e]);
      }
      publish(e, a = []) {
        this.lastEvents.set(e.id, e),
          this.lastEvents.size > 300 &&
            this.lastEvents.delete(this.lastEvents.keys().next().value);
        let n = a.map(ha).filter(Boolean).slice(0, 4);
        for (let o of n) this.urls.includes(o) || this.extra.add(o);
        let s = [...new Set([...this.urls, ...n])];
        for (let o of s) this.ensure(o);
        return s.length
          ? new Promise((o) => {
              let r = s.length,
                i = 0,
                c = !1;
              for (let l of s)
                this.relays
                  .get(l)
                  .publish(e)
                  .then((u) => {
                    r--,
                      u.ok &&
                        (i++,
                        c || ((c = !0), o({ ok: !0, okCount: i, url: l }))),
                      r === 0 && !c && ((c = !0), o({ ok: !1, okCount: 0 }));
                  });
            })
          : Promise.resolve({ ok: !1, okCount: 0 });
      }
      status() {
        return this.urls.map((e) => ({
          url: e,
          status: (this.relays.get(e) || {}).status || "idle",
        }));
      }
      openCount() {
        return this.status().filter((e) => e.status === "open").length;
      }
    };
  function ha(t) {
    if (typeof t != "string" || ((t = t.trim()), !/^wss?:\/\//i.test(t)))
      return "";
    try {
      let e = new URL(t);
      return (e.protocol + "//" + e.host + e.pathname)
        .replace(/\/+$/, "")
        .toLowerCase();
    } catch {
      return "";
    }
  }
  var li = [
      "wss://relay.damus.io",
      "wss://nos.lol",
      "wss://relay.primal.net",
      "wss://relay.0xchat.com",
    ],
    Xs = 2e4,
    ci = 24e5,
    JM_FILE_MAX = 5e6,
    JM_FILE_MIME = /^[a-z0-9.+_-]{1,60}\/[a-z0-9.+_-]{1,60}$/i,
    Ld =
      /^(audio\/(webm|ogg|mp4|mpeg|aac)|image\/(webp|jpeg|png|gif))(;[a-z0-9=,. -]*)?$/i,
    Za = 86400,
    Qs = { pending: 0, failed: 0, sent: 1, delivered: 2, read: 3 },
    Re = /^[0-9a-f]{64}$/,
    Ah = /^[a-z0-9-]{1,24}$/,
    Sh = /^data:image\/(webp|jpeg|png);base64,[A-Za-z0-9+/]+={0,2}$/,
    Ch = 24e3;
  /* ---- Novidades 0.5 (iguais ao computador): banner, jogos, ouvindo agora, código curto ---- */
  var JM_BANNER_MAX = 24e3,
    JM_MAX_GAMES = 6,
    JM_SC_KIND = 30078,
    JM_SC_DIAS = 7,
    JM_SC_ALFABETO = "0123456789ABCDEFGHJKMNPQRSTVWXYZ",
    JM_LIS_HEARTBEAT = 4 * 6e4,
    JM_LIS_TTL = 9 * 6e4;
  function jmCleanBanner(t) {
    return t === ""
      ? ""
      : typeof t == "string" && t.length <= JM_BANNER_MAX && Sh.test(t)
        ? t
        : void 0;
  }
  function jmCleanGames(t) {
    let e = [];
    if (!Array.isArray(t)) return e;
    for (let a of t) {
      let n = String(a ?? "").replace(/[\u0000-\u001f\u007f]/g, "").trim().slice(0, 30);
      if (n && !e.some((s) => s.toLowerCase() === n.toLowerCase())) e.push(n);
      if (e.length >= JM_MAX_GAMES) break;
    }
    return e;
  }
  // Backup: HKDF-SHA256 da chave da conta + AES-256-GCM sobre o JSON em gzip.
  // Mesmo formato do PC ("JMBK1" + sal 16 + nonce 12 + cifrado).
  async function jmBackupKey(t, e) {
    let a = await crypto.subtle.importKey("raw", t, "HKDF", !1, ["deriveKey"]);
    return crypto.subtle.deriveKey({ name: "HKDF", hash: "SHA-256", salt: e, info: new TextEncoder().encode("jmenseger-backup-v1") }, a, { name: "AES-GCM", length: 256 }, !1, ["encrypt", "decrypt"]);
  }
  async function jmGz(t, e) {
    let a = new Blob([t]).stream().pipeThrough(e ? new CompressionStream("gzip") : new DecompressionStream("gzip"));
    return new Uint8Array(await new Response(a).arrayBuffer());
  }
  async function jmSeal(t, e) {
    let a = await jmGz(new TextEncoder().encode(e), !0),
      n = crypto.getRandomValues(new Uint8Array(16)),
      s = crypto.getRandomValues(new Uint8Array(12)),
      o = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv: s }, await jmBackupKey(t, n), a)),
      r = new Uint8Array(33 + o.length);
    return r.set(new TextEncoder().encode("JMBK1")), r.set(n, 5), r.set(s, 21), r.set(o, 33), r;
  }
  async function jmOpen(t, e) {
    if (e.length < 49 || new TextDecoder().decode(e.subarray(0, 5)) !== "JMBK1") throw new Error("Este arquivo n\xE3o \xE9 um backup do J-Menseger.");
    let a;
    try {
      a = await crypto.subtle.decrypt({ name: "AES-GCM", iv: e.subarray(21, 33) }, await jmBackupKey(t, e.subarray(5, 21)), e.subarray(33));
    } catch {
      throw new Error("Este backup \xE9 de outra conta (ou o arquivo est\xE1 corrompido).");
    }
    try {
      return new TextDecoder().decode(await jmGz(new Uint8Array(a), !1));
    } catch {
      throw new Error("Backup corrompido.");
    }
  }
  function jmFileName(t) {
    let e = String(t || "")
      .split(/[\\/]/)
      .pop()
      .replace(/[\u0000-\u001f<>:"|?*]/g, "")
      .slice(0, 120)
      .trim()
      .replace(/^\.+/, "");
    return e || "arquivo";
  }
  function jmCleanLp(t) {
    if (!t || typeof t != "object") return null;
    let e = String(t.u || "").trim().slice(0, 600),
      a = ie(t.t, 140);
    if (!/^https?:\/\//.test(e) || !a) return null;
    let n = { u: e, t: a };
    return (
      (t.d = ie(t.d, 240)) && (n.d = t.d),
      typeof t.i == "string" && t.i.length <= 4e4 && /^data:image\/(webp|jpeg|png);base64,/.test(t.i) && (n.i = t.i),
      n
    );
  }
  function jmCleanLpJson(t) {
    if (!t) return;
    try {
      return jmCleanLp(JSON.parse(t)) || void 0;
    } catch {
      return;
    }
  }
  function jmCleanStatus(t) {
    if (!t || typeof t != "object") return null;
    let e = String(t.e || "").replace(/[\u0000-\u001f]/g, "").slice(0, 8),
      a = ie(t.t, 80),
      n = !!t.dnd,
      s = Number(t.x) > 0 ? Number(t.x) : 0;
    if ((!e && !a && !n) || (s && s <= Date.now())) return null;
    let o = {};
    return e && (o.e = e), a && (o.t = a), n && (o.dnd = !0), s && (o.x = s), o;
  }
  var JM_PRES_HEARTBEAT = 24e4, JM_PRES_TTL = 54e4;
  /** Cargos do grupo: nome e cor, e quem tem cada um. Igual ao PC. */
  var JM_PERMS = [["pin", "Fixar mensagens"], ["del", "Apagar mensagens de outros"], ["todos", "Mencionar @todos"]];
  /** Admin, ou cargo com esse poder. */
  function jmCan(t, e, a) {
    if (t.owner === e || (t.admins || []).includes(e)) return !0;
    let n = (t.mroles || {})[e] || [];
    return (t.roles || []).some((s) => n.includes(s.id) && (s.p || []).includes(a));
  }
  function jmCleanRoles(t, e) {
    let a = (Array.isArray(t.roles) ? t.roles : [])
        .filter((s) => s && /^[a-z0-9]{1,12}$/.test(s.id))
        .slice(0, 12)
        .map((s) => {
          let o = { id: s.id, n: ie(s.n, 24) || "Cargo", c: Number.isInteger(s.c) ? ((s.c % 8) + 8) % 8 : 0 },
            r = [...new Set((Array.isArray(s.p) ? s.p : []).filter((i) => JM_PERMS.some(([c]) => c === i)))];
          return r.length && (o.p = r), o;
        }),
      n = {};
    if (t.mroles && typeof t.mroles == "object")
      for (let [s, o] of Object.entries(t.mroles))
        if (Re.test(s) && e.includes(s) && Array.isArray(o)) {
          let r = o.filter((i) => a.some((c) => c.id === i)).slice(0, 4);
          r.length && (n[s] = r);
        }
    return a.length ? { roles: a, mroles: n } : {};
  }
    function jmCleanListening(t) {
    if (!t || typeof t != "object") return null;
    let e = (a, n) => String(a ?? "").replace(/[\u0000-\u001f\u007f]/g, "").slice(0, n).trim(),
      a = e(t.s, 120);
    if (!a) return null;
    let n = typeof t.id == "string" && /^[A-Za-z0-9]{22}$/.test(t.id) ? t.id : void 0,
      s = Number.isInteger(t.d) && t.d > 0 && t.d <= 10800 ? t.d : void 0;
    return { s: a, a: e(t.a, 120), al: e(t.al, 120), id: n, st: Math.max(0, Number(t.st) || 0), d: s };
  }
  function jmListeningAlive(t, e, a) {
    return a - e > JM_LIS_TTL ? !1 : t.d && t.st > 0 ? a < t.st + t.d * 1e3 + 9e4 : !0;
  }
  function jmListeningUrl(t) {
    return t.id
      ? "https://open.spotify.com/track/" + t.id
      : "https://open.spotify.com/search/" + encodeURIComponent(t.s + " " + t.a);
  }
  function jmShortGenerate() {
    let t = crypto.getRandomValues(new Uint8Array(8)),
      e = [...t].map((a) => JM_SC_ALFABETO[a & 31]).join("");
    return e.slice(0, 4) + "-" + e.slice(4);
  }
  function jmShortParse(t) {
    let e = "";
    for (let a of String(t || "").trim().toUpperCase()) {
      let n = a === "O" ? "0" : a === "I" || a === "L" ? "1" : a;
      if (n === "-" || n === " ") continue;
      if (!JM_SC_ALFABETO.includes(n)) return null;
      e += n;
    }
    e.startsWith("JM") && e.length === 10 && (e = e.slice(2));
    return e.length === 8 ? e.slice(0, 4) + "-" + e.slice(4) : null;
  }
  function ma(t) {
    return t === ""
      ? ""
      : typeof t == "string" && t.length <= Ch && Sh.test(t)
        ? t
        : void 0;
  }
  var Td = {
      relays: li,
      readReceipts: !0,
      typing: !0,
      notifPreview: "name",
      callsFrom: "contacts",
      ttlDefault: 0,
      turn: { url: "", user: "", pass: "" },
      forceRelay: !1,
      background: !0,
      sounds: !0,
      vibrate: !0,
      lockScreenshots: !1,
      notifications: !0,
      presence: !0,
      linkPreviews: !0,
    },
    Bd = {
      amigos: { text: ["geral", "fotos"], voice: ["Sala de estar"] },
      estudos: {
        text: ["avisos", "d\xFAvidas", "materiais"],
        voice: ["Estudo em grupo"],
      },
      jogos: { text: ["geral", "clipes"], voice: ["Jogatina", "Sala 2"] },
      branco: { text: ["geral"], voice: ["Voz"] },
    };
  function Md(t) {
    return (
      (t || "")
        .toLowerCase()
        .normalize("NFD")
        .replace(/[̀-ͯ]/g, "")
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "")
        .slice(0, 20) || "canal"
    );
  }
  function to(t) {
    if (!t) return null;
    let e = String(t).trim(),
      a = e.match(/(npub1[02-9ac-hj-np-z]{58}|nprofile1[02-9ac-hj-np-z]+)/i);
    if ((a && (e = a[1]), Re.test(e.toLowerCase()))) return e.toLowerCase();
    try {
      let n = kr(e.toLowerCase());
      if (n.type === "npub") return n.data;
      if (n.type === "nprofile") return n.data.pubkey;
    } catch {}
    return null;
  }
  var ao = (t) => An(t),
    Rd = (t) => {
      let e = An(t);
      return e.slice(0, 10) + "\u2026" + e.slice(-4);
    },
    eo = class t extends Ga {
      constructor({ db: e, WebSocketImpl: a, secure: n }) {
        super(),
          (this.db = e),
          (this.WS = a),
          (this.secure = n || {
            get: async (s) => ((await e.get("kv", "sec:" + s)) || {}).v,
            set: async (s, o) => e.put("kv", { k: "sec:" + s, v: o }),
            del: async (s) => e.del("kv", "sec:" + s),
          }),
          (this.sk = null),
          (this.pk = null),
          (this.me = null),
          (this.contacts = new Map()),
          (this.groups = new Map()),
          (this.chats = new Map()),
          (this.settings = { ...Td }),
          (this.seen = new Set()),
          (this.seenList = []),
          (this.lastSync = 0),
          (this.active = { chat: null, visible: !0 }),
          (this.q = Promise.resolve()),
          (this.rq = new Map()),
          (this.typingAt = new Map()),
          (this.rooms = new Map()),
          (this.profReqAt = new Map()),
          (this.timers = []),
          (this.pool = null);
      }
      async kv(e, a) {
        let n = await this.db.get("kv", e);
        return n ? n.v : a;
      }
      async setKv(e, a) {
        return e === "lock" && (this.hasPin = !!a), this.db.put("kv", { k: e, v: a });
      }
      async load() {
        (this.hasPin = !!(await this.kv("lock", null))),
        (this.me = await this.kv("me", null)),
          (this.settings = { ...Td, ...(await this.kv("settings", {})) });
        let e = await this.secure.get("sk");
        e && Re.test(e) && ((this.sk = _t(e)), (this.pk = Ke(this.sk)));
        for (let a of await this.db.all("contacts")) this.contacts.set(a.pk, a);
        for (let a of await this.db.all("groups")) this.groups.set(a.lid, a);
        for (let a of await this.db.all("chats")) this.chats.set(a.key, a);
        return (
          (this.seenList = await this.kv("seen", [])),
          (this.seen = new Set(this.seenList)),
          (this.lastSync = await this.kv("lastSync", 0)),
          !!(this.sk && this.me)
        );
      }
      newWords() {
        return ed(Gr, 128);
      }
      static normWords(e) {
        return String(e || "")
          .toLowerCase()
          .normalize("NFKD")
          .replace(/[\u0300-\u036f]/g, "")
          .replace(/[^a-z\s]/g, " ")
          .trim()
          .split(/\s+/)
          .filter(Boolean)
          .join(" ");
      }
      validWords(e) {
        let a = t.normWords(e),
          n = a.split(" ").length;
        return n !== 12 && n !== 24 ? null : zr(a, Gr) || zr(a, nd) ? a : null;
      }
      skFromInput(e) {
        let a = String(e || "").trim();
        if (/^nsec1/i.test(a)) {
          let s = kr(a.toLowerCase());
          if (s.type === "nsec") return s.data;
        }
        if (Re.test(a.toLowerCase())) return _t(a.toLowerCase());
        let n = this.validWords(a);
        if (n) return _r(n);
        throw new Error("Chave ou palavras inv\xE1lidas");
      }
      verifyRecovery(e) {
        try {
          return Ke(this.skFromInput(e)) === this.pk;
        } catch {
          return !1;
        }
      }
      async createIdentity({ name: e, note: a, words: n, t: s }) {
        let o = _r(n);
        await this.secure.set("sk", Nt(o)),
          await this.secure.set("words", n),
          (this.sk = o),
          (this.pk = Ke(o)),
          (this.me = {
            name: ie(e, 40),
            note: ie(a, 80),
            t: s ?? Math.floor(Math.random() * 6),
            created: ve(),
          }),
          await this.setKv("me", this.me),
          (this.lastSync = be()),
          await this.setKv("lastSync", this.lastSync);
      }
      async restoreIdentity(e, { name: a, note: n }) {
        let s = this.skFromInput(e),
          o = this.validWords(e);
        await this.secure.set("sk", Nt(s)),
          o && (await this.secure.set("words", o)),
          (this.sk = s),
          (this.pk = Ke(s)),
          (this.me = {
            name: ie(a, 40),
            note: ie(n, 80),
            t: Math.floor(Math.random() * 6),
            created: ve(),
            restored: !0,
          }),
          await this.setKv("me", this.me),
          (this.lastSync = be() - 30 * Za),
          await this.setKv("lastSync", this.lastSync);
      }
      async exportKeys() {
        return {
          nsec: bl(this.sk),
          words: (await this.secure.get("words")) || null,
          npub: An(this.pk),
        };
      }
      myNpub() {
        return An(this.pk);
      }
      start() {
        this.pool ||
          ((this.pool = new Js({
            WebSocketImpl: this.WS,
            signAuth: (e, a) =>
              ri(this.sk, {
                kind: 22242,
                tags: [
                  ["relay", e],
                  ["challenge", a],
                ],
              }),
          })),
          this.pool.on("event", (e) => {
            this.q = this.q
              .then(() => this.onWrap(e))
              .catch((a) => console.warn("[wrap]", a && a.message));
          }),
          this.pool.on("status", () => this.emit("relays", this.pool.status())),
          this.pool.on("open", () => {
            clearTimeout(this._ot),
              (this._ot = setTimeout(() => this.flushOutbox(), 800));
          }),
          this.pool.setRelays(this.settings.relays),
          this.pool.subscribe("inbox", () => this.filters()),
          this.timers.push(setInterval(() => this.tick(), 15e3)),
          this.timers.push(setInterval(() => this.flushOutbox(), 3e4)),
          this.timers.push(setInterval(() => this.roomSweep(), 5e3)),
          setTimeout(() => this.publishRelayList(), 4e3),
          setTimeout(() => this.purgeExpired(), 2e3));
      }
      stop() {
        for (let e of this.timers) clearInterval(e);
        (this.timers = []),
          this.pool && this.pool.closeAll(),
          (this.pool = null);
      }
      pks() {
        let e = [this.pk];
        for (let a of this.groups.values()) a.left || e.push(a.pk);
        return e;
      }
      filters() {
        let e = Math.max(0, (this.lastSync || be()) - 2 * Za - 900),
          a = this.pks();
        return [
          { kinds: [Hn], "#p": a, since: e },
          { kinds: [Ya], "#p": a, since: be() - 60 },
        ];
      }
      resubscribe() {
        this.pool && this.pool.refresh("inbox");
      }
      async tick() {
        this.pool &&
          this.pool.openCount() > 0 &&
          ((this.lastSync = be()), await this.setKv("lastSync", this.lastSync)),
          await this.purgeExpired();
      }
      wake() {
        this.pool && this.pool.kickAll();
      }
      async publishRelayList() {
        if (
          !this.pool ||
          (await this.kv("pub10050", "")) === this.settings.relays.join(",")
        )
          return;
        let e = ri(this.sk, {
          kind: 10050,
          tags: this.settings.relays.slice(0, 6).map((n) => ["relay", n]),
        });
        (await this.pool.publish(e)).ok &&
          (await this.setKv("pub10050", this.settings.relays.join(",")));
      }
      markSeen(e) {
        this.seen.add(e),
          this.seenList.push(e),
          this.seenList.length > 6e3 &&
            this.seenList.splice(0, 1e3).forEach((n) => this.seen.delete(n)),
          clearTimeout(this._st),
          (this._st = setTimeout(
            () => this.setKv("seen", this.seenList),
            2500,
          ));
      }
      groupByPk(e) {
        for (let a of this.groups.values()) {
          if (a.left) continue;
          if (a.pk === e) return { g: a, sk: a.sk };
          let n = (a.oldKeys || []).find((s) => s.pk === e);
          if (n) return { g: a, sk: n.sk };
        }
        return null;
      }
      async onWrap(e) {
        if (
          !e ||
          typeof e.id != "string" ||
          this.seen.has(e.id) ||
          (e.kind !== Hn && e.kind !== Ya)
        )
          return;
        let a = Wa(e, "p"),
          n = null,
          s = null;
        if (a === this.pk) n = this.sk;
        else {
          let i = this.groupByPk(a);
          if (!i) return;
          (s = i.g), (n = _t(i.sk));
        }
        this.markSeen(e.id);
        let o;
        try {
          o = Cd(e, n);
        } catch {
          return;
        }
        o.created_at > be() + 600 && (o.created_at = be());
        let r = { eph: e.kind === Ya, group: s };
        s ? await this.onGroupRumor(o, s, r) : await this.onDmRumor(o, r);
      }
      contactName(e) {
        if (e === this.pk) return this.me ? this.me.name : "Voc\xEA";
        let a = this.contacts.get(e);
        if (a && (a.nick || a.name)) return a.nick || a.name;
        for (let n of this.groups.values()) {
          let s = n.meta && n.meta.names && n.meta.names[e];
          if (s) return s;
        }
        return Rd(e);
      }
      async ensureContact(e, a) {
        let n = this.contacts.get(e);
        return (
          n ||
            ((n = {
              pk: e,
              name: "",
              nick: "",
              note: "",
              t: parseInt(e.slice(0, 2), 16) % 6,
              status: a,
              verified: !1,
              relays: [],
              addedAt: ve(),
            }),
            this.contacts.set(e, n),
            await this.db.put("contacts", n),
            this.emit("contact", n)),
          n
        );
      }
      chatFor(e) {
        let a = this.chats.get(e);
        if (!a) {
          let [n, s, o] = e.split(":");
          (a =
            n === "dm"
              ? {
                  key: e,
                  kind: "dm",
                  pk: s,
                  lastTs: 0,
                  unread: 0,
                  ttl: 0,
                  pendingRead: [],
                }
              : {
                  key: e,
                  kind: "group",
                  lid: s,
                  ch: o,
                  lastTs: 0,
                  unread: 0,
                  ttl: 0,
                  pendingRead: [],
                }),
            this.chats.set(e, a);
        }
        return a;
      }
      async saveChat(e) {
        await this.db.put("chats", e), this.emit("chat", e);
      }
      async onDmRumor(e, a) {
        let n = e.pubkey,
          s = n === this.pk,
          o = s ? Wa(e, "p") : n;
        if (s && o === this.pk && e.kind === jt) {
          try {
            await this.onSelfApp(JSON.parse(e.content));
          } catch {}
          return;
        }
        if (!o || !Re.test(o) || o === this.pk) return;
        let r = this.contacts.get(o);
        if (r && r.status === "blocked") return;
        let i = "dm:" + o;
        if (e.kind === Ys) {
          s
            ? await this.ensureContact(o, "ok")
            : await this.ensureContact(o, "request"),
            await this.addMsg(
              i,
              {
                id: e.id,
                from: n,
                kind: "text",
                text: ie(e.content, 6e3),
                reply: this.replyRef(e),
                fwd: Wa(e, "fwd") ? !0 : void 0,
                lp: jmCleanLpJson(Wa(e, "lp")),
                ts: e.created_at * 1e3,
                exp: this.expOf(e),
                status: s ? "sent" : void 0,
              },
              !s,
            ),
            s || this.maybeProfReq(o);
          return;
        }
        if (e.kind === Zs) return this.applyReaction(i, e, n);
        if (e.kind !== jt) return;
        let c;
        try {
          c = JSON.parse(e.content);
        } catch {
          return;
        }
        if (!(!c || typeof c.t != "string"))
          switch (c.t) {
            case "prof":
              if (s) return;
              {
                let l = await this.ensureContact(o, "request");
                (l.name = ie(c.n, 40)),
                  (l.note = ie(c.s, 80)),
                  Number.isInteger(c.c) && c.c >= 0 && c.c < 6 && (l.t = c.c);
                {
                  let u = ma(c.p);
                  u ? (l.pic = u) : u === "" && delete l.pic;
                }
                Array.isArray(c.r) &&
                  (l.relays = c.r.map(ha).filter(Boolean).slice(0, 5)),
                  Array.isArray(c.g) && (l.games = jmCleanGames(c.g)),
                  Array.isArray(c.g) && ((l.st = jmCleanStatus(c.st) || void 0), l.st || delete l.st),
                  await this.db.put("contacts", l),
                  this.emit("contact", l),
                  this.emit("chats");
              }
              return;
            case "profreq":
              !s && r && r.status === "ok" && this.sendProfile(o);
              return;
            case "pban": {
              if (s || !r) return;
              let l = jmCleanBanner(c.b);
              if (l === void 0) return;
              l ? (r.banner = l) : delete r.banner,
                await this.db.put("contacts", r),
                this.emit("contact", r);
              return;
            }
            case "lis": {
              if (s || !a.eph || !r || r.status !== "ok") return;
              let l = jmCleanListening(c);
              this.listening || (this.listening = new Map()),
                l ? this.listening.set(o, { l, at: Date.now() }) : this.listening.delete(o),
                this.emit("listening", o),
                this.emit("contact", r);
              return;
            }
            case "rcpt":
              !s &&
                Array.isArray(c.ids) &&
                (c.s === "d" || c.s === "r") &&
                (await this.applyReceipts(
                  i,
                  c.ids.slice(0, 200),
                  c.s === "r" ? "read" : "delivered",
                ));
              return;
            case "typ":
              !s && a.eph && this.settings.typing && this.emit("typing", i, o);
              return;
            case "del":
              Array.isArray(c.ids) &&
                (await this.applyDelete(i, c.ids.slice(0, 100), n));
              return;
            case "edit":
              await this.applyEdit(i, c.id, c.x, n, e.created_at);
              return;
            case "voo":
              s || (await this.markVoOpened(i, c.id, !0));
              return;
            case "evt":
              s || (await this.ensureContact(o, "request")), await this.addEvent(i, e, c, n, s);
              return;
            case "rsvp":
              await this.applyRsvp(i, c.id, c.r, n);
              return;
            case "pin":
              await this.applyPin(i, c.id, !!c.on, n);
              return;
            case "poll":
              s || (await this.ensureContact(o, "request")),
                await this.addPoll(i, e, c, n, s);
              return;
            case "vote":
              await this.applyVote(i, c.id, c.v, n);
              return;
            case "pres":
              if (s || !a.eph || !r || r.status !== "ok") return;
              this.presence || (this.presence = new Map());
              {
                let l = this.presence.get(o),
                  u = !l || Date.now() - l.at > JM_PRES_TTL;
                u &&
                  c.s !== "off" &&
                  this.myPres &&
                  this.myPres.s !== "off" &&
                  this.sendApp(i, { t: "pres", s: this.myPres.s }, { ephemeral: !0, selfCopy: !1, noTtl: !0 }).catch(() => {});
              }
              this.presence.set(o, { s: c.s === "away" ? "away" : c.s === "off" ? "off" : c.s === "dnd" ? "dnd" : "on", at: Date.now() }),
                this.emit("presence", o),
                this.emit("contact", r);
              return;
            case "ttl": {
              let l = Math.max(0, Math.min(28 * Za, parseInt(c.v, 10) || 0)),
                u = this.chatFor(i);
              (u.ttl = l),
                await this.saveChat(u),
                await this.addMsg(
                  i,
                  {
                    id: e.id,
                    from: n,
                    kind: "sys",
                    sys: "ttl",
                    v: l,
                    ts: e.created_at * 1e3,
                  },
                  !1,
                );
              return;
            }
            case "mh":
              return this.onMediaHeader(i, e, c, n, s);
            case "mc":
              return this.onChunk(c);
            case "snd":
              if (typeof c.id != "string") return;
              s || (await this.ensureContact(o, "request")),
                await this.addMsg(
                  i,
                  {
                    id: e.id,
                    from: n,
                    kind: "sound",
                    snd: ie(c.id, 20),
                    ts: e.created_at * 1e3,
                    exp: this.expOf(e),
                    status: s ? "sent" : void 0,
                  },
                  !s,
                );
              return;
            case "ginv": {
              if (!s && (!r || r.status !== "ok")) return;
              let l = this.parseInvite(c.g);
              if (!l) return;
              await this.addMsg(
                i,
                {
                  id: e.id,
                  from: n,
                  kind: "invite",
                  inv: l,
                  ts: e.created_at * 1e3,
                  status: s ? "sent" : void 0,
                },
                !s,
              );
              return;
            }
            case "grekey":
              s || (await this.applyRekey(n, c));
              return;
            case "call":
              if (s) return;
              if (a.eph) {
                this.emit("signal", { from: o, data: c });
                return;
              }
              c.a === "missed" &&
                (await this.addMsg(
                  i,
                  {
                    id: e.id,
                    from: n,
                    kind: "sys",
                    sys: "missed",
                    v: c.v ? 1 : 0,
                    ts: e.created_at * 1e3,
                  },
                  !0,
                ),
                this.emit("missedCall", o, !!c.v, e.created_at * 1e3));
              return;
          }
      }
      async onGroupRumor(e, a, n) {
        let s = e.pubkey,
          o = s === this.pk,
          r = Wa(e, "ch"),
          i = a.meta.channels.filter((p) => p.type === "text"),
          c = i.find((p) => p.id === r) ? r : (i[0] || {}).id,
          l = c ? `g:${a.lid}:${c}` : null,
          u = !!r && (a.gone || []).includes(r);
        if (e.kind === Ys) {
          if (!l || u) return;
          await this.addMsg(
            l,
            {
              id: e.id,
              from: s,
              kind: "text",
              text: ie(e.content, 6e3),
              reply: this.replyRef(e),
              fwd: Wa(e, "fwd") ? !0 : void 0,
              lp: jmCleanLpJson(Wa(e, "lp")),
              ts: e.created_at * 1e3,
              exp: this.expOf(e),
              status: o ? "sent" : void 0,
            },
            !o,
          );
          return;
        }
        if (e.kind === Zs) {
          l && !u && (await this.applyReaction(l, e, s));
          return;
        }
        if (e.kind !== jt) return;
        let d;
        try {
          d = JSON.parse(e.content);
        } catch {
          return;
        }
        if (!d || typeof d.t != "string") return;
        let f = (p) => a.meta.owner === p || (a.meta.admins || []).includes(p);
        switch (d.t) {
          case "gmeta": {
            if (!f(s) || o) return;
            let p = this.cleanMeta(d.meta, a.meta.owner);
            if (
              !p ||
              !(d.ver > a.ver) ||
              (p.owner !== a.meta.owner && s !== a.meta.owner)
            )
              return;
            let v = a.meta.members.includes(this.pk),
              k = a.meta.channels.filter(
                (L) =>
                  L.type === "text" && !p.channels.some((N) => N.id === L.id),
              );
            (a.meta = p), (a.ver = d.ver), await this.saveGroup(a);
            for (let L of k) await this.purgeChannel(a, L.id);
            v &&
              !p.members.includes(this.pk) &&
              (await this.sysGroup(a, "kicked", s, e.created_at));
            return;
          }
          case "gpic": {
            if (!f(s) || o) return;
            let p = ma(d.p),
              v = Number(d.v) || 0;
            if (p === void 0 || v <= (a.picV || 0)) return;
            (a.pic = p),
              (a.picV = v),
              await this.saveGroup(a),
              await this.sysGroup(
                a,
                p ? "gpic" : "gpicdel",
                s,
                e.created_at,
                e.id,
              );
            return;
          }
          case "gjoin": {
            let p = ie(d.n, 40);
            a.meta.names = a.meta.names || {};
            let v = !a.meta.members.includes(s);
            p && (a.meta.names[s] = p);
            let k = ma(d.p);
            k !== void 0 &&
              !o &&
              ((a.pics = a.pics || {}), k ? (a.pics[s] = k) : delete a.pics[s]),
              v && f(this.pk) && a.meta.members.length < 64
                ? (a.meta.members.push(s),
                  a.ver++,
                  await this.saveGroup(a),
                  await this.publishMeta(a),
                  a.pic &&
                    (await this.sendGroupApp(a, {
                      t: "gpic",
                      p: a.pic,
                      v: a.picV || 1,
                    })))
                : await this.saveGroup(a),
              v &&
                !o &&
                (await this.sysGroup(a, "join", s, e.created_at, e.id));
            return;
          }
          case "gleave": {
            if (o) return;
            a.meta.members.includes(s) &&
              ((a.meta.members = a.meta.members.filter((p) => p !== s)),
              (a.meta.admins = (a.meta.admins || []).filter((p) => p !== s)),
              f(this.pk)
                ? (a.ver++, await this.saveGroup(a), await this.publishMeta(a))
                : await this.saveGroup(a),
              await this.sysGroup(a, "leave", s, e.created_at, e.id));
            return;
          }
          case "mh":
            l && !u && (await this.onMediaHeader(l, e, d, s, o));
            return;
          case "mc":
            return this.onChunk(d);
          case "snd":
            l &&
              !u &&
              typeof d.id == "string" &&
              (await this.addMsg(
                l,
                {
                  id: e.id,
                  from: s,
                  kind: "sound",
                  snd: ie(d.id, 20),
                  ts: e.created_at * 1e3,
                  status: o ? "sent" : void 0,
                },
                !o,
              ));
            return;
          case "del":
            l &&
              !u &&
              Array.isArray(d.ids) &&
              (await this.applyDelete(l, d.ids.slice(0, 100), s, jmCan(a.meta, s, "del")));
            return;
          case "edit":
            l && !u && (await this.applyEdit(l, d.id, d.x, s, e.created_at));
            return;
          case "pin":
            l && !u && jmCan(a.meta, s, "pin") && (await this.applyPin(l, d.id, !!d.on, s));
            return;
          case "evt":
            l && !u && (await this.addEvent(l, e, d, s, o));
            return;
          case "rsvp":
            l && !u && (await this.applyRsvp(l, d.id, d.r, s));
            return;
          case "poll":
            l && !u && (await this.addPoll(l, e, d, s, o));
            return;
          case "vote":
            l && !u && (await this.applyVote(l, d.id, d.v, s));
            return;
          case "room":
            n.eph &&
              !o &&
              typeof d.ch == "string" &&
              this.onRoom(a, d.ch, s, d.a === "leave" ? "leave" : "here");
            return;
        }
      }
      replyRef(e) {
        let a = (e.tags || []).find(
          (n) => n[0] === "e" && typeof n[1] == "string" && Re.test(n[1]),
        );
        return a ? a[1] : void 0;
      }
      expOf(e) {
        let a = parseInt(Wa(e, "expiration"), 10);
        return a > 0 ? a * 1e3 : void 0;
      }
      async addMsg(e, a, n) {
        if (a.exp && a.exp <= ve()) return null;
        let s = await this.db.get("msgs", a.id);
        if (s) return s;
        (a.chat = e),
          a.ts || (a.ts = ve()),
          Object.keys(a).forEach((c) => a[c] === void 0 && delete a[c]),
          await this.db.put("msgs", a);
        let o = this.chatFor(e),
          r = this.active.chat === e && this.active.visible;
        a.ts >= o.lastTs &&
          ((o.lastTs = a.ts),
          (o.lastFrom = a.from),
          (o.lastId = a.id),
          (o.lastKind = a.kind),
          (o.lastText = this.preview(a)),
          (o.lastStatus = a.status));
        let i = n && a.kind !== "sys";
        if (
          (i && !r && (o.unread = (o.unread || 0) + 1), i && o.kind === "dm")
        ) {
          let c = (this.contacts.get(o.pk) || {}).status === "ok";
          c && this.queueReceipt(o.pk, a.id, "d"),
            c && r && this.settings.readReceipts
              ? this.queueReceipt(o.pk, a.id, "r")
              : (o.pendingRead = [...(o.pendingRead || []), a.id].slice(-200));
        }
        return (
          o.deleted && delete o.deleted,
          await this.saveChat(o),
          this.emit("msg", e, a),
          n && a.kind !== "sys" && this.emit("incoming", e, a, r),
          a
        );
      }
      preview(e) {
        switch (e.kind) {
          case "text":
            return e.text;
          case "media":
            return e.media && e.media.type === "file"
              ? "Arquivo: " + (e.media.name || "")
              : e.media && e.media.vo
              ? e.media.type === "audio"
                ? "\xC1udio de visualiza\xE7\xE3o \xFAnica"
                : "Foto de visualiza\xE7\xE3o \xFAnica"
              : e.media && e.media.sb
              ? "Som: " + e.media.sb.n
              : e.media && e.media.type === "audio"
                ? "Mensagem de voz"
                : e.media && /^image\/gif/i.test(e.media.mime || "")
                  ? "GIF"
                  : "Foto";
          case "sound":
            return "Som do soundboard";
          case "poll":
            return "Enquete: " + ((e.poll && e.poll.q) || "");
          case "event":
            return "Evento: " + ((e.evt && e.evt.ti) || "");
          case "invite":
            return (
              "Convite para " +
              ((e.inv && e.inv.meta && e.inv.meta.name) || "um grupo")
            );
          case "deleted":
            return "Mensagem apagada";
          case "sys":
            return (
              {
                missed: "Chamada perdida",
                ttl: e.v
                  ? "Mensagens tempor\xE1rias ativadas"
                  : "Mensagens tempor\xE1rias desativadas",
                created: "Grupo criado",
                joined: "Voc\xEA entrou no grupo",
                join: this.contactName(e.from) + " entrou no grupo",
                leave: this.contactName(e.from) + " saiu do grupo",
                removed: this.contactName(e.from) + " foi removido",
                kicked: "Voc\xEA foi removido do grupo",
                gpic: this.contactName(e.from) + " mudou a foto do grupo",
                gpicdel: this.contactName(e.from) + " removeu a foto do grupo",
              }[e.sys] || "Aviso"
            );
          default:
            return "";
        }
      }
      async applyReaction(e, a, n) {
        let s = Wa(a, "e");
        if (!s || !Re.test(s)) return;
        let o = await this.db.get("msgs", s);
        if (!o || o.chat !== e) return;
        o.reacts = o.reacts || {};
        let r = ie(a.content, 16);
        !r || r === "-" ? delete o.reacts[n] : (o.reacts[n] = r),
          await this.db.put("msgs", o),
          this.emit("msgUpdate", e, o);
      }
      async applyReceipts(e, a, n) {
        for (let s of a) {
          if (typeof s != "string" || !Re.test(s)) continue;
          let o = await this.db.get("msgs", s);
          if (
            !o ||
            o.chat !== e ||
            o.from !== this.pk ||
            (Qs[o.status] || 0) >= Qs[n]
          )
            continue;
          (o.status = n), await this.db.put("msgs", o);
          let r = this.chats.get(e);
          r && r.lastId === s && ((r.lastStatus = n), await this.saveChat(r)),
            this.emit("msgUpdate", e, o);
        }
      }
      async applyDelete(e, a, n, jmMod = !1) {
        for (let s of a) {
          if (typeof s != "string" || !Re.test(s)) continue;
          let o = await this.db.get("msgs", s);
          if (!o || o.chat !== e || (o.from !== n && !jmMod)) continue;
          o.media && (await this.db.del("media", o.media.mid));
          let r = {
            id: o.id,
            chat: o.chat,
            from: o.from,
            ts: o.ts,
            kind: "deleted",
          };
          await this.db.put("msgs", r);
          let i = this.chats.get(e);
          i &&
            i.lastId === s &&
            ((i.lastText = "Mensagem apagada"), await this.saveChat(i)),
            this.emit("msgUpdate", e, r);
        }
      }
      queueReceipt(e, a, n) {
        let s = this.rq.get(e);
        s || ((s = { d: new Set(), r: new Set() }), this.rq.set(e, s)),
          s[n].add(a),
          clearTimeout(this._rt),
          (this._rt = setTimeout(() => this.flushReceipts(), 1200));
      }
      async flushReceipts() {
        let e = [...this.rq.entries()];
        this.rq.clear();
        for (let [a, n] of e) {
          let s = this.contacts.get(a);
          if (!(!s || s.status === "blocked"))
            for (let o of ["d", "r"])
              n[o].size &&
                ((o === "r" && !this.settings.readReceipts) ||
                  (await this.sendApp(
                    "dm:" + a,
                    { t: "rcpt", s: o, ids: [...n[o]] },
                    { selfCopy: !1, expiresIn: 3 * Za, noTtl: !0 },
                  )));
        }
      }
      maybeProfReq(e) {
        let a = this.contacts.get(e);
        if (!a || a.name || a.status !== "ok") return;
        let n = this.profReqAt.get(e) || 0;
        ve() - n < 6 * 3600 * 1e3 ||
          (this.profReqAt.set(e, ve()),
          this.sendApp(
            "dm:" + e,
            { t: "profreq" },
            { selfCopy: !1, expiresIn: 3 * Za, noTtl: !0 },
          ));
      }
      chatTarget(e) {
        if (e.kind === "dm")
          return {
            recip: e.pk,
            tags: [["p", e.pk]],
            relays: (this.contacts.get(e.pk) || {}).relays || [],
          };
        let a = this.groups.get(e.lid);
        if (!a || a.left) throw new Error("Grupo indispon\xEDvel");
        return {
          recip: a.pk,
          tags: [
            ["p", a.pk],
            ["ch", e.ch],
          ],
          relays: [],
        };
      }
      build(e, a, n, s = [], o = {}) {
        let r = this.chatFor(e),
          i = this.chatTarget(r),
          c = [...i.tags, ...s],
          l = o.noTtl ? 0 : r.ttl || 0,
          u = l ? be() + l : 0;
        u && c.push(["expiration", String(u)]);
        let d = Sd(this.sk, a, n, c),
          f = o.expiresIn ? be() + o.expiresIn : u ? u + 3600 : void 0,
          p = [
            oi(d, this.sk, i.recip, { ephemeral: o.ephemeral, expiration: f }),
          ];
        return (
          o.selfCopy !== !1 &&
            r.kind === "dm" &&
            !o.ephemeral &&
            p.push(oi(d, this.sk, this.pk, { expiration: f })),
          { rumor: d, wraps: p, relays: i.relays, exp: u ? u * 1e3 : void 0 }
        );
      }
      async publish(e, a, n) {
        if (!this.pool) return !1;
        let s = await this.pool.publish(a[0], n);
        for (let o = 1; o < a.length; o++) this.pool.publish(a[o], n);
        return (
          s.ok ||
            (await this.db.put("outbox", {
              id: e,
              wraps: a,
              relays: n,
              at: ve(),
            })),
          s.ok
        );
      }
      async sendApp(e, a, n = {}) {
        let s = this.build(e, jt, JSON.stringify(a), n.tags || [], n);
        return n.ephemeral
          ? (this.pool && this.pool.publish(s.wraps[0], s.relays), s.rumor)
          : (await this.publish(s.rumor.id, s.wraps, s.relays), s.rumor);
      }
      async localSend(e, a, n) {
        return (
          (a.status = "pending"),
          (a.exp = n.exp),
          await this.addMsg(e, a, !1),
          (await this.publish(n.rumor.id, n.wraps, n.relays)) &&
            (await this.setStatus(e, n.rumor.id, "sent"), (a.status = "sent")),
          a
        );
      }
      async setStatus(e, a, n) {
        let s = await this.db.get("msgs", a);
        if (!s || (Qs[s.status] || 0) >= Qs[n]) return;
        (s.status = n), await this.db.put("msgs", s);
        let o = this.chats.get(e);
        o && o.lastId === a && ((o.lastStatus = n), await this.saveChat(o)),
          this.emit("msgUpdate", e, s);
      }
      async flushOutbox() {
        if (!(!this.pool || this.pool.openCount() === 0 || this._flushing)) {
          this._flushing = !0;
          try {
            for (let e of await this.db.all("outbox"))
              if ((await this.pool.publish(e.wraps[0], e.relays || [])).ok) {
                for (let s = 1; s < e.wraps.length; s++)
                  this.pool.publish(e.wraps[s], e.relays || []);
                await this.db.del("outbox", e.id);
                let n = await this.db.get("msgs", e.id);
                n &&
                  n.status === "pending" &&
                  (await this.setStatus(n.chat, n.id, "sent"));
              }
          } finally {
            this._flushing = !1;
          }
        }
      }
      async retry(e, a) {
        await this.flushOutbox();
      }
      async sendText(e, a, { reply: n, fwd: jf, lp: jl } = {}) {
        if (((a = String(a || "").trim()), !a)) return null;
        a = a.slice(0, 6e3);
        let s = n ? [["e", n, "", "reply"]] : [];
        (jl = jmCleanLp(jl)), jf && s.push(["fwd", "1"]), jl && s.push(["lp", JSON.stringify(jl)]);
        let o = this.build(e, Ys, a, s);
        return (
          await this.ensureDmContact(e),
          this.localSend(
            e,
            {
              id: o.rumor.id,
              from: this.pk,
              kind: "text",
              text: a,
              reply: n,
              fwd: jf ? !0 : void 0,
              lp: jl || void 0,
              ts: o.rumor.created_at * 1e3,
            },
            o,
          )
        );
      }
      async ensureDmContact(e) {
        let a = this.chatFor(e);
        if (a.kind !== "dm") return;
        let n = await this.ensureContact(a.pk, "ok");
        n.status === "request" &&
          ((n.status = "ok"),
          await this.db.put("contacts", n),
          this.emit("contact", n)),
          n.profSent || (await this.sendProfile(a.pk));
      }
      async sendSound(e, a) {
        let n = this.build(e, jt, JSON.stringify({ t: "snd", id: a }));
        return (
          await this.ensureDmContact(e),
          this.localSend(
            e,
            {
              id: n.rumor.id,
              from: this.pk,
              kind: "sound",
              snd: a,
              ts: n.rumor.created_at * 1e3,
            },
            n,
          )
        );
      }
      async sendReaction(e, a, n) {
        let s = await this.db.get("msgs", a);
        if (!s) return;
        (s.reacts = s.reacts || {}),
          !n || s.reacts[this.pk] === n
            ? (delete s.reacts[this.pk], (n = "-"))
            : (s.reacts[this.pk] = n),
          await this.db.put("msgs", s),
          this.emit("msgUpdate", e, s);
        let o = this.build(
          e,
          Zs,
          n,
          [
            ["e", a],
            ["k", "14"],
          ],
          { noTtl: !0 },
        );
        await this.publish(o.rumor.id, o.wraps, o.relays);
      }
      /** Edição de texto: só quem escreveu edita, e só mensagens de texto. */
      async applyEdit(e, a, n, s, o) {
        if (typeof a != "string" || !Re.test(a)) return;
        let r = ie(n, 6e3);
        if (!r) return;
        let i = await this.db.get("msgs", a);
        if (!i || i.chat !== e || i.from !== s || i.kind !== "text") return;
        let c = (o || be()) * 1e3;
        if (i.edited && i.edited > c) return;
        (i.text = r), (i.edited = c), await this.db.put("msgs", i);
        let l = this.chats.get(e);
        l && l.lastId === a && ((l.lastText = r), await this.saveChat(l)),
          this.emit("msgUpdate", e, i);
      }
      async editMsg(e, a, n) {
        let s = await this.db.get("msgs", a);
        if (((n = String(n || "").trim().slice(0, 6e3)), !s || s.from !== this.pk || s.kind !== "text" || !n || n === s.text)) return;
        await this.applyEdit(e, a, n, this.pk, be()),
          await this.sendApp(e, { t: "edit", id: a, x: n }, { noTtl: !0 });
      }
      /** Mensagens fixadas ficam na conversa (as 50 mais recentes). */
      async applyPin(e, a, n, s) {
        if (typeof a != "string" || !Re.test(a)) return;
        let o = this.chatFor(e),
          r = (o.pins || []).filter((i) => i !== a);
        n && r.unshift(a), (o.pins = r.slice(0, 50)), await this.saveChat(o), this.emit("pins", e);
      }
      async setPin(e, a, n) {
        await this.applyPin(e, a, n, this.pk),
          await this.sendApp(e, { t: "pin", id: a, on: n ? 1 : 0 }, { noTtl: !0 });
      }
      /** Enquete: pergunta, de 2 a 10 opções, e se aceita mais de uma resposta. */
      async addPoll(e, a, n, s, o) {
        let r = ie(n.q, 200),
          i = (Array.isArray(n.o) ? n.o : []).map((c) => ie(c, 80)).filter(Boolean).slice(0, 10);
        !r ||
          i.length < 2 ||
          (await this.addMsg(
            e,
            {
              id: a.id,
              from: s,
              kind: "poll",
              poll: { q: r, o: i, m: n.m ? 1 : 0 },
              votes: {},
              ts: a.created_at * 1e3,
              exp: this.expOf(a),
              status: o ? "sent" : void 0,
            },
            !o,
          ));
      }
      async sendPoll(e, a, n, s) {
        let o = { t: "poll", q: ie(a, 200), o: n.map((i) => ie(i, 80)).filter(Boolean).slice(0, 10), m: s ? 1 : 0 };
        if (!o.q || o.o.length < 2) return null;
        let r = this.build(e, jt, JSON.stringify(o));
        return (
          await this.ensureDmContact(e),
          this.localSend(e, { id: r.rumor.id, from: this.pk, kind: "poll", poll: { q: o.q, o: o.o, m: o.m }, votes: {}, ts: r.rumor.created_at * 1e3 }, r)
        );
      }
      async applyVote(e, a, n, s) {
        if (typeof a != "string" || !Re.test(a)) return;
        let o = await this.db.get("msgs", a);
        if (!o || o.chat !== e || o.kind !== "poll") return;
        let r = [...new Set((Array.isArray(n) ? n : []).filter((i) => Number.isInteger(i) && i >= 0 && i < o.poll.o.length))];
        o.poll.m || (r = r.slice(0, 1)),
          (o.votes = o.votes || {}),
          r.length ? (o.votes[s] = r) : delete o.votes[s],
          await this.db.put("msgs", o),
          this.emit("msgUpdate", e, o);
      }
      async vote(e, a, n) {
        await this.applyVote(e, a, n, this.pk),
          await this.sendApp(e, { t: "vote", id: a, v: n }, { noTtl: !0 });
      }
      /** Online / ausente: só entre contatos aceitos, e só se a pessoa deixar. */
      presenceOf(e) {
        if (!this.settings.presence) return null;
        let a = this.presence && this.presence.get(e);
        return !a || Date.now() - a.at > JM_PRES_TTL || a.s === "off" ? null : a.s;
      }
      async setPresence(e, a = !1) {
        let jmD = jmCleanStatus(this.me && this.me.st),
          n = this.settings.presence ? (jmD && jmD.dnd ? "dnd" : e) : "off",
          s = Date.now();
        if (!a && this.myPres && this.myPres.s === n && s - this.myPres.at < JM_PRES_HEARTBEAT) return;
        if (n === "off" && (!this.myPres || this.myPres.s === "off") && !a) return;
        this.myPres = { s: n, at: s };
        for (let o of [...this.contacts.values()].filter((r) => r.status === "ok"))
          try {
            await this.sendApp("dm:" + o.pk, { t: "pres", s: n }, { ephemeral: !0, selfCopy: !1, noTtl: !0 });
          } catch {}
      }
      // ---- Eventos ----
      async addEvent(e, a, n, s, o) {
        let r = ie(n.ti, 120),
          i = Number(n.at);
        !r ||
          !(i >= 15778368e5 && i < 41024448e5) ||
          (await this.addMsg(e, { id: a.id, from: s, kind: "event", evt: { ti: r, at: i, d: ie(n.d, 500) }, rsvp: {}, ts: a.created_at * 1e3, exp: this.expOf(a), status: o ? "sent" : void 0 }, !o));
      }
      async sendEvent(e, a, n, s) {
        if (((a = ie(a, 120)), !a)) throw new Error("D\xEA um nome ao evento.");
        if (!(n > Date.now() - 36e5)) throw new Error("A data do evento j\xE1 passou.");
        let o = { t: "evt", ti: a, at: n };
        (s = ie(s, 500)) && (o.d = s);
        let r = this.build(e, jt, JSON.stringify(o));
        return await this.ensureDmContact(e), this.localSend(e, { id: r.rumor.id, from: this.pk, kind: "event", evt: { ti: a, at: n, d: s || "" }, rsvp: {}, ts: r.rumor.created_at * 1e3 }, r);
      }
      async applyRsvp(e, a, n, s) {
        if (typeof a != "string" || !Re.test(a)) return;
        let o = await this.db.get("msgs", a);
        !o ||
          o.chat !== e ||
          o.kind !== "event" ||
          ((o.rsvp = o.rsvp || {}), ["y", "m", "n"].includes(n) ? (o.rsvp[s] = n) : delete o.rsvp[s], await this.db.put("msgs", o), this.emit("msgUpdate", e, o));
      }
      async rsvp(e, a, n) {
        await this.applyRsvp(e, a, n, this.pk), await this.sendApp(e, { t: "rsvp", id: a, r: n }, { noTtl: !0 });
      }
      // ---- Visualização única e encaminhar ----
      async markVoOpened(e, a, n) {
        if (typeof a != "string" || !Re.test(a)) return;
        let s = await this.db.get("msgs", a);
        if (!s || s.chat !== e || s.kind !== "media" || !s.media || !s.media.vo || s.media.opened) return;
        if (n && s.from !== this.pk) return;
        (s.media.opened = 1), delete s.media.th, await this.db.del("media", s.media.mid), await this.db.put("msgs", s), this.emit("msgUpdate", e, s);
      }
      /** Abre a mídia de visualização única uma vez: devolve o Blob e apaga. */
      async openViewOnce(e, a) {
        let n = await this.db.get("msgs", a);
        if (!n || !n.media || !n.media.vo || n.media.opened || n.from === this.pk) throw new Error("Esta m\xEDdia j\xE1 foi aberta.");
        if (!n.media.ready) throw new Error("Ainda chegando\u2026");
        let s = await this.mediaBlob(n.media.mid);
        if (!s) throw new Error("M\xEDdia indispon\xEDvel");
        return (
          await this.markVoOpened(e, a, !1),
          e.startsWith("dm:") && this.sendApp(e, { t: "voo", id: a }, { noTtl: !0, selfCopy: !1 }).catch(() => {}),
          s
        );
      }
      async forward(e, a) {
        let n = await this.db.get("msgs", e);
        if (!n) throw new Error("Mensagem n\xE3o encontrada");
        if (n.kind === "text") return this.sendText(a, n.text, { fwd: !0, lp: n.lp });
        if (n.kind === "sound") return this.sendSound(a, n.snd);
        if (n.kind !== "media" || !n.media) throw new Error("Este tipo de mensagem n\xE3o pode ser encaminhado.");
        let s = n.media;
        if (s.vo) throw new Error("Visualiza\xE7\xE3o \xFAnica n\xE3o pode ser encaminhada.");
        let o = await this.mediaBlob(s.mid);
        if (!o) throw new Error("A m\xEDdia ainda n\xE3o chegou inteira.");
        return this.sendMedia(a, new Uint8Array(await o.arrayBuffer()), { type: s.type, mime: s.mime, dur: s.dur, wf: s.wf, w: s.w, h: s.h, th: s.th, sb: s.sb, name: s.name, fwd: !0 });
      }
      // ---- Backup e a mesma conta em vários aparelhos (mesmo formato do PC) ----
      async deviceId() {
        let e = await this.kv("deviceId", null);
        return e || ((e = Ka(8)), await this.setKv("deviceId", e)), e;
      }
      /** Retrato da conta. Com mensagens é o backup; sem, vai para o outro aparelho. */
      async snapshot(e) {
        let a = this.me || {},
          n = { name: a.name || "", note: a.note || "", t: a.t | 0, games: a.games || [] };
        e && ((n.pic = a.pic || null), (n.banner = a.banner || null));
        let s = [...this.contacts.values()].map((c) => {
            let l = { pk: c.pk, name: c.name || "", nick: c.nick || "", note: c.note || "", t: c.t | 0, status: c.status, verified: !!c.verified, relays: c.relays || [], games: c.games || [], addedAt: c.addedAt || 0 };
            return e && ((l.pic = c.pic || null), (l.banner = c.banner || null)), l;
          }),
          o = [...this.groups.values()]
            .filter((c) => !c.left)
            .map((c) => {
              let l = { lid: c.lid, sk: c.sk, old: (c.oldKeys || []).map((u) => u.sk), meta: c.meta, ver: c.ver, picV: c.picV || 0, mute: !!c.mute, gone: c.gone || [] };
              return e && (l.pic = c.pic || null), l;
            }),
          r = [...this.chats.values()].map((c) => ({ key: c.key, ttl: c.ttl || 0, muted: !!c.muted, pinned: !!c.pinned, pins: c.pins || [] })),
          i = {
            v: 1,
            app: "jmenseger",
            by: "android",
            dev: await this.deviceId(),
            at: ve(),
            me: n,
            contacts: s,
            groups: o,
            chats: r,
            settings: { readReceipts: !!this.settings.readReceipts, typing: !!this.settings.typing, presence: this.settings.presence !== !1, ttlDefault: this.settings.ttlDefault | 0, relays: this.settings.relays },
          };
        if (e) {
          let c = [];
          for (let l of await this.db.all("msgs")) {
            if (!["text", "sys", "sound", "poll", "event", "deleted"].includes(l.kind)) continue;
            let u = { id: l.id, chat: l.chat, from: l.from, ts: l.ts, kind: l.kind };
            for (let d of ["text", "sys", "v", "snd", "poll", "votes", "evt", "rsvp", "reply", "reacts", "status", "exp", "edited"]) l[d] !== void 0 && l[d] !== null && (u[d] = l[d]);
            u.status === "failed" && (u.status = "pending"), c.push(u);
          }
          i.msgs = c.sort((l, u) => l.ts - u.ts);
        }
        return i;
      }
      /** Junta um retrato com o que já existe. Nada é apagado. */
      async mergeSnapshot(e, a) {
        if (!e || e.v !== 1) throw new Error("Vers\xE3o de backup desconhecida.");
        let n = { contacts: 0, groups: 0, messages: 0 },
          s = e.me || {};
        (a || !(this.me && this.me.name)) &&
          s.name &&
          (await this.updateProfile({ name: s.name, note: s.note, t: Number.isInteger(s.t) ? s.t % 6 : void 0, pic: !(this.me && this.me.pic) && s.pic ? s.pic : void 0 }).catch(() => {}));
        let o = { request: 0, ok: 1, blocked: 2 };
        for (let c of Array.isArray(e.contacts) ? e.contacts : []) {
          if (!c || !Re.test(c.pk) || c.pk === this.pk || !(c.status in o)) continue;
          let l = this.contacts.get(c.pk),
            u = l || (await this.ensureContact(c.pk, c.status));
          !l && n.contacts++,
            u.name || (u.name = ie(c.name, 40)),
            u.note || (u.note = ie(c.note, 80)),
            c.nick && (u.nick = ie(c.nick, 40)),
            Number.isInteger(c.t) && (u.t = c.t % 6),
            (!l || o[c.status] > o[u.status]) && (u.status = c.status),
            (u.verified = !!(u.verified || c.verified)),
            (u.relays && u.relays.length) || (u.relays = (Array.isArray(c.relays) ? c.relays : []).map(ha).filter(Boolean).slice(0, 5)),
            (u.games && u.games.length) || !Array.isArray(c.games) || (u.games = jmCleanGames(c.games));
          if (!u.pic && c.pic) {
            let d = ma(c.pic);
            d && (u.pic = d);
          }
          if (!u.banner && c.banner) {
            let d = jmCleanBanner(c.banner);
            d && (u.banner = d);
          }
          await this.db.put("contacts", u), this.emit("contact", u);
        }
        let r = !1;
        for (let c of Array.isArray(e.groups) ? e.groups : []) {
          if (!c || typeof c.sk != "string" || !Re.test(c.sk)) continue;
          let l = Ke(_t(c.sk)),
            u = Re.test(c.lid) ? c.lid : l,
            d = this.groups.get(u),
            f = Number(c.ver) || 1;
          if (d && !d.left && d.ver >= f) continue;
          let p = this.cleanMeta(c.meta, "");
          if (!p) continue;
          let v = {
            ...(d || {}),
            lid: u,
            pk: l,
            sk: c.sk,
            oldKeys: (Array.isArray(c.old) ? c.old : []).filter((k) => typeof k == "string" && Re.test(k)).slice(0, 20).map((k) => ({ pk: Ke(_t(k)), sk: k })),
            meta: p,
            ver: f,
            picV: Number(c.picV) || 0,
            mute: !!c.mute,
            gone: (Array.isArray(c.gone) ? c.gone : []).filter((k) => typeof k == "string").slice(0, 50),
            joinedAt: (d && d.joinedAt) || ve(),
          };
          if ((delete v.left, !v.pic && c.pic)) {
            let k = ma(c.pic);
            k && (v.pic = k);
          }
          this.groups.set(u, v), await this.saveGroup(v), (!d || d.left) && (n.groups++, (r = !0));
        }
        r && this.resubscribe();
        let i = new Set();
        for (let c of Array.isArray(e.msgs) ? e.msgs : []) {
          if (!c || !Re.test(c.id) || !Re.test(c.from) || typeof c.chat != "string" || !/^(dm:[0-9a-f]{64}|g:[0-9a-f]{64}:[a-z0-9-]+)$/.test(c.chat)) continue;
          if (!["text", "sys", "sound", "poll", "event", "deleted"].includes(c.kind) || !Number.isFinite(c.ts)) continue;
          if (await this.db.get("msgs", c.id)) continue;
          let l = { id: c.id, chat: c.chat, from: c.from, ts: c.ts, kind: c.kind };
          c.kind === "text" && (l.text = ie(c.text, 6e3)),
            c.kind === "sys" && ((l.sys = ie(c.sys, 20)), (l.v = Number(c.v) || 0)),
            c.kind === "sound" && (l.snd = ie(c.snd, 20)),
            c.kind === "poll" &&
              c.poll &&
              ((l.poll = { q: ie(c.poll.q, 200), o: (Array.isArray(c.poll.o) ? c.poll.o : []).map((k) => ie(k, 80)).slice(0, 10), m: c.poll.m ? 1 : 0 }), (l.votes = c.votes && typeof c.votes == "object" ? c.votes : {})),
            Re.test(c.reply || "") && (l.reply = c.reply),
            c.reacts && typeof c.reacts == "object" && (l.reacts = c.reacts),
            ["pending", "sent", "delivered", "read"].includes(c.status) && (l.status = c.status),
            Number.isFinite(c.exp) && (l.exp = c.exp),
            Number.isFinite(c.edited) && (l.edited = c.edited);
          c.kind === "event" &&
            c.evt &&
            ((l.evt = { ti: ie(c.evt.ti, 120), at: Number(c.evt.at) || 0, d: ie(c.evt.d, 500) }), (l.rsvp = c.rsvp && typeof c.rsvp == "object" ? c.rsvp : {}));
          if (l.exp && l.exp <= ve()) continue;
          await this.db.put("msgs", l), n.messages++, i.add(c.chat);
        }
        for (let c of Array.isArray(e.chats) ? e.chats : []) {
          if (!c || typeof c.key != "string" || !/^(dm:[0-9a-f]{64}|g:[0-9a-f]{64}:[a-z0-9-]+)$/.test(c.key) || c.key === "dm:" + this.pk) continue;
          let l = this.chatFor(c.key);
          Number.isFinite(c.ttl) && (l.ttl = Math.max(0, Math.min(28 * Za, c.ttl))),
            typeof c.muted == "boolean" && (l.muted = c.muted),
            typeof c.pinned == "boolean" && (l.pinned = c.pinned),
            Array.isArray(c.pins) && c.pins.length && (l.pins = c.pins.filter((u) => Re.test(u)).slice(0, 50)),
            await this.saveChat(l),
            i.add(c.key);
        }
        for (let c of i) await this.refreshLast(c);
        if (a && e.settings) {
          let c = e.settings;
          for (let [l, u] of [["readReceipts", "boolean"], ["typing", "boolean"], ["presence", "boolean"], ["ttlDefault", "number"]])
            typeof c[l] === u && (this.settings[l] = c[l]);
          await this.setKv("settings", this.settings), this.emit("settings", this.settings);
        }
        return this.emit("chats"), n;
      }
      async exportBackup() {
        let e = JSON.stringify(await this.snapshot(!0));
        return jmSeal(this.sk, e);
      }
      async importBackup(e) {
        let a = await jmOpen(this.sk, e),
          n;
        try {
          n = JSON.parse(a);
        } catch {
          throw new Error("Backup corrompido.");
        }
        return this.mergeSnapshot(n, !0);
      }
      async sendSelf(e) {
        if (!this.pool) return !1;
        let a = Sd(this.sk, jt, JSON.stringify(e), [["p", this.pk]]),
          n = oi(a, this.sk, this.pk, { expiration: be() + 2 * Za });
        return (await this.pool.publish(n, [])).ok;
      }
      async onSelfApp(e) {
        if (!e || typeof e.t != "string") return;
        let a = await this.deviceId();
        if (e.dev === a) return;
        if (e.t === "sreq") return void this.syncPush(!0).catch(() => {});
        if (e.t === "dev" && typeof e.dev == "string" && e.dev.length <= 32) {
          if ((await this.kv("devicesOff", [])).includes(e.dev)) return;
          let n = await this.kv("devices", {});
          return (n[e.dev] = { n: ie(e.n, 40) || "Aparelho", p: ie(e.p, 20), last: Date.now() }), await this.setKv("devices", n), void this.emit("devices");
        }
        if (e.t === "devoff" && typeof e.dev == "string") {
          if (e.dev === a) return void this.emit("remoteLogout");
          let n = await this.kv("devices", {});
          return delete n[e.dev], await this.setKv("devices", n), void this.emit("devices");
        }
        if (e.t === "sync" && e.d) {
          let n = await this.mergeSnapshot(e.d, !1);
          n.contacts + n.groups > 0 && this.emit("synced", n);
        }
      }
      async announceDevice() {
        let e = (navigator.userAgent.match(/Android[^;]*;\s*([^;)]+?)(?:\s+Build|\))/) || [])[1] || "Celular";
        await this.sendSelf({ t: "dev", dev: await this.deviceId(), n: ie(e, 40), p: "Android" });
      }
      async devices() {
        let e = await this.kv("devices", {});
        return Object.entries(e)
          .map(([a, n]) => ({ id: a, ...n }))
          .sort((a, n) => (n.last || 0) - (a.last || 0));
      }
      async removeDevice(e) {
        let a = await this.kv("devices", {});
        delete a[e], await this.setKv("devices", a);
        let n = await this.kv("devicesOff", []);
        n.includes(e) || n.push(e), await this.setKv("devicesOff", n);
        let s = Sd(this.sk, jt, JSON.stringify({ t: "devoff", dev: e }), [["p", this.pk]]);
        await this.pool.publish(oi(s, this.sk, this.pk, { expiration: be() + 30 * Za }), []), this.emit("devices");
      }
      async setMyStatus(e) {
        (this.me.st = jmCleanStatus(e) || void 0), this.me.st || delete this.me.st, await this.setKv("me", this.me);
        for (let a of [...this.contacts.values()].filter((n) => n.status === "ok"))
          try {
            await this.sendProfile(a.pk);
          } catch {}
        this.setPresence("on", !0).catch(() => {});
      }
      statusOf(e) {
        let a = this.contacts.get(e);
        return a ? jmCleanStatus(a.st) : null;
      }
      /** Pede aos outros aparelhos da conta o que eles têm (uma vez por instalação). */
      async syncRequest() {
        (await this.kv("syncReq", !1)) || ((await this.sendSelf({ t: "sreq", dev: await this.deviceId() })) && (await this.setKv("syncReq", !0)));
      }
      /** Manda contatos, grupos e ajustes para os outros aparelhos, em pedaços. */
      async syncPush(e) {
        if (!this.pool || this.pool.openCount() === 0) return;
        let a = await this.snapshot(!1),
          n = JSON.stringify({ ...a, at: null }),
          s = Nt(new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(n)))).slice(0, 24);
        if (!e && (await this.kv("syncHash", "")) === s) return;
        if (!e && !a.contacts.length && !a.groups.length) return void (await this.setKv("syncHash", s));
        let o = { contacts: a.contacts, groups: a.groups, chats: a.chats },
          r = { ...a, contacts: [], groups: [], chats: [] },
          i = [],
          c = JSON.parse(JSON.stringify(r));
        for (let l of ["contacts", "groups", "chats"])
          for (let u of o[l]) {
            let d = JSON.stringify(u).length;
            d > 24e3 || (JSON.stringify(c).length + d > 24e3 && (i.push(c), (c = JSON.parse(JSON.stringify(r)))), c[l].push(u));
          }
        i.push(c);
        for (let l of i) if (!(await this.sendSelf({ t: "sync", dev: a.dev, d: l }))) return;
        await this.setKv("syncHash", s);
      }
      async deleteForAll(e, a) {
        let jmG = e.startsWith("g:") && this.groups.get(e.split(":")[1]);
        await this.applyDelete(e, a, this.pk, !!jmG && jmCan(jmG.meta, this.pk, "del")),
          await this.sendApp(e, { t: "del", ids: a }, { noTtl: !0 });
      }
      async deleteForMe(e, a) {
        for (let n of a) {
          let s = await this.db.get("msgs", n);
          !s ||
            s.chat !== e ||
            (s.media && (await this.db.del("media", s.media.mid)),
            await this.db.del("msgs", n),
            this.emit("msgDelete", e, n));
        }
        await this.refreshLast(e);
      }
      async refreshLast(e) {
        let a = this.chats.get(e);
        if (!a) return;
        let n = (await this.db.msgsByChat(e, 1 / 0, 1))[0];
        n
          ? ((a.lastTs = n.ts),
            (a.lastId = n.id),
            (a.lastFrom = n.from),
            (a.lastKind = n.kind),
            (a.lastText = this.preview(n)),
            (a.lastStatus = n.status))
          : ((a.lastText = ""), (a.lastId = null)),
          await this.saveChat(a);
      }
      async setTTL(e, a) {
        let n = this.chatFor(e);
        if (((n.ttl = a), await this.saveChat(n), n.kind === "dm")) {
          let s = await this.sendApp(e, { t: "ttl", v: a }, { noTtl: !0 });
          await this.addMsg(
            e,
            {
              id: s.id,
              from: this.pk,
              kind: "sys",
              sys: "ttl",
              v: a,
              ts: s.created_at * 1e3,
            },
            !1,
          );
        }
      }
      sendTyping(e) {
        if (!this.settings.typing || !e.startsWith("dm:")) return;
        let a = this.typingAt.get(e) || 0;
        ve() - a < 4e3 ||
          (this.typingAt.set(e, ve()),
          this.sendApp(e, { t: "typ" }, { ephemeral: !0, noTtl: !0 }).catch(
            () => {},
          ));
      }
      setActive(e, a) {
        (this.active = { chat: e, visible: a }), e && a && this.markRead(e);
      }
      async markRead(e) {
        let a = this.chats.get(e);
        if (!a) return;
        let n = a.unread || (a.pendingRead && a.pendingRead.length);
        if (
          a.kind === "dm" &&
          a.pendingRead &&
          a.pendingRead.length &&
          this.settings.readReceipts
        )
          for (let s of a.pendingRead) this.queueReceipt(a.pk, s, "r");
        (a.pendingRead = []), (a.unread = 0), n && (await this.saveChat(a));
      }
      async sendProfile(e) {
        if (!this.me) return;
        await this.sendApp(
          "dm:" + e,
          {
            t: "prof",
            n: this.me.name,
            s: this.me.note || "",
            c: this.me.t,
            p: this.me.pic || "",
            r: this.settings.relays.slice(0, 5),
            g: this.me.games || [],
            ...(jmCleanStatus(this.me.st) ? { st: jmCleanStatus(this.me.st) } : {}),
          },
          { selfCopy: !1, noTtl: !0 },
        );
        this.me.banner &&
          (await this.sendApp(
            "dm:" + e,
            { t: "pban", b: this.me.banner },
            { selfCopy: !1, noTtl: !0 },
          ));
        this.myListening &&
          this.sendApp("dm:" + e, { t: "lis", ...this.myListening.l }, { ephemeral: !0, selfCopy: !1, noTtl: !0 }).catch(() => {});
        let a = this.contacts.get(e);
        a &&
          !a.profSent &&
          ((a.profSent = !0), await this.db.put("contacts", a));
      }
      async updateProfile({ name: e, note: a, t: n, pic: s }) {
        if (
          (e !== void 0 && (this.me.name = ie(e, 40)),
          a !== void 0 && (this.me.note = ie(a, 80)),
          n !== void 0 && (this.me.t = n),
          s !== void 0)
        ) {
          let i = ma(s);
          i ? (this.me.pic = i) : i === "" && delete this.me.pic;
        }
        await this.setKv("me", this.me);
        let o = [...this.contacts.values()]
            .filter((i) => i.status === "ok")
            .map((i) => i.pk),
          r = [...this.groups.values()].filter((i) => !i.left);
        (async () => {
          for (let i of o)
            try {
              await this.sendProfile(i);
            } catch {}
          for (let i of r)
            try {
              await this.sendGroupApp(i, this.joinMsg());
            } catch {}
        })();
      }
      /** Banner e jogos favoritos; banner "" tira. Os contatos aceitos recebem. */
      async updateProfileExtras({ banner: e, games: a }) {
        let n = !!this.me.banner;
        if (e !== void 0) {
          let i = jmCleanBanner(e);
          if (i === void 0) throw new Error("Banner grande demais");
          i ? (this.me.banner = i) : delete this.me.banner;
        }
        a !== void 0 && (this.me.games = jmCleanGames(a)),
          await this.setKv("me", this.me);
        let s = n && !this.me.banner,
          o = [...this.contacts.values()].filter((i) => i.status === "ok").map((i) => i.pk);
        (async () => {
          for (let i of o)
            try {
              await this.sendProfile(i),
                s && (await this.sendApp("dm:" + i, { t: "pban", b: "" }, { selfCopy: !1, noTtl: !0 }));
            } catch {}
        })();
      }
      /** O que um contato está ouvindo (some sozinho se parar de avisar). */
      listeningOf(e) {
        let a = this.listening && this.listening.get(e);
        return a && jmListeningAlive(a.l, a.at, Date.now()) ? a.l : null;
      }
      /**
       * Conta aos contatos aceitos o que você está ouvindo (null: parou).
       * Só manda quando muda a música ou, a mesma, a cada 4 min.
       */
      async setListening(e) {
        let a = jmCleanListening(e),
          n = this.myListening,
          s = Date.now(),
          o = (c, l) => c.s === l.s && c.a === l.a && c.id === l.id,
          r = null;
        if (!n && !a) return;
        if (n && !a) (this.myListening = null), (r = { t: "lis", s: "" });
        else if (n && o(n.l, a) && s - n.at < JM_LIS_HEARTBEAT) return;
        else
          n && o(n.l, a) && (a.st = n.l.st),
            (this.myListening = { l: a, at: s }),
            (r = { t: "lis", ...a });
        this.emit("listening", this.pk);
        for (let c of [...this.contacts.values()].filter((l) => l.status === "ok"))
          try {
            await this.sendApp("dm:" + c.pk, r, { ephemeral: !0, selfCopy: !1, noTtl: !0 });
          } catch {}
      }
      /** Código curto de convite (XXXX-XXXX), vale 7 dias. Igual ao do PC. */
      async createShortCode() {
        let e = await this.kv("shortCode", null),
          a = be();
        (!e || e.exp - a < 3600) && (e = { code: jmShortGenerate(), exp: a + JM_SC_DIAS * Za });
        let n = ri(this.sk, {
          kind: JM_SC_KIND,
          tags: [
            ["d", "jmenseger:convite:" + e.code],
            ["expiration", String(e.exp)],
          ],
        });
        if (!(await this.pool.publish(n)).ok) throw new Error("Sem conexão com os servidores.");
        return await this.setKv("shortCode", e), e;
      }
      async shortCode() {
        let e = await this.kv("shortCode", null);
        return e && e.exp > be() ? e : null;
      }
      /** De quem é um código curto: devolve a chave pública (hex). */
      async resolveShortCode(e) {
        let a = jmShortParse(e);
        if (!a) throw new Error("C\xF3digo inv\xE1lido.");
        let n = "jmenseger:convite:" + a,
          s = "sc" + Math.random().toString(36).slice(2, 8),
          o = new Set(),
          r = be();
        await new Promise((c) => {
          let l = 0,
            u = (d, m, p) => {
              p === s &&
                d &&
                d.kind === JM_SC_KIND &&
                Wa(d, "d") === n &&
                !(Number(Wa(d, "expiration")) && Number(Wa(d, "expiration")) < r) &&
                Re.test(d.pubkey) &&
                mr(d) &&
                o.add(d.pubkey);
            },
            f = (d, m) => {
              m === s && ++l >= Math.max(1, this.pool.openCount()) && h();
            },
            h = () => {
              clearTimeout(g), x1(), x2(), this.pool.unsubscribe(s), c();
            },
            g = setTimeout(h, 6e3),
            x1 = this.pool.on("event", u),
            x2 = this.pool.on("eose", f);
          this.pool.subscribe(s, () => [{ kinds: [JM_SC_KIND], "#d": [n], limit: 5 }]);
        });
        if (o.size === 0) throw new Error("C\xF3digo n\xE3o encontrado ou vencido. Pe\xE7a um c\xF3digo novo.");
        if (o.size > 1) throw new Error("Esse c\xF3digo est\xE1 duplicado nos servidores. Pe\xE7a o c\xF3digo completo (npub1\u2026).");
        return [...o][0];
      }
      joinMsg() {
        return { t: "gjoin", n: this.me.name, p: this.me.pic || "" };
      }
      async addContact(e, a) {
        let n = to(e);
        if (!n)
          throw new Error("C\xF3digo inv\xE1lido. Confira e tente de novo.");
        if (n === this.pk)
          throw new Error("Esse \xE9 o seu pr\xF3prio c\xF3digo.");
        let s = await this.ensureContact(n, "ok");
        (s.status = "ok"),
          a && (s.nick = ie(a, 40)),
          await this.db.put("contacts", s),
          this.emit("contact", s);
        let o = this.chatFor("dm:" + n);
        return (
          !o.ttl &&
            this.settings.ttlDefault &&
            (o.ttl = this.settings.ttlDefault),
          o.lastTs || (o.lastTs = ve()),
          await this.saveChat(o),
          await this.sendProfile(n),
          s
        );
      }
      async acceptContact(e) {
        let a = this.contacts.get(e);
        if (!a) return;
        (a.status = "ok"),
          await this.db.put("contacts", a),
          this.emit("contact", a),
          this.emit("chats"),
          await this.sendProfile(e);
        let n = this.chats.get("dm:" + e);
        if (n && n.pendingRead)
          for (let s of n.pendingRead) this.queueReceipt(e, s, "d");
        await this.markRead("dm:" + e);
      }
      async setContactStatus(e, a) {
        let n = await this.ensureContact(e, a);
        (n.status = a),
          await this.db.put("contacts", n),
          this.emit("contact", n),
          this.emit("chats");
      }
      async updateContact(e, a) {
        let n = this.contacts.get(e);
        n &&
          (Object.assign(n, a),
          await this.db.put("contacts", n),
          this.emit("contact", n),
          this.emit("chats"));
      }
      async deleteChat(e) {
        let a = await this.db.delChat(e);
        for (let s of a) s.media && (await this.db.del("media", s.media.mid));
        let n = this.chats.get(e);
        n &&
          ((n.lastText = ""),
          (n.lastId = null),
          (n.unread = 0),
          (n.deleted = !0),
          await this.db.put("chats", n)),
          this.emit("chats");
      }
      chatList() {
        return [...this.chats.values()]
          .filter((e) => !e.deleted || e.unread)
          .sort(
            (e, a) =>
              (a.pinned ? 1 : 0) - (e.pinned ? 1 : 0) ||
              (a.lastTs || 0) - (e.lastTs || 0),
          );
      }
      messages(e, a, n) {
        return this.db.msgsByChat(e, a, n);
      }
      async sendMedia(e, a, n, s) {
        let jmArq = n.type === "file";
        if ((a instanceof Uint8Array || (a = new Uint8Array(a)), a.length > (jmArq ? JM_FILE_MAX : ci)))
          throw new Error(jmArq ? "Arquivo grande demais (at\xE9 5 MB)" : "Arquivo grande demais");
        if (!(jmArq ? JM_FILE_MIME : Ld).test(n.mime)) throw new Error("Formato n\xE3o suportado");
        await this.ensureDmContact(e);
        let o = Ka(16),
          r = Math.ceil(a.length / Xs),
          i = {
            t: "mh",
            mid: o,
            type: n.type,
            mime: n.mime,
            size: a.length,
            n: r,
            sha: In(a),
          };
        jmArq && (i.fn = jmFileName(n.name)),
          !jmArq && n.vo && !n.sb && (i.vo = 1),
          n.dur && (i.dur = Math.round(n.dur)),
          n.wf && (i.wf = n.wf.slice(0, 48)),
          n.w && ((i.w = n.w), (i.h = n.h)),
          n.th && n.th.length < 3e3 && !i.vo && (i.th = n.th),
          n.sb &&
            n.type === "audio" &&
            (i.sb = {
              n: ie(n.sb.n, 24) || "Som",
              e: ie(n.sb.e, 8) || "\u{1F50A}",
            });
        let c = new Blob([a], { type: n.mime }),
          l = this.build(e, jt, JSON.stringify(i), n.fwd ? [["fwd", "1"]] : []);
        await this.db.put("media", {
          mid: o,
          msgId: l.rumor.id,
          chat: e,
          blob: c,
          mime: n.mime,
        });
        let u = {
          mid: o,
          type: i.type,
          mime: i.mime,
          size: i.size,
          n: r,
          dur: i.dur,
          wf: i.wf,
          w: i.w,
          h: i.h,
          th: i.th,
          sb: i.sb,
          name: i.fn,
          vo: i.vo,
          got: r,
          ready: !0,
          sent: 0,
        };
        u.sb || delete u.sb, u.name || delete u.name, u.vo || delete u.vo;
        let d = await this.localSend(
          e,
          {
            id: l.rumor.id,
            from: this.pk,
            kind: "media",
            media: u,
            fwd: n.fwd ? !0 : void 0,
            ts: l.rumor.created_at * 1e3,
          },
          l,
        );
        for (let f = 0; f < r; f++) {
          let p = a.subarray(f * Xs, (f + 1) * Xs),
            v = this.build(
              e,
              jt,
              JSON.stringify({ t: "mc", mid: o, i: f, d: Fa.enc(p) }),
              [],
              { selfCopy: !1 },
            );
          await this.publish(v.rumor.id, v.wraps, v.relays),
            s && s(f + 1, r),
            this.emit("progress", l.rumor.id, f + 1, r),
            await Ed(40);
        }
        return d;
      }
      async onMediaHeader(e, a, n, s, o) {
        if (typeof n.mid != "string" || !/^[0-9a-f]{32}$/.test(n.mid)) return;
        let r = parseInt(n.n, 10),
          i = parseInt(n.size, 10),
          jmArq = n.type === "file",
          jmLim = jmArq ? JM_FILE_MAX : ci;
        if (
          !(r > 0 && r <= Math.ceil(jmLim / Xs)) ||
          !(i > 0 && i <= jmLim) ||
          !(jmArq ? JM_FILE_MIME : Ld).test(n.mime || "") ||
          (n.type !== "audio" && n.type !== "image" && !jmArq)
        )
          return;
        !o && e.startsWith("dm:") && (await this.ensureContact(s, "request"));
        let c = {
          mid: n.mid,
          type: n.type,
          mime: String(n.mime),
          size: i,
          n: r,
          sha: typeof n.sha == "string" ? n.sha : "",
          got: 0,
          ready: !1,
        };
        jmArq && (c.name = jmFileName(n.fn)), !jmArq && n.vo === 1 && (c.vo = 1);
        n.dur && (c.dur = Math.min(3600, parseInt(n.dur, 10) || 0)),
          Array.isArray(n.wf) &&
            (c.wf = n.wf
              .slice(0, 48)
              .map((d) => Math.max(0, Math.min(31, d | 0)))),
          n.w &&
            ((c.w = Math.min(1e4, n.w | 0)), (c.h = Math.min(1e4, n.h | 0))),
          typeof n.th == "string" &&
            /^data:image\/(webp|jpeg|png);base64,/.test(n.th) &&
            n.th.length < 3e3 &&
            (c.th = n.th),
          n.type === "audio" &&
            n.sb &&
            typeof n.sb == "object" &&
            (c.sb = {
              n: ie(n.sb.n, 24) || "Som",
              e: ie(n.sb.e, 8) || "\u{1F50A}",
            });
        let l = await this.db.get("media", n.mid);
        l && l.blob
          ? ((c.ready = !0), (c.got = r))
          : await this.db.put("media", { mid: n.mid, msgId: a.id, chat: e }),
          (await this.addMsg(
            e,
            {
              id: a.id,
              from: s,
              kind: "media",
              media: c,
              fwd: Wa(a, "fwd") ? !0 : void 0,
              ts: a.created_at * 1e3,
              exp: this.expOf(a),
              status: o ? "sent" : void 0,
            },
            !o,
          )) &&
            !c.ready &&
            (await this.tryAssemble(n.mid));
      }
      async onChunk(e) {
        if (
          typeof e.mid != "string" ||
          !/^[0-9a-f]{32}$/.test(e.mid) ||
          !Number.isInteger(e.i) ||
          e.i < 0 ||
          e.i >= Math.ceil(JM_FILE_MAX / Xs) ||
          typeof e.d != "string" ||
          e.d.length > 3e4
        )
          return;
        let a = await this.db.get("media", e.mid);
        (a && a.blob) ||
          (await this.db.put("chunks", {
            k: e.mid + ":" + e.i,
            mid: e.mid,
            i: e.i,
            d: e.d,
          }),
          await this.tryAssemble(e.mid));
      }
      async tryAssemble(e) {
        let a = await this.db.get("media", e);
        if (!a || a.blob || !a.msgId) return;
        let n = await this.db.get("msgs", a.msgId);
        if (!n || !n.media) return;
        let s = await this.db.chunksByMid(e),
          o = n.media.n;
        if (((n.media.got = Math.min(o, s.length)), s.length < o)) {
          await this.db.put("msgs", n), this.emit("msgUpdate", n.chat, n);
          return;
        }
        s.sort((u, d) => u.i - d.i);
        let r = s.slice(0, o).map((u) => Fa.dec(u.d)),
          i = r.reduce((u, d) => u + d.length, 0),
          c = new Uint8Array(i),
          l = 0;
        for (let u of r) c.set(u, l), (l += u.length);
        if (n.media.sha && In(c) !== n.media.sha) {
          (n.media.err = !0),
            await this.db.put("msgs", n),
            this.emit("msgUpdate", n.chat, n);
          return;
        }
        await this.db.put("media", {
          mid: e,
          msgId: n.id,
          chat: n.chat,
          blob: new Blob([c], { type: n.media.mime }),
          mime: n.media.mime,
        });
        for (let u of s) await this.db.del("chunks", u.k);
        (n.media.ready = !0),
          (n.media.got = o),
          await this.db.put("msgs", n),
          this.emit("msgUpdate", n.chat, n);
      }
      async mediaBlob(e) {
        let a = await this.db.get("media", e);
        return a && a.blob ? a.blob : null;
      }
      async purgeExpired() {
        let e;
        try {
          e = await this.db.msgsExpired(ve());
        } catch {
          return;
        }
        let a = new Set();
        for (let n of e)
          n.media && (await this.db.del("media", n.media.mid)),
            await this.db.del("msgs", n.id),
            a.add(n.chat),
            this.emit("msgDelete", n.chat, n.id);
        for (let n of a) await this.refreshLast(n);
      }
      cleanMeta(e, a) {
        if (!e || typeof e != "object") return null;
        let n = (Array.isArray(e.channels) ? e.channels : [])
          .slice(0, 20)
          .filter(
            (i) =>
              i && Ah.test(i.id) && (i.type === "text" || i.type === "voice"),
          )
          .map((i) => {
            let c = { id: i.id, name: ie(i.name, 24) || i.id, type: i.type },
              l = ie(i.topic, 80);
            return l && (c.topic = l), c;
          });
        if (!n.some((i) => i.type === "text")) return null;
        let s = (Array.isArray(e.members) ? e.members : [])
            .filter((i) => Re.test(i))
            .slice(0, 64),
          o = Re.test(e.owner) ? e.owner : a,
          r = {};
        if (e.names && typeof e.names == "object")
          for (let [i, c] of Object.entries(e.names))
            Re.test(i) && (r[i] = ie(c, 40));
        return {
          lid: Re.test(e.lid) ? e.lid : void 0,
          name: ie(e.name, 40) || "Grupo",
          desc: ie(e.desc, 140),
          t: Number.isInteger(e.t) ? e.t % 6 : 0,
          channels: n,
          owner: o,
          admins: (Array.isArray(e.admins) ? e.admins : [])
            .filter((i) => Re.test(i))
            .slice(0, 10),
          members: s,
          names: r,
          ...jmCleanRoles(e, s),
        };
      }
      parseInvite(e) {
        if (!e || typeof e.sk != "string" || !Re.test(e.sk)) return null;
        let a = this.cleanMeta(e.meta);
        if (!a) return null;
        let n = Ke(_t(e.sk));
        return {
          sk: e.sk,
          pk: n,
          lid: a.lid || n,
          meta: a,
          ver: parseInt(e.ver, 10) || 1,
          pic: ma(e.pic) || "",
          picV: Number(e.picV) || 0,
        };
      }
      async saveGroup(e) {
        await this.db.put("groups", e), this.emit("group", e);
      }
      async createGroup(e, a = "amigos", n = "") {
        let s = Bd[a] || Bd.amigos,
          o = kn(),
          r = Ke(o),
          i = new Set(),
          c = (d, f) => {
            let p = Md(d);
            for (; i.has(p); ) p += "-2";
            return i.add(p), { id: p, name: d, type: f };
          },
          l = {
            lid: r,
            name: ie(e, 40) || "Grupo",
            desc: ie(n, 140),
            t: Math.floor(Math.random() * 6),
            channels: [
              ...s.text.map((d) => c(d, "text")),
              ...s.voice.map((d) => c(d, "voice")),
            ],
            owner: this.pk,
            admins: [this.pk],
            members: [this.pk],
            names: { [this.pk]: this.me.name },
          },
          u = {
            lid: r,
            pk: r,
            sk: Nt(o),
            oldKeys: [],
            meta: l,
            ver: 1,
            joinedAt: ve(),
          };
        return (
          this.groups.set(u.lid, u),
          await this.saveGroup(u),
          this.resubscribe(),
          await this.sysGroup(u, "created", this.pk, be()),
          u
        );
      }
      async sysGroup(e, a, n, s, o) {
        let r = e.meta.channels.find((i) => i.type === "text");
        r &&
          (await this.addMsg(
            `g:${e.lid}:${r.id}`,
            { id: o || Ka(32), from: n, kind: "sys", sys: a, ts: s * 1e3 },
            !1,
          ));
      }
      async sendGroupApp(e, a, n = {}) {
        let s = e.meta.channels.find((o) => o.type === "text");
        return this.sendApp(`g:${e.lid}:${s.id}`, a, { noTtl: !0, ...n });
      }
      async publishMeta(e) {
        await this.sendGroupApp(e, { t: "gmeta", meta: e.meta, ver: e.ver });
      }
      async inviteToGroup(e, a) {
        let n = this.groups.get(e);
        if (!n) return;
        let s = await this.ensureContact(a, "ok");
        s.status !== "ok" &&
          ((s.status = "ok"), await this.db.put("contacts", s)),
          s.profSent || (await this.sendProfile(a));
        let o = {
            t: "ginv",
            g: {
              sk: n.sk,
              meta: n.meta,
              ver: n.ver,
              pic: n.pic || "",
              picV: n.picV || 0,
            },
          },
          r = this.build("dm:" + a, jt, JSON.stringify(o), [], { noTtl: !0 }),
          i = {
            sk: n.sk,
            pk: n.pk,
            lid: n.lid,
            meta: n.meta,
            ver: n.ver,
            pic: n.pic || "",
            picV: n.picV || 0,
          };
        return this.localSend(
          "dm:" + a,
          {
            id: r.rumor.id,
            from: this.pk,
            kind: "invite",
            inv: i,
            ts: r.rumor.created_at * 1e3,
          },
          r,
        );
      }
      async acceptInvite(e) {
        let a = await this.db.get("msgs", e);
        if (!a || !a.inv) return null;
        let n = a.inv,
          s = this.groups.get(n.lid);
        if (s && !s.left && s.pk === n.pk) return s;
        let o = this.cleanMeta(n.meta);
        return (
          o.members.includes(this.pk) || o.members.push(this.pk),
          (o.names[this.pk] = this.me.name),
          (s = {
            lid: n.lid,
            pk: n.pk,
            sk: n.sk,
            oldKeys: [],
            meta: o,
            ver: n.ver,
            joinedAt: ve(),
            pic: ma(n.pic) || "",
            picV: Number(n.picV) || 0,
          }),
          this.groups.set(s.lid, s),
          await this.saveGroup(s),
          this.resubscribe(),
          (a.inv.joined = !0),
          await this.db.put("msgs", a),
          this.emit("msgUpdate", a.chat, a),
          await this.sysGroup(s, "joined", this.pk, be()),
          await this.sendGroupApp(s, this.joinMsg()),
          s
        );
      }
      async leaveGroup(e) {
        let a = this.groups.get(e);
        if (a) {
          try {
            await this.sendGroupApp(a, { t: "gleave" });
          } catch {}
          (a.left = !0), await this.saveGroup(a);
          for (let n of a.meta.channels) {
            if (n.type !== "text") continue;
            let s = `g:${e}:${n.id}`,
              o = await this.db.delChat(s);
            for (let r of o)
              r.media && (await this.db.del("media", r.media.mid));
            this.chats.delete(s), await this.db.del("chats", s);
          }
          this.groups.delete(e),
            await this.db.del("groups", e),
            this.resubscribe(),
            this.emit("group", null);
        }
      }
      async editGroup(e, a) {
        let n = this.groups.get(e);
        if (!n) return;
        if (
          (a.name !== void 0 && (n.meta.name = ie(a.name, 40) || n.meta.name),
          a.desc !== void 0 && (n.meta.desc = ie(a.desc, 140)),
          a.addChannel)
        ) {
          let { name: o, type: r } = a.addChannel,
            i = Md(o);
          for (; n.meta.channels.some((c) => c.id === i); ) i += "-2";
          n.meta.channels.push({
            id: i,
            name: ie(o, 24),
            type: r === "voice" ? "voice" : "text",
          });
        }
        if (a.editChannel) {
          let o = n.meta.channels.find((r) => r.id === a.editChannel.id);
          if (
            o &&
            (a.editChannel.name !== void 0 &&
              (o.name = ie(a.editChannel.name, 24) || o.name),
            a.editChannel.topic !== void 0)
          ) {
            let r = ie(a.editChannel.topic, 80);
            r ? (o.topic = r) : delete o.topic;
          }
        }
        if (a.moveChannel) {
          let o = n.meta.channels,
            r = o.findIndex((c) => c.id === a.moveChannel.id),
            i = a.moveChannel.dir < 0 ? -1 : 1;
          if (r >= 0) {
            let c = r + i;
            for (; c >= 0 && c < o.length && o[c].type !== o[r].type; ) c += i;
            c >= 0 && c < o.length && ([o[r], o[c]] = [o[c], o[r]]);
          }
        }
        let s = [];
        if (a.removeChannel) {
          let o = n.meta.channels.find((i) => i.id === a.removeChannel),
            r =
              o &&
              o.type === "text" &&
              n.meta.channels.filter((i) => i.type === "text").length === 1;
          o &&
            !r &&
            ((n.meta.channels = n.meta.channels.filter((i) => i !== o)),
            s.push(o));
        }
        if (a.admin) {
          let { pk: o, on: r } = a.admin;
          n.meta.admins = r
            ? [...new Set([...(n.meta.admins || []), o])]
            : (n.meta.admins || []).filter(
                (i) => i !== o && i !== n.meta.owner,
              );
        }
        if (a.addRole) {
          let o = (n.meta.roles = n.meta.roles || []);
          if (o.length < 12) {
            let r = { id: Ka(4).slice(0, 8), n: ie(a.addRole.n, 24) || "Cargo", c: (a.addRole.c | 0) % 8 },
              i = (a.addRole.p || []).filter((c) => JM_PERMS.some(([l]) => l === c));
            i.length && (r.p = i), o.push(r);
          }
        }
        if (a.editRole) {
          let o = (n.meta.roles || []).find((r) => r.id === a.editRole.id);
          o &&
            (a.editRole.n !== void 0 && (o.n = ie(a.editRole.n, 24) || o.n),
            a.editRole.c !== void 0 && (o.c = (a.editRole.c | 0) % 8),
          Array.isArray(a.editRole.p) && (a.editRole.p.length ? (o.p = a.editRole.p.filter((r) => JM_PERMS.some(([i]) => i === r))) : delete o.p));
        }
        if (a.removeRole) {
          n.meta.roles = (n.meta.roles || []).filter((o) => o.id !== a.removeRole);
          for (let o of Object.keys(n.meta.mroles || {}))
            (n.meta.mroles[o] = n.meta.mroles[o].filter((r) => r !== a.removeRole)),
              n.meta.mroles[o].length || delete n.meta.mroles[o];
        }
        if (a.memberRole) {
          let { pk: o, id: r, on: i } = a.memberRole,
            c = (n.meta.mroles = n.meta.mroles || {}),
            l = (c[o] || []).filter((u) => u !== r);
          i && (n.meta.roles || []).some((u) => u.id === r) && l.length < 4 && l.push(r),
            l.length ? (c[o] = l) : delete c[o];
        }
        n.ver++, await this.saveGroup(n), await this.publishMeta(n);
        for (let o of s)
          o.type === "text" && (await this.purgeChannel(n, o.id));
      }
      async purgeChannel(e, a) {
        (e.gone = [...new Set([...(e.gone || []), a])].slice(-50)),
          await this.db.put("groups", e);
        let n = `g:${e.lid}:${a}`,
          s = await this.db.delChat(n);
        for (let o of s) o.media && (await this.db.del("media", o.media.mid));
        this.chats.delete(n),
          await this.db.del("chats", n),
          this.emit("chatGone", n),
          this.emit("chats");
      }
      async setGroupPic(e, a) {
        let n = this.groups.get(e),
          s = ma(a);
        return !n ||
          s === void 0 ||
          !(n.meta.owner === this.pk || (n.meta.admins || []).includes(this.pk))
          ? !1
          : ((n.pic = s),
            (n.picV = Math.max(ve(), (n.picV || 0) + 1)),
            await this.saveGroup(n),
            await this.sendGroupApp(n, { t: "gpic", p: s, v: n.picV }),
            await this.sysGroup(n, s ? "gpic" : "gpicdel", this.pk, be()),
            !0);
      }
      async removeMember(e, a) {
        let n = this.groups.get(e);
        if (!n || n.meta.owner !== this.pk || a === this.pk) return;
        let s = kn(),
          o = Ke(s);
        n.oldKeys = [...(n.oldKeys || []), { pk: n.pk, sk: n.sk }].slice(-5);
        let r = n.pk;
        (n.pk = o),
          (n.sk = Nt(s)),
          (n.meta.members = n.meta.members.filter((i) => i !== a)),
          (n.meta.admins = (n.meta.admins || []).filter((i) => i !== a)),
          n.ver++,
          await this.saveGroup(n),
          this.resubscribe();
        for (let i of n.meta.members)
          i !== this.pk &&
            (await this.ensureContact(i, "ok"),
            await this.sendApp(
              "dm:" + i,
              {
                t: "grekey",
                lid: n.lid,
                old: r,
                sk: n.sk,
                meta: n.meta,
                ver: n.ver,
              },
              { selfCopy: !1, noTtl: !0 },
            ));
        await this.sysGroup(n, "removed", a, be());
      }
      async applyRekey(e, a) {
        if (
          typeof a.sk != "string" ||
          !Re.test(a.sk) ||
          typeof a.old != "string"
        )
          return;
        let n = [...this.groups.values()].find(
          (o) => o.pk === a.old || o.lid === a.lid,
        );
        if (!n || n.meta.owner !== e) return;
        let s = this.cleanMeta(a.meta, n.meta.owner);
        s &&
          ((n.oldKeys = [...(n.oldKeys || []), { pk: n.pk, sk: n.sk }].slice(
            -5,
          )),
          (n.pk = Ke(_t(a.sk))),
          (n.sk = a.sk),
          (n.meta = s),
          (n.ver = parseInt(a.ver, 10) || n.ver + 1),
          await this.saveGroup(n),
          this.resubscribe());
      }
      groupChats(e) {
        let a = this.groups.get(e);
        return a
          ? a.meta.channels
              .filter((n) => n.type === "text")
              .map((n) => this.chatFor(`g:${e}:${n.id}`))
          : [];
      }
      groupUnread(e) {
        return this.groupChats(e).reduce((a, n) => a + (n.unread || 0), 0);
      }
      roomKey(e, a) {
        return e + ":" + a;
      }
      onRoom(e, a, n, s) {
        if (!e.meta.channels.some((c) => c.id === a && c.type === "voice"))
          return;
        let o = this.roomKey(e.lid, a),
          r = this.rooms.get(o);
        r || ((r = new Map()), this.rooms.set(o, r));
        let i = r.has(n);
        s === "leave" ? r.delete(n) : r.set(n, ve()),
          i !== (s !== "leave")
            ? this.emit("room", e.lid, a, [...r.keys()])
            : this.emit("roomPing", e.lid, a, n);
      }
      roomSweep() {
        let e = ve();
        for (let [a, n] of this.rooms) {
          let s = !1;
          for (let [o, r] of n) e - r > 26e3 && (n.delete(o), (s = !0));
          if (s) {
            let [o, r] = a.split(":");
            this.emit("room", o, r, [...n.keys()]);
          }
        }
      }
      roomMembers(e, a) {
        return [...(this.rooms.get(this.roomKey(e, a)) || new Map()).keys()];
      }
      roomPresence(e, a, n) {
        let s = this.groups.get(e);
        if (s)
          return this.sendGroupApp(
            s,
            { t: "room", ch: a, a: n },
            { ephemeral: !0 },
          );
      }
      signal(e, a) {
        return this.sendApp(
          "dm:" + e,
          { t: "call", ...a },
          { ephemeral: !0, selfCopy: !1, noTtl: !0 },
        );
      }
      async missedCall(e, a) {
        await this.sendApp(
          "dm:" + e,
          { t: "call", a: "missed", v: a ? 1 : 0 },
          { selfCopy: !1, noTtl: !0, expiresIn: 7 * Za },
        );
      }
      async setSetting(e, a) {
        (this.settings[e] = a),
          await this.setKv("settings", this.settings),
          e === "relays" &&
            this.pool &&
            (this.pool.setRelays(a),
            this.pool.refresh("inbox"),
            this.publishRelayList()),
          this.emit("settings", this.settings);
      }
      async wipe() {
        this.stop(),
          await this.secure.del("sk"),
          await this.secure.del("words"),
          await this.db.wipe();
      }
    };
  var Lh = {
    kv: { keyPath: "k" },
    contacts: { keyPath: "pk" },
    groups: { keyPath: "lid" },
    chats: { keyPath: "key" },
    msgs: {
      keyPath: "id",
      indexes: [
        ["chat_ts", ["chat", "ts"]],
        ["exp", "exp"],
      ],
    },
    media: { keyPath: "mid" },
    chunks: { keyPath: "k", indexes: [["mid", "mid"]] },
    calls: { keyPath: "id", indexes: [["ts", "ts"]] },
    outbox: { keyPath: "id" },
    sounds: { keyPath: "id" },
  };
  function ba(t) {
    return new Promise((e, a) => {
      (t.onsuccess = () => e(t.result)), (t.onerror = () => a(t.error));
    });
  }
  async function Id(t = "jmenseger") {
    let e = await new Promise((n, s) => {
        let o = indexedDB.open(t, 2);
        (o.onupgradeneeded = () => {
          let r = o.result;
          for (let [i, c] of Object.entries(Lh)) {
            if (r.objectStoreNames.contains(i)) continue;
            let l = r.createObjectStore(i, { keyPath: c.keyPath });
            for (let [u, d] of c.indexes || []) l.createIndex(u, d);
          }
        }),
          (o.onsuccess = () => n(o.result)),
          (o.onerror = () => s(o.error));
      }),
      a = (n, s) => e.transaction(n, s).objectStore(n);
    return {
      get: (n, s) => ba(a(n, "readonly").get(s)),
      put: (n, s) => ba(a(n, "readwrite").put(s)),
      del: (n, s) => ba(a(n, "readwrite").delete(s)),
      all: (n) => ba(a(n, "readonly").getAll()),
      clearStore: (n) => ba(a(n, "readwrite").clear()),
      async msgsByChat(n, s = 1 / 0, o = 60) {
        let r = a("msgs", "readonly").index("chat_ts"),
          i = IDBKeyRange.bound([n, -1 / 0], [n, s], !1, !0),
          c = [];
        return (
          await new Promise((l, u) => {
            let d = r.openCursor(i, "prev");
            (d.onsuccess = () => {
              let f = d.result;
              if (!f || c.length >= o) return l();
              c.push(f.value), f.continue();
            }),
              (d.onerror = () => u(d.error));
          }),
          c.reverse()
        );
      },
      async msgsExpired(n) {
        let s = a("msgs", "readonly").index("exp");
        return ba(s.getAll(IDBKeyRange.upperBound(n)));
      },
      chunksByMid: (n) => ba(a("chunks", "readonly").index("mid").getAll(n)),
      async delChat(n) {
        let s = await this.msgsByChat(n, 1 / 0, 1e9),
          o = a("msgs", "readwrite");
        for (let r of s) o.delete(r.id);
        return s;
      },
      async wipe() {
        e.close(),
          await new Promise((n) => {
            let s = indexedDB.deleteDatabase(t);
            s.onsuccess = s.onerror = s.onblocked = () => n();
          });
      },
    };
  }
  var Ie = typeof window < "u" ? window.JMNative : null,
    me = (t, ...e) => {
      try {
        return Ie && typeof Ie[t] == "function" ? Ie[t](...e) : void 0;
      } catch (a) {
        console.warn("[native]", t, a);
        return;
      }
    },
    X = {
      available: !!Ie,
      version() {
        return me("version") || "web";
      },
      async secureGet(t) {
        if (Ie) return me("secureGet", t) || null;
        try {
          return localStorage.getItem("jm.sec." + t);
        } catch {
          return null;
        }
      },
      async secureSet(t, e) {
        if (Ie) return me("secureSet", t, e);
        try {
          localStorage.setItem("jm.sec." + t, e);
        } catch {}
      },
      async secureDel(t) {
        if (Ie) return me("secureDel", t);
        try {
          localStorage.removeItem("jm.sec." + t);
        } catch {}
      },
      notify(t) {
        if (Ie) return me("notify", JSON.stringify(t));
        try {
          typeof Notification < "u" &&
            Notification.permission === "granted" &&
            new Notification(t.title, { body: t.body || "", tag: t.tag });
        } catch {}
      },
      cancelNotify(t) {
        me("cancelNotify", t);
      },
      updateCheck() {
        me("updateCheck");
      },
      /** A tela está aberta? (no navegador, sempre) */
      foreground() {
        return !Ie || me("foreground") !== !1;
      },
      /** Grava um arquivo (base64) na pasta Download; devolve onde ficou, "" se falhou, undefined no navegador. */
      linkPreview(t, e) {
        me("linkPreview", t, e);
      },
      screenStart() {
        me("screenStart");
      },
      screenStop() {
        me("screenStop");
      },
      saveFile(t, e) {
        return Ie ? me("saveFile", t, e) || "" : void 0;
      },
      updateDownload() {
        me("updateDownload");
      },
      incomingCall(t) {
        me("incomingCall", JSON.stringify(t));
      },
      cancelIncoming() {
        me("cancelIncoming");
      },
      notificationsAllowed() {
        return Ie
          ? !!me("notificationsAllowed")
          : typeof Notification < "u" && Notification.permission === "granted";
      },
      requestNotifications() {
        if (Ie) return me("requestNotifications");
        try {
          typeof Notification < "u" && Notification.requestPermission();
        } catch {}
      },
      setBackground(t) {
        me("setBackground", !!t);
      },
      callMode(t, e) {
        me("callMode", !!t, !!e);
      },
      setSpeaker(t) {
        me("setSpeaker", !!t);
      },
      setSecure(t) {
        me("setSecure", !!t);
      },
      keepScreen(t) {
        me("keepScreen", !!t);
      },
      vibrate(t) {
        if (Ie) return me("vibrate", t | 0);
        try {
          navigator.vibrate && navigator.vibrate(t);
        } catch {}
      },
      batterySettings() {
        me("batterySettings");
      },
      appSettings() {
        me("appSettings");
      },
      setBars(t, e) {
        me("setBars", t, !!e);
      },
      share(t) {
        return Ie
          ? me("share", t)
          : navigator.share
            ? navigator.share({ text: t }).catch(() => {})
            : this.copy(t);
      },
      async copy(t) {
        if (Ie) return me("copy", t), !0;
        try {
          return await navigator.clipboard.writeText(t), !0;
        } catch {
          return !1;
        }
      },
      paste() {
        return (Ie && me("paste")) || "";
      },
      openUrl(t) {
        if (Ie) return me("openUrl", t);
        window.open(t, "_blank", "noopener");
      },
      exitApp() {
        me("moveToBack");
      },
    };
  var y = (t, e = document) => e.querySelector(t),
    oe = (t, e = document) => [...e.querySelectorAll(t)],
    M = (t) =>
      String(t ?? "").replace(
        /[&<>"']/g,
        (e) =>
          ({
            "&": "&amp;",
            "<": "&lt;",
            ">": "&gt;",
            '"': "&quot;",
            "'": "&#39;",
          })[e],
      ),
    x = (t, e = "") =>
      `<svg class="i ${e}" aria-hidden="true"><use href="#i-${t}"/></svg>`,
    Dd = (t) =>
      String(t || "")
        .trim()
        .split(/\s+/)
        .slice(0, 2)
        .map((e) => e[0] || "")
        .join("")
        .toUpperCase() || "?",
    Ja = (t) =>
      String(t || "")
        .toLowerCase()
        .normalize("NFD")
        .replace(/[̀-ͯ]/g, ""),
    qe = (
      t,
      e,
    ) => `<svg class="logo ${e ? "anim" : ""}" viewBox="0 0 64 64" width="${t}" height="${t}" aria-hidden="true">
  <rect class="lt" width="64" height="64" rx="19" style="fill:var(--accent)"/>
  <path class="lb" d="M21 14H43A8 8 0 0 1 51 22V34A8 8 0 0 1 43 42H33.5L24 49.6C22.8 50.5 20.5 49.9 20.5 48.4V42A8 8 0 0 1 13 34V22A8 8 0 0 1 21 14Z" style="fill:var(--accent-ink)"/>
  <path class="lj" pathLength="1" d="M26 21H39M33 21V30.5A5.5 5.5 0 0 1 27.5 36H26.5" style="fill:none;stroke:var(--accent);stroke-width:5;stroke-linecap:round;stroke-linejoin:round"/></svg>`,
    Th = /^data:image\/(webp|jpeg|png);base64,[A-Za-z0-9+/]+={0,2}$/;
  function et({
    name: t,
    t: e = 0,
    size: a = 52,
    sq: n = !1,
    verified: s = !1,
    ring: o,
    pic: r,
  }) {
    let i = s ? `<i class="vbadge">${x("check")}</i>` : "",
      c =
        r && Th.test(r)
          ? `<img src="${r}" alt="" decoding="async" draggable="false">`
          : "";
    return `<span class="av ${n ? "sq " : ""}${c ? "pic " : ""}t${(e | 0) % 6}" style="--sz:${a}px${o ? ";--ring:" + o : ""}">${c || M(Dd(t))}${i}</span>`;
  }
  var Nd = [
      "domingo",
      "segunda-feira",
      "ter\xE7a-feira",
      "quarta-feira",
      "quinta-feira",
      "sexta-feira",
      "s\xE1bado",
    ],
    Bh = [
      "janeiro",
      "fevereiro",
      "mar\xE7o",
      "abril",
      "maio",
      "junho",
      "julho",
      "agosto",
      "setembro",
      "outubro",
      "novembro",
      "dezembro",
    ],
    no = (t) => {
      let e = new Date(t);
      return e.setHours(0, 0, 0, 0), e.getTime();
    },
    so = (t) => {
      let e = new Date(t);
      return (
        String(e.getHours()).padStart(2, "0") +
        ":" +
        String(e.getMinutes()).padStart(2, "0")
      );
    };
  function Xa(t) {
    if (!t) return "";
    let e = no(Date.now()),
      a = no(t);
    if (a === e) return so(t);
    if (a === e - 864e5) return "Ontem";
    if (e - a < 6 * 864e5) {
      let s = Nd[new Date(t).getDay()];
      return s.charAt(0).toUpperCase() + s.slice(1, 3);
    }
    let n = new Date(t);
    return (
      String(n.getDate()).padStart(2, "0") +
      "/" +
      String(n.getMonth() + 1).padStart(2, "0") +
      "/" +
      String(n.getFullYear()).slice(2)
    );
  }
  function _d(t) {
    let e = no(Date.now()),
      a = no(t);
    if (a === e) return "Hoje";
    if (a === e - 864e5) return "Ontem";
    let n = new Date(t);
    if (e - a < 6 * 864e5) {
      let s = Nd[n.getDay()];
      return s.charAt(0).toUpperCase() + s.slice(1);
    }
    return `${n.getDate()} de ${Bh[n.getMonth()]}${n.getFullYear() !== new Date().getFullYear() ? " de " + n.getFullYear() : ""}`;
  }
  var xa = (t) => (
      (t = Math.max(0, Math.round(t))),
      Math.floor(t / 60) + ":" + String(t % 60).padStart(2, "0")
    ),
    Un = (t) => (
      (t = Math.max(0, Math.floor(t))),
      String(Math.floor(t / 60)).padStart(2, "0") +
        ":" +
        String(t % 60).padStart(2, "0")
    );
  function yt(t) {
    return t
      ? t < 3600
        ? Math.round(t / 60) + " min"
        : t < 86400
          ? Math.round(t / 3600) + (t < 7200 ? " hora" : " horas")
          : Math.round(t / 86400) + (t < 172800 ? " dia" : " dias")
      : "Desligadas";
  }
  function Od(t) {
    return M(t).replace(
      /\bhttps?:\/\/[^\s<]+[^\s<.,;:!?)\]'"]/gi,
      (e) => `<a class="lnk" data-act="openUrl" data-u="${e}">${e}</a>`,
    );
  }
  var jd = (t) => {
      let e = String(t || "").trim();
      return (
        e.length <= 12 &&
        new RegExp(
          "^(\\p{Extended_Pictographic}|\\p{Emoji_Component}|\u200D|\uFE0F|\\s)+$",
          "u",
        ).test(e) &&
        new RegExp("\\p{Extended_Pictographic}", "u").test(e)
      );
    },
    vt = {},
    b = (t, e) => {
      vt[t] = e;
    },
    Hd;
  function _(t, e = 2400) {
    let a = y("#toast");
    a &&
      ((a.textContent = t),
      a.classList.add("show"),
      clearTimeout(Hd),
      (Hd = setTimeout(() => a.classList.remove("show"), e)));
  }
  var ga = null;
  function ee(t, e = {}) {
    let a = y("#sheet");
    if (ga) {
      let n = ga;
      ga = null;
      try {
        n();
      } catch {}
    }
    return (
      (a.innerHTML = '<div class="grab"></div>' + t),
      (a.scrollTop = 0),
      y("#device").classList.add("sheet-open"),
      (ga = e.onClose || null),
      a
    );
  }
  function F() {
    if ((y("#device").classList.remove("sheet-open"), ga)) {
      let t = ga;
      ga = null;
      try {
        t();
      } catch {}
    }
  }
  var di = () => y("#device").classList.contains("sheet-open");
  function ge({
    title: t,
    text: e,
    ok: a = "Confirmar",
    cancel: n = "Cancelar",
    danger: s = !1,
  }) {
    return new Promise((o) => {
      let r = !1,
        i = (c) => {
          r || ((r = !0), o(c));
        };
      ee(
        `<h2 class="sh-title">${M(t)}</h2>${e ? `<p class="sh-sub">${M(e)}</p>` : ""}
      <div class="two" style="margin-top:20px"><button class="btn ghost" data-act="cfNo" style="margin:0">${M(n)}</button><button class="btn ${s ? "danger" : ""}" data-act="cfYes" style="margin:0">${M(a)}</button></div>`,
        { onClose: () => i(!1) },
      ),
        b("cfYes", () => {
          i(!0), F();
        }),
        b("cfNo", () => {
          i(!1), F();
        });
    });
  }
  var He = [],
    Mh = 10;
  function Ud() {
    let t = [y("#screen-main"), ...He.map((e) => e.el)];
    t.forEach((e, a) => e.classList.toggle("behind", a < t.length - 1));
  }
  var J = {
    stack: He,
    top() {
      return He[He.length - 1] || null;
    },
    find(t) {
      return He.find((e) => e.name === t) || null;
    },
    open(t, e, a = {}) {
      let n = document.createElement("section");
      return (
        (n.className = "screen " + (a.cls || "")),
        (n.dataset.name = t),
        (n.innerHTML = e),
        (n.style.zIndex = ++Mh),
        y("#scrim").before(n),
        He.push({ el: n, name: t, onClose: a.onClose }),
        n.offsetWidth,
        n.classList.add("open"),
        Ud(),
        n
      );
    },
    pop(t) {
      let e = He.pop();
      if (!e) return;
      let a = e.el;
      if (
        (t
          ? ((a.style.transition = "none"),
            a.classList.remove("open"),
            a.remove())
          : (a.classList.remove("open"), setTimeout(() => a.remove(), 480)),
        Ud(),
        e.onClose)
      )
        try {
          e.onClose();
        } catch (n) {
          console.error(n);
        }
    },
    popTo(t) {
      for (; He.length && J.top().name !== t; ) J.pop(!0);
    },
    clear() {
      for (; He.length; ) J.pop(!0);
    },
  };
  function qd() {
    let t = y("#viewport"),
      e = null;
    t.addEventListener("pointerdown", (c) => {
      if (
        !He.length ||
        di() ||
        !y("#auth").hidden ||
        !y("#lock").hidden ||
        y("#call").classList.contains("open") ||
        !y("#viewer").hidden
      )
        return;
      let l = t.getBoundingClientRect();
      c.clientX - l.left > 24 ||
        (e = {
          x0: c.clientX,
          y0: c.clientY,
          id: c.pointerId,
          on: !1,
          t0: performance.now(),
          w: l.width,
        });
    }),
      t.addEventListener("pointermove", (c) => {
        if (!e || c.pointerId !== e.id) return;
        let l = c.clientX - e.x0,
          u = c.clientY - e.y0;
        if (!e.on)
          if (l > 10 && l > Math.abs(u) * 1.2) {
            (e.on = !0),
              (e.top = He[He.length - 1].el),
              (e.under =
                He.length > 1 ? He[He.length - 2].el : y("#screen-main"));
            try {
              t.setPointerCapture(c.pointerId);
            } catch {}
            (e.top.style.transition = "none"),
              (e.under.style.transition = "none");
          } else if (Math.abs(u) > 14) {
            e = null;
            return;
          } else return;
        let d = Math.max(0, l),
          f = Math.min(1, d / e.w);
        (e.top.style.transform = `translate3d(${d}px,0,0)`),
          (e.under.style.transform = `translate3d(${-26 * (1 - f)}%,0,0)`),
          e.under.style.setProperty("--dim", String(0.55 * (1 - f)));
      });
    let a = (c, l) => {
      if (!e || c.pointerId !== e.id) return;
      let u = e;
      if (((e = null), !u.on)) return;
      let d = Math.max(0, c.clientX - u.x0),
        f = d / Math.max(1, performance.now() - u.t0),
        p = !l && (d > u.w * 0.33 || f > 0.5),
        v = () =>
          [u.top, u.under].forEach((k) => {
            (k.style.transition = ""),
              (k.style.transform = ""),
              k.style.removeProperty("--dim");
          });
      (u.top.style.transition = "transform .38s var(--ease)"),
        (u.under.style.transition = "transform .38s var(--ease)"),
        p
          ? ((u.top.style.transform = "translate3d(100%,0,0)"),
            (u.under.style.transform = "translate3d(0,0,0)"),
            u.under.style.setProperty("--dim", "0"),
            setTimeout(() => {
              v(), J.pop(!0);
            }, 380))
          : ((u.top.style.transform = "translate3d(0,0,0)"),
            (u.under.style.transform = "translate3d(-26%,0,0)"),
            u.under.style.setProperty("--dim", ".55"),
            setTimeout(v, 390));
    };
    t.addEventListener("pointerup", (c) => a(c, !1)),
      t.addEventListener("pointercancel", (c) => a(c, !0));
    let n = y("#sheet"),
      s = y("#scrim"),
      o = null;
    n.addEventListener("pointerdown", (c) => {
      if (c.target.closest(".grab")) {
        o = { y0: c.clientY, t0: performance.now() };
        try {
          n.setPointerCapture(c.pointerId);
        } catch {}
        n.style.transition = "none";
      }
    }),
      n.addEventListener("pointermove", (c) => {
        if (!o) return;
        let l = Math.max(0, c.clientY - o.y0);
        (n.style.transform = `translate3d(0,${l}px,0)`),
          (s.style.opacity = String(Math.max(0, 1 - l / 420)));
      });
    let r = (c) => {
      if (!o) return;
      let l = Math.max(0, c.clientY - o.y0),
        u = l / Math.max(1, performance.now() - o.t0);
      (o = null),
        (n.style.transition = ""),
        (n.style.transform = ""),
        (s.style.opacity = ""),
        (l > 110 || u > 0.6) && F();
    };
    n.addEventListener("pointerup", r), n.addEventListener("pointercancel", r);
    let i =
      ".row,.chan,.srow,.mitem,.ibtn,.chip,.pbt,.stile,.atile,.btn,.tplb,.sendbtn,.cbtn .cc,.endbtn,.fab,.seg button,.joinbtn,.bgp,.swb .sc,.mini,.rolebtn,.seg2 button,.key";
    document.addEventListener(
      "pointerdown",
      (c) => {
        let l = c.target.closest(i);
        if (!l || l.disabled) return;
        let u = l.getBoundingClientRect(),
          d = Math.max(u.width, u.height) * 2.2,
          f = document.createElement("span");
        (f.className = "rpl"),
          (f.style.cssText = `width:${d}px;height:${d}px;left:${c.clientX - u.left - d / 2}px;top:${c.clientY - u.top - d / 2}px`),
          l.appendChild(f),
          setTimeout(() => f.remove(), 650);
      },
      { passive: !0 },
    ),
      document.addEventListener("click", (c) => {
        let l = c.target.closest("[data-act]");
        if (!l || l.tagName === "INPUT") return;
        let u = vt[l.dataset.act];
        if (u)
          try {
            u(l, c);
          } catch (d) {
            console.error(d), _("Algo deu errado. Tente de novo.");
          }
      }),
      document.addEventListener("keydown", (c) => {
        let l = c.target;
        (c.key === "Enter" || c.key === " ") &&
          l.matches &&
          l.matches('[role="button"][data-act]') &&
          (c.preventDefault(), l.click());
      });
  }
  function qt(t) {
    t &&
      ([...t.children].forEach((e, a) =>
        e.style.setProperty("--i", Math.min(a, 10)),
      ),
      t.classList.remove("anim"),
      t.offsetWidth,
      t.classList.add("anim"),
      clearTimeout(t._t),
      (t._t = setTimeout(() => t.classList.remove("anim"), 1100)));
  }
  function Pd(t, e) {
    let a;
    return (...n) => {
      clearTimeout(a), (a = setTimeout(() => t(...n), e));
    };
  }
  var h = {
    engine: null,
    tab: "chats",
    filter: "all",
    q: "",
    visible: !0,
    hiddenAt: 0,
    typing: new Map(),
    invites: [],
    version: "0.4.0-beta",
  };
  function Vd(t) {
    let e = h.engine;
    if (t === e.pk) return (e.me && e.me.pic) || "";
    let a = e.contacts.get(t);
    if (a && a.pic) return a.status === "blocked" ? "" : a.pic;
    for (let n of e.groups.values()) if (n.pics && n.pics[t]) return n.pics[t];
    return "";
  }
  function Pt(t) {
    let e = h.engine;
    if (t === e.pk)
      return {
        name: e.me ? e.me.name : "Voc\xEA",
        t: e.me ? e.me.t : 0,
        me: !0,
        pic: Vd(t),
      };
    let a = e.contacts.get(t);
    return {
      name: e.contactName(t),
      t: a ? a.t : parseInt(t.slice(0, 2), 16) % 6,
      verified: !!(a && a.verified),
      contact: a,
      status: a ? a.status : null,
      pic: Vd(t),
    };
  }
  function pe(t, e = 52, a = {}) {
    let n = Pt(t);
    return et({
      name: n.name,
      t: n.t,
      size: e,
      verified: a.badge !== !1 && n.verified,
      ring: a.ring,
      pic: n.pic,
    });
  }
  function Pe(t, e = 52) {
    return et({
      name: t.meta.name,
      t: t.meta.t || 0,
      size: e,
      sq: !0,
      pic: t.pic,
    });
  }
  var ne = (t) => Pt(t).name,
    Ue = (t) => {
      let e = ne(t);
      return e.includes("\u2026") ? e : e.split(" ")[0];
    };
  function zd(t) {
    let e = h.engine;
    if (t.kind === "dm") return ne(t.pk);
    let a = e.groups.get(t.lid);
    return a ? a.meta.name : "Grupo";
  }
  function ya(t) {
    let e = h.typing.get(t);
    return e && e > Date.now();
  }
  var P = {};
  var ro = (t) => (
      (t = t.replace("#", "")),
      t.length === 3 &&
        (t = t
          .split("")
          .map((e) => e + e)
          .join("")),
      [0, 2, 4].map((e) => parseInt(t.slice(e, e + 2), 16))
    ),
    Gd = (t) =>
      "#" +
      t
        .map((e) =>
          Math.max(0, Math.min(255, Math.round(e)))
            .toString(16)
            .padStart(2, "0"),
        )
        .join(""),
    Vt = (t, e, a) => {
      let n = ro(t),
        s = ro(e);
      return Gd(n.map((o, r) => o + (s[r] - o) * a));
    },
    _n = (t) => {
      let [e, a, n] = ro(t).map(
        (s) => (
          (s /= 255),
          s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
        ),
      );
      return 0.2126 * e + 0.7152 * a + 0.0722 * n;
    },
    fi = (t, e) => {
      let a = _n(t),
        n = _n(e);
      return (Math.max(a, n) + 0.05) / (Math.min(a, n) + 0.05);
    },
    ui = (t) => (fi(t, "#08090a") >= fi(t, "#ffffff") ? "#08090a" : "#ffffff");
  function Rh(t) {
    let [e, a, n] = ro(t).map((u) => u / 255),
      s = Math.max(e, a, n),
      o = Math.min(e, a, n),
      r = (s + o) / 2,
      i = s - o,
      c = 0,
      l = 0;
    return (
      i &&
        ((l = i / (1 - Math.abs(2 * r - 1))),
        (c =
          s === e
            ? ((a - n) / i) % 6
            : s === a
              ? (n - e) / i + 2
              : (e - a) / i + 4),
        (c *= 60),
        c < 0 && (c += 360)),
      [Math.round(c), Math.round(l * 100), Math.round(r * 100)]
    );
  }
  function Kd(t, e, a) {
    (e /= 100), (a /= 100);
    let n = (r) => (r + t / 30) % 12,
      s = e * Math.min(a, 1 - a),
      o = (r) =>
        a - s * Math.max(-1, Math.min(n(r) - 3, Math.min(9 - n(r), 1)));
    return Gd([o(0) * 255, o(8) * 255, o(4) * 255]);
  }
  function Ih(t, e, a) {
    let n = _n(e) < 0.42,
      s = t,
      o = 0;
    for (; fi(s, e) < a && o++ < 20; )
      s = Vt(s, n ? "#ffffff" : "#000000", 0.08);
    return s;
  }
  var Fd = { accent: "#2fd47c", bg: "#000000", sent: null, recv: null },
    pi = [
      { n: "Verde", c: "#2fd47c" },
      { n: "Azul", c: "#4aa3ff" },
      { n: "Roxo", c: "#a184ff" },
      { n: "Rosa", c: "#ff6b9a" },
      { n: "Laranja", c: "#ff9442" },
      { n: "Amarelo", c: "#ffd43b" },
      { n: "Vermelho", c: "#ff5a5f" },
      { n: "Ciano", c: "#2fd6e0" },
    ],
    Wd = [
      { n: "Preto", c: "#000000" },
      { n: "Grafite", c: "#15171a" },
      { n: "Meia-noite", c: "#0b1220" },
      { n: "Vinho", c: "#1b0d14" },
      { n: "Floresta", c: "#0a1511" },
      { n: "Claro", c: "#f4f5f7" },
    ],
    Hh = [
      "bg",
      "s1",
      "s2",
      "s3",
      "s4",
      "line",
      "fg",
      "fg2",
      "fg3",
      "accent",
      "accent-ink",
      "accent-t",
      "sent",
      "sent-fg",
      "recv",
      "recv-fg",
      "stage",
    ],
    Dn = /^#[0-9a-f]{6}$/i,
    Nn = (t, e) => t && e && t.toLowerCase() === e.toLowerCase(),
    Le = { ...Fd },
    zt = {};
  function Uh(t) {
    let e = _n(t.bg) < 0.4,
      a = Vt(e ? "#ffffff" : "#000000", t.accent, 0.16),
      n = (l) => Vt(t.bg, a, l),
      s = e ? "#f3f4f6" : "#101215",
      o = n(e ? 0.09 : 0.06),
      r = t.sent || Vt(t.bg, t.accent, e ? 0.36 : 0.3),
      i = t.recv || o,
      c = e ? Vt(t.bg, "#000000", 0.55) : Vt(t.bg, "#000000", 0.22);
    return (
      _n(c) < 0.0015 && (c = "#050506"),
      {
        dark: e,
        bg: t.bg,
        s1: n(e ? 0.045 : 0.03),
        s2: o,
        s3: n(e ? 0.15 : 0.1),
        s4: n(e ? 0.22 : 0.16),
        line: n(e ? 0.11 : 0.09),
        fg: s,
        fg2: Vt(s, t.bg, 0.36),
        fg3: Vt(s, t.bg, 0.6),
        accent: t.accent,
        "accent-ink": ui(t.accent),
        "accent-t": Ih(t.accent, t.bg, 3),
        sent: r,
        "sent-fg": ui(r),
        recv: i,
        "recv-fg": ui(i),
        stage: c,
      }
    );
  }
  function hi(t = !0, e = !1) {
    zt = Uh(Le);
    let a = document.documentElement;
    if (
      (e && a.classList.add("no-anim"),
      Hh.forEach((n) => a.style.setProperty("--" + n, zt[n])),
      (a.style.colorScheme = zt.dark ? "dark" : "light"),
      e && (a.offsetWidth, a.classList.remove("no-anim")),
      t)
    )
      try {
        localStorage.setItem("jm.theme", JSON.stringify(Le));
      } catch {}
    X.setBars(zt.bg, !zt.dark);
  }
  function Yd() {
    try {
      let t = JSON.parse(localStorage.getItem("jm.theme") || "null");
      t &&
        Dn.test(t.accent || "") &&
        Dn.test(t.bg || "") &&
        (Le = {
          accent: t.accent,
          bg: t.bg,
          sent: Dn.test(t.sent || "") ? t.sent : null,
          recv: Dn.test(t.recv || "") ? t.recv : null,
        });
    } catch {}
    hi(!1, !0);
  }
  var mi = () => {
    let t = pi.find((e) => Nn(e.c, Le.accent));
    return t ? t.n : "Personalizada";
  };
  function oo(t) {
    let e = { h: "Matiz", s: "Satura\xE7\xE3o", l: "Brilho" };
    return `<div class="ced${t === "sent" || t === "recv" ? "" : " solo"}" data-role="${t}"><div class="cedi">
    <div class="cedrow"><span class="ced-prev"></span><input class="hexin" data-role="${t}" maxlength="7" spellcheck="false" autocapitalize="off" autocomplete="off" aria-label="C\xF3digo hexadecimal da cor">
      ${t === "sent" || t === "recv" ? `<button class="mini" data-act="edAuto" data-role="${t}">Autom\xE1tico</button>` : ""}</div>
    ${["h", "s", "l"].map((a) => `<label class="sl"><span>${e[a]}</span><input class="rs" type="range" data-role="${t}" data-ch="${a}" min="0" max="${a === "h" ? 360 : 100}" aria-label="${e[a]}"></label>`).join("")}
  </div></div>`;
  }
  function io(t) {
    let e =
        pi
          .map(
            (s) =>
              `<button class="swb" data-act="setAccent" data-c="${s.c}" style="--c:${s.c}" aria-label="${s.n}"><span class="sc">${x("check")}</span>${s.n}</button>`,
          )
          .join("") +
        '<button class="swb cust" data-act="edToggle" data-role="accent" aria-label="Escolher outra cor de destaque"><span class="sc"></span>Outra</button>',
      a =
        Wd.map(
          (s) =>
            `<button class="bgp" data-act="setBg" data-c="${s.c}" style="--c:${s.c}" aria-label="Fundo ${s.n}"><span class="bc"></span>${s.n}</button>`,
        ).join("") +
        '<button class="bgp cust" data-act="edToggle" data-role="bg" aria-label="Escolher outra cor de fundo"><span class="bc"></span>Outra</button>',
      n = t
        ? `<div class="label">Bal\xF5es</div>
    <div class="role"><button class="rolebtn" data-act="edToggle" data-role="sent"><span class="rsw rsw-sent"></span><span class="rtx"><b>Bal\xE3o enviado</b><small class="rs-sent">Autom\xE1tico</small></span>${x("chevd", "sm")}</button>${oo("sent")}</div>
    <div class="role"><button class="rolebtn" data-act="edToggle" data-role="recv"><span class="rsw rsw-recv"></span><span class="rtx"><b>Bal\xE3o recebido</b><small class="rs-recv">Autom\xE1tico</small></span>${x("chevd", "sm")}</button>${oo("recv")}</div>`
        : "";
    return `<div class="studio">
    <div class="pv" aria-label="Pr\xE9via">
      <div class="pvh"><span class="brand"><span>J-</span>Menseger</span><span class="chtag pvName">Verde</span></div>
      <div class="mrow them first last"><div class="msg them" style="cursor:default">Olha como ficou a cor nova<span class="meta">14:20</span></div></div>
      <div class="mrow me first last"><div class="msg me" style="cursor:default">Gostei! Combina com o fundo<span class="meta">14:21<span class="tick read">${x("checks", "xs")}</span></span></div></div>
      <div class="crow"><div class="field">Mensagem</div><span class="sendbtn">${x("send", "sm")}</span></div>
    </div>
    <div class="label">Cor de destaque</div>
    <div class="sws">${e}</div>
    ${oo("accent")}
    <div class="label">Fundo</div>
    <div class="bgs">${a}</div>
    ${oo("bg")}
    ${n}
    <p class="note">${t ? "Qualquer cor vale. O texto ajusta sozinho para continuar leg\xEDvel." : "Come\xE7amos em verde. D\xE1 para mudar tudo depois em Ajustes \u2192 Apar\xEAncia."}</p>
  </div>`;
  }
  var Dh = (t) =>
    t === "accent"
      ? Le.accent
      : t === "bg"
        ? Le.bg
        : t === "sent"
          ? zt.sent
          : zt.recv;
  function va(t) {
    oe(".swb[data-c]").forEach((e) =>
      e.classList.toggle("on", Nn(e.dataset.c, Le.accent)),
    ),
      oe(".swb.cust").forEach((e) => {
        e.classList.toggle("on", !pi.some((a) => Nn(a.c, Le.accent))),
          e.style.setProperty("--c", Le.accent);
      }),
      oe(".bgp[data-c]").forEach((e) =>
        e.classList.toggle("on", Nn(e.dataset.c, Le.bg)),
      ),
      oe(".bgp.cust").forEach((e) => {
        e.classList.toggle("on", !Wd.some((a) => Nn(a.c, Le.bg))),
          e.style.setProperty("--c", Le.bg);
      }),
      oe(".pvName, .acc-name").forEach((e) => (e.textContent = mi())),
      ["sent", "recv"].forEach((e) => {
        oe(".rs-" + e).forEach(
          (a) =>
            (a.textContent = Le[e] ? Le[e].toUpperCase() : "Autom\xE1tico"),
        ),
          oe(".rsw-" + e).forEach((a) => (a.style.background = zt[e]));
      }),
      oe(".ced").forEach((e) => {
        let a = e.dataset.role,
          n = Dh(a),
          [s, o, r] = Rh(n);
        y(".ced-prev", e).style.background = n;
        let i = y(".hexin", e);
        document.activeElement !== i &&
          ((i.value = n.toUpperCase()), i.classList.remove("bad"));
        let c = { h: s, s: o, l: r };
        oe(".rs", e).forEach((u) => {
          e !== t && (u.value = c[u.dataset.ch]);
        });
        let l = {
          h: +y('[data-ch="h"]', e).value,
          s: +y('[data-ch="s"]', e).value,
          l: +y('[data-ch="l"]', e).value,
        };
        oe(".rs", e).forEach((u) => {
          let d = u.dataset.ch;
          u.style.setProperty("--thumb", Kd(l.h, l.s, l.l)),
            u.style.setProperty(
              "--trk",
              d === "h"
                ? `linear-gradient(90deg,${[0, 60, 120, 180, 240, 300, 360].map((f) => `hsl(${f} ${l.s}% ${l.l}%)`).join(",")})`
                : d === "s"
                  ? `linear-gradient(90deg,hsl(${l.h} 0% ${l.l}%),hsl(${l.h} 100% ${l.l}%))`
                  : `linear-gradient(90deg,#000,hsl(${l.h} ${l.s}% 50%),#fff)`,
            );
        });
      });
  }
  function On(t, e, a) {
    (Le[t] = e ? e.toLowerCase() : null), hi(), va(a);
  }
  function Zd() {
    (Le = { ...Fd }), hi(), va();
  }
  b("setAccent", (t) => On("accent", t.dataset.c));
  b("setBg", (t) => On("bg", t.dataset.c));
  b("edAuto", (t) => On(t.dataset.role, null));
  b("edToggle", (t) => {
    let e = t.dataset.role,
      a = t.closest(".studio") || document,
      n = y(`.ced[data-role="${e}"]`, a);
    if (!n) return;
    let s = n.classList.contains("open");
    if (
      (oe(".ced.open", a).forEach((o) => {
        o.classList.remove("open");
        let r = o.closest(".role");
        r && r.classList.remove("open");
      }),
      !s)
    ) {
      n.classList.add("open");
      let o = n.closest(".role");
      o && o.classList.add("open"),
        va(),
        setTimeout(
          () => n.scrollIntoView({ block: "nearest", behavior: "smooth" }),
          260,
        );
    }
  });
  document.addEventListener("input", (t) => {
    let e = t.target;
    if (e.classList && e.classList.contains("rs")) {
      let a = e.closest(".ced"),
        n = (s) => +y(`[data-ch="${s}"]`, a).value;
      On(a.dataset.role, Kd(n("h"), n("s"), n("l")), a);
    } else if (e.classList && e.classList.contains("hexin")) {
      let a = e.value.trim();
      a && a[0] !== "#" && (a = "#" + a),
        Dn.test(a)
          ? (e.classList.remove("bad"),
            On(e.dataset.role, a, e.closest(".ced")))
          : e.classList.toggle("bad", a.length >= 7);
    }
  });
  var wt = null,
    wa = null,
    bi = 0.7,
    Pn = [
      { id: "aplausos", e: "\u{1F44F}", n: "Aplausos", d: 3.7 },
      { id: "badumtss", e: "\u{1F606}", n: "Ba dum tss", d: 2.6 },
      { id: "trombone", e: "\u{1F622}", n: "Trombone triste", d: 4 },
      { id: "grilos", e: "\u{1F997}", n: "Grilos", d: 3.4 },
      { id: "suspense", e: "\u{1F631}", n: "Dun dun duuun", d: 4.6 },
      { id: "rufar", e: "\u{1F941}", n: "Rufar de tambores", d: 4.7 },
      { id: "vitoria", e: "\u{1F3C6}", n: "Vit\xF3ria", d: 3.5 },
      { id: "tcharam", e: "\u{1F389}", n: "Tcharam!", d: 2.8 },
      { id: "coro", e: "\u{1F607}", n: "Aleluia", d: 4.6 },
      { id: "assobio", e: "\u{1F60F}", n: "Fiu-fiu", d: 2.1 },
      { id: "violino", e: "\u{1F3BB}", n: "Violino triste", d: 4.4 },
      { id: "impacto", e: "\u{1F4A5}", n: "Impacto", d: 2.6 },
      { id: "sino", e: "\u{1F514}", n: "Acertou!", d: 2.3 },
      { id: "palmas", e: "\u{1F643}", n: "Palmas lentas", d: 3.7 },
    ].map((t) => ({ ...t, builtin: !0 })),
    Jd = {
      tambor: "badumtss",
      ding: "sino",
      uau: "coro",
      risada: "tcharam",
      buzina: "impacto",
      laser: "impacto",
      moeda: "sino",
    },
    Xd = /^c[0-9a-f]{15}$/,
    Vn = 16e4,
    tt = [],
    en = new Set(),
    tn = null,
    rt = new Map(),
    Qa = new Map(),
    jn = new Set();
  async function Qd(t) {
    tn = t;
    try {
      tt = (await t.db.all("sounds")).sort((e, a) => e.ts - a.ts);
    } catch {
      tt = [];
    }
    en = new Set(await t.kv("sbHidden", []));
  }
  var co = () => [...Pn.filter((t) => !en.has(t.id)), ...tt],
    eu = () => Pn.filter((t) => en.has(t.id));
  var gi = (t) => "c" + In(t).slice(0, 15);
  function kt(t) {
    return (
      (t = Jd[t] || t),
      Pn.find((e) => e.id === t) || tt.find((e) => e.id === t) || null
    );
  }
  function Ve() {
    if (!wt) {
      let t = window.AudioContext || window.webkitAudioContext;
      if (!t) return null;
      (wt = new t()),
        (wa = wt.createGain()),
        (wa.gain.value = bi * 0.9),
        wa.connect(wt.destination);
    }
    return wt.state === "suspended" && wt.resume(), wt;
  }
  function lo(t) {
    (bi = t), wa && (wa.gain.value = t * 0.9);
  }
  var xi = () => bi;
  async function Nh(t) {
    if (Pn.some((n) => n.id === t)) {
      let n = await fetch("sounds/" + t + ".ogg");
      return n.ok ? n.arrayBuffer() : null;
    }
    let e = tt.find((n) => n.id === t);
    if (e) return e.blob.arrayBuffer();
    let a = Qa.get(t);
    return a ? a.slice().buffer : null;
  }
  async function tu(t) {
    if (((t = Jd[t] || t), rt.has(t))) return rt.get(t);
    let e = Ve();
    if (!e) return null;
    let a = await Nh(t);
    if (!a) return null;
    let n = await e.decodeAudioData(a);
    return rt.set(t, n), rt.size > 60 && rt.delete(rt.keys().next().value), n;
  }
  function au(t) {
    let e = Ve();
    if (jn.size >= 3) {
      let n = jn.values().next().value;
      try {
        n.stop();
      } catch {}
      jn.delete(n);
    }
    let a = e.createBufferSource();
    return (
      (a.buffer = t),
      a.connect(wa),
      (a.onended = () => jn.delete(a)),
      jn.add(a),
      a.start(),
      t.duration
    );
  }
  async function Kt(t) {
    try {
      let e = await tu(t);
      return e ? au(e) : 0;
    } catch {
      return 0;
    }
  }
  async function nu(t, e) {
    try {
      let a = rt.get(t);
      return (
        a || ((a = await Ve().decodeAudioData(e.slice(0))), rt.set(t, a)), au(a)
      );
    } catch {
      return 0;
    }
  }
  async function yi() {
    for (let t of co())
      if (!rt.has(t.id))
        try {
          await tu(t.id);
        } catch {}
  }
  function su(t, e) {
    Qa.set(t, e), Qa.size > 30 && Qa.delete(Qa.keys().next().value);
  }
  var vi = (t) => Qa.get(t) || null;
  async function uo(t) {
    let e = tt.find((a) => a.id === t);
    return e ? new Uint8Array(await e.blob.arrayBuffer()) : null;
  }
  async function an({ bytes: t, mime: e, n: a, e: n, d: s }) {
    let o = gi(t),
      r = {
        id: o,
        n: String(a || "Som").slice(0, 24),
        e: n || "\u{1F50A}",
        d: Math.round((s || 1) * 10) / 10,
        mime: e,
        blob: new Blob([t], { type: e }),
        ts: Date.now(),
      };
    return (
      await tn.db.put("sounds", r),
      (tt = tt.filter((i) => i.id !== o).concat(r)),
      r
    );
  }
  async function ou(t, { n: e, e: a }) {
    let n = tt.find((s) => s.id === t);
    n &&
      (e && (n.n = String(e).slice(0, 24)),
      a && (n.e = a),
      await tn.db.put("sounds", n));
  }
  async function ru(t) {
    Pn.some((e) => e.id === t)
      ? (en.add(t), await tn.setKv("sbHidden", [...en]))
      : (await tn.db.del("sounds", t),
        (tt = tt.filter((e) => e.id !== t)),
        rt.delete(t));
  }
  async function iu() {
    en.clear(), await tn.setKv("sbHidden", []);
  }
  function Gt(t, e, a, n, s, o, r) {
    let i = wt.createOscillator(),
      c = wt.createGain();
    (i.type = t),
      i.frequency.setValueAtTime(e, n),
      a && i.frequency.exponentialRampToValueAtTime(a, n + s),
      c.gain.setValueAtTime(1e-4, n),
      c.gain.exponentialRampToValueAtTime(o, n + 0.012),
      c.gain.exponentialRampToValueAtTime(1e-4, n + s),
      i.connect(c),
      c.connect(r || wa),
      i.start(n),
      i.stop(n + s + 0.05);
  }
  function cu() {
    let t = Ve();
    if (!t) return;
    let e = t.currentTime + 0.01;
    Gt("sine", 880, 0, e, 0.12, 0.16), Gt("sine", 1320, 0, e + 0.09, 0.2, 0.12);
  }
  var qn = null;
  function lu() {
    ka();
    let t = Ve();
    if (!t) return;
    let e = () => {
      let a = t.currentTime + 0.02;
      for (let n = 0; n < 2; n++)
        Gt("sine", 523, 0, a + n * 0.42, 0.35, 0.35),
          Gt("sine", 659, 0, a + n * 0.42 + 0.12, 0.3, 0.25),
          Gt("sine", 784, 0, a + n * 0.42 + 0.22, 0.26, 0.2);
    };
    e(), (qn = setInterval(e, 2200));
  }
  function du() {
    ka();
    let t = Ve();
    if (!t) return;
    let e = () => {
      let a = t.currentTime + 0.02;
      Gt("sine", 425, 0, a, 1, 0.08);
    };
    e(), (qn = setInterval(e, 3500));
  }
  function ka() {
    qn && (clearInterval(qn), (qn = null));
  }
  function wi() {
    let t = Ve();
    if (!t) return;
    let e = t.currentTime + 0.02;
    Gt("sine", 480, 0, e, 0.18, 0.15), Gt("sine", 380, 0, e + 0.2, 0.26, 0.15);
  }
  var uu = new Uint32Array(256);
  for (let t = 0; t < 256; t++) {
    let e = t << 24;
    for (let a = 0; a < 8; a++)
      e = e & 2147483648 ? (e << 1) ^ 79764919 : e << 1;
    uu[t] = e >>> 0;
  }
  function _h(t) {
    let e = 0;
    for (let a = 0; a < t.length; a++)
      e = ((e << 8) ^ uu[((e >>> 24) ^ t[a]) & 255]) >>> 0;
    return e;
  }
  function Oh(t, e, a, n) {
    let s = (Math.random() * 4294967295) >>> 0,
      o = [],
      r = 0,
      i = (N, V, O) => {
        let q = [];
        for (let m of N) {
          let $ = m.length;
          for (; $ >= 255; ) q.push(255), ($ -= 255);
          q.push($);
        }
        let K = N.reduce((m, $) => m + $.length, 0),
          T = new Uint8Array(27 + q.length + K),
          g = new DataView(T.buffer);
        T.set([79, 103, 103, 83], 0),
          (T[5] = O),
          g.setBigUint64(6, BigInt(V), !0),
          g.setUint32(14, s, !0),
          g.setUint32(18, r++, !0),
          (T[26] = q.length),
          T.set(q, 27);
        let w = 27 + q.length;
        for (let m of N) T.set(m, w), (w += m.length);
        g.setUint32(22, _h(T), !0), o.push(T);
      },
      c = new TextEncoder(),
      l = new Uint8Array(19);
    l.set(c.encode("OpusHead"), 0),
      (l[8] = 1),
      (l[9] = 1),
      new DataView(l.buffer).setUint16(10, a, !0),
      new DataView(l.buffer).setUint32(12, 48e3, !0),
      i([l], 0, 2);
    let u = c.encode("J-Menseger"),
      d = new Uint8Array(12 + u.length + 4);
    d.set(c.encode("OpusTags"), 0),
      new DataView(d.buffer).setUint32(8, u.length, !0),
      d.set(u, 12),
      i([d], 0, 0);
    let f = 0,
      p = [],
      v = 0;
    for (let N = 0; N < t.length; N++) {
      let V = Math.floor(t[N].length / 255) + 1;
      p.length &&
        (v + V > 255 || p.length >= 50) &&
        (i(p, a + f, 0), (p = []), (v = 0)),
        p.push(t[N]),
        (v += V),
        (f += e[N]);
    }
    i(p, Math.min(a + n, f), 4);
    let k = new Uint8Array(o.reduce((N, V) => N + V.length, 0)),
      L = 0;
    for (let N of o) k.set(N, L), (L += N.length);
    return k;
  }
  async function jh(t, e) {
    let a = {
      codec: "opus",
      sampleRate: 48e3,
      numberOfChannels: 1,
      bitrate: e,
    };
    if (typeof AudioEncoder > "u" || typeof AudioData > "u") return null;
    try {
      if (!(await AudioEncoder.isConfigSupported(a)).supported) return null;
    } catch {
      return null;
    }
    let n = [],
      s = [],
      o = null,
      r = null,
      i = new AudioEncoder({
        output: (u, d) => {
          let f = new Uint8Array(u.byteLength);
          u.copyTo(f),
            n.push(f),
            s.push(u.duration ? Math.round((u.duration * 48e3) / 1e6) : 960),
            d &&
              d.decoderConfig &&
              d.decoderConfig.description &&
              (r = new Uint8Array(d.decoderConfig.description));
        },
        error: (u) => {
          o = u;
        },
      });
    i.configure(a);
    let c = 4800;
    for (let u = 0; u < t.length; u += c) {
      let d = t.slice(u, Math.min(t.length, u + c)),
        f = new AudioData({
          format: "f32-planar",
          sampleRate: 48e3,
          numberOfFrames: d.length,
          numberOfChannels: 1,
          timestamp: Math.round((u * 1e6) / 48e3),
          data: d,
        });
      i.encode(f), f.close();
    }
    if ((await i.flush(), i.close(), o || !n.length)) return null;
    let l = r && r.length >= 12 ? r[10] | (r[11] << 8) : 312;
    return { bytes: Oh(n, s, l, t.length), mime: "audio/ogg" };
  }
  async function qh(t, e) {
    let a = window.AudioContext || window.webkitAudioContext,
      n = new a({ sampleRate: 48e3 }),
      s = n.createBuffer(1, t.length, 48e3);
    s.copyToChannel(t, 0);
    let o = n.createBufferSource();
    o.buffer = s;
    let r = n.createMediaStreamDestination();
    o.connect(r);
    let i =
        ["audio/webm;codecs=opus", "audio/ogg;codecs=opus", "audio/webm"].find(
          (p) => window.MediaRecorder && MediaRecorder.isTypeSupported(p),
        ) || "",
      c = new MediaRecorder(
        r.stream,
        i ? { mimeType: i, audioBitsPerSecond: e } : { audioBitsPerSecond: e },
      ),
      l = [];
    c.ondataavailable = (p) => {
      p.data && p.data.size && l.push(p.data);
    };
    let u = new Promise((p) => {
      c.onstop = p;
    });
    c.start(),
      (o.onended = () => setTimeout(() => c.stop(), 150)),
      o.start(),
      await u,
      n.close();
    let d = (c.mimeType || i || "audio/webm").split(";")[0],
      f = new Blob(l, { type: d });
    return { bytes: new Uint8Array(await f.arrayBuffer()), mime: d };
  }
  async function fu(t, e = 64e3) {
    return (await jh(t, e)) || qh(t, e);
  }
  async function pu(t, e, a) {
    let n = Math.max(1, Math.round((a - e) * 48e3)),
      s = new OfflineAudioContext(1, n, 48e3),
      o = s.createBufferSource();
    o.buffer = t;
    let r = s.createGain();
    o.connect(r), r.connect(s.destination);
    let i = 0.006,
      c = Math.min(0.04, (a - e) / 4);
    r.gain.setValueAtTime(0, 0),
      r.gain.linearRampToValueAtTime(1, i),
      r.gain.setValueAtTime(1, Math.max(i, a - e - c)),
      r.gain.linearRampToValueAtTime(0, a - e),
      o.start(0, e, a - e);
    let l = (await s.startRendering()).getChannelData(0),
      u = 0;
    for (let f = 0; f < l.length; f++) u = Math.max(u, Math.abs(l[f]));
    let d = u > 5e-4 ? Math.min(8, 0.89 / u) : 1;
    if (d !== 1) for (let f = 0; f < l.length; f++) l[f] *= d;
    return l;
  }
  var nn = function (t, e) {
    let s = t,
      o = zn[e],
      r = null,
      i = 0,
      c = null,
      l = [],
      u = {},
      d = function (g, w) {
        (i = s * 4 + 17),
          (r = (function (m) {
            let $ = new Array(m);
            for (let A = 0; A < m; A += 1) {
              $[A] = new Array(m);
              for (let R = 0; R < m; R += 1) $[A][R] = null;
            }
            return $;
          })(i)),
          f(0, 0),
          f(i - 7, 0),
          f(0, i - 7),
          k(),
          v(),
          N(g, w),
          s >= 7 && L(g),
          c == null && (c = q(s, o, l)),
          V(c, w);
      },
      f = function (g, w) {
        for (let m = -1; m <= 7; m += 1)
          if (!(g + m <= -1 || i <= g + m))
            for (let $ = -1; $ <= 7; $ += 1)
              w + $ <= -1 ||
                i <= w + $ ||
                ((0 <= m && m <= 6 && ($ == 0 || $ == 6)) ||
                (0 <= $ && $ <= 6 && (m == 0 || m == 6)) ||
                (2 <= m && m <= 4 && 2 <= $ && $ <= 4)
                  ? (r[g + m][w + $] = !0)
                  : (r[g + m][w + $] = !1));
      },
      p = function () {
        let g = 0,
          w = 0;
        for (let m = 0; m < 8; m += 1) {
          d(!0, m);
          let $ = Wt.getLostPoint(u);
          (m == 0 || g > $) && ((g = $), (w = m));
        }
        return w;
      },
      v = function () {
        for (let g = 8; g < i - 8; g += 1)
          r[g][6] == null && (r[g][6] = g % 2 == 0);
        for (let g = 8; g < i - 8; g += 1)
          r[6][g] == null && (r[6][g] = g % 2 == 0);
      },
      k = function () {
        let g = Wt.getPatternPosition(s);
        for (let w = 0; w < g.length; w += 1)
          for (let m = 0; m < g.length; m += 1) {
            let $ = g[w],
              A = g[m];
            if (r[$][A] == null)
              for (let R = -2; R <= 2; R += 1)
                for (let E = -2; E <= 2; E += 1)
                  R == -2 || R == 2 || E == -2 || E == 2 || (R == 0 && E == 0)
                    ? (r[$ + R][A + E] = !0)
                    : (r[$ + R][A + E] = !1);
          }
      },
      L = function (g) {
        let w = Wt.getBCHTypeNumber(s);
        for (let m = 0; m < 18; m += 1) {
          let $ = !g && ((w >> m) & 1) == 1;
          r[Math.floor(m / 3)][(m % 3) + i - 8 - 3] = $;
        }
        for (let m = 0; m < 18; m += 1) {
          let $ = !g && ((w >> m) & 1) == 1;
          r[(m % 3) + i - 8 - 3][Math.floor(m / 3)] = $;
        }
      },
      N = function (g, w) {
        let m = (o << 3) | w,
          $ = Wt.getBCHTypeInfo(m);
        for (let A = 0; A < 15; A += 1) {
          let R = !g && (($ >> A) & 1) == 1;
          A < 6
            ? (r[A][8] = R)
            : A < 8
              ? (r[A + 1][8] = R)
              : (r[i - 15 + A][8] = R);
        }
        for (let A = 0; A < 15; A += 1) {
          let R = !g && (($ >> A) & 1) == 1;
          A < 8
            ? (r[8][i - A - 1] = R)
            : A < 9
              ? (r[8][15 - A - 1 + 1] = R)
              : (r[8][15 - A - 1] = R);
        }
        r[i - 8][8] = !g;
      },
      V = function (g, w) {
        let m = -1,
          $ = i - 1,
          A = 7,
          R = 0,
          E = Wt.getMaskFunction(w);
        for (let B = i - 1; B > 0; B -= 2)
          for (B == 6 && (B -= 1); ; ) {
            for (let S = 0; S < 2; S += 1)
              if (r[$][B - S] == null) {
                let D = !1;
                R < g.length && (D = ((g[R] >>> A) & 1) == 1),
                  E($, B - S) && (D = !D),
                  (r[$][B - S] = D),
                  (A -= 1),
                  A == -1 && ((R += 1), (A = 7));
              }
            if ((($ += m), $ < 0 || i <= $)) {
              ($ -= m), (m = -m);
              break;
            }
          }
      },
      O = function (g, w) {
        let m = 0,
          $ = 0,
          A = 0,
          R = new Array(w.length),
          E = new Array(w.length);
        for (let I = 0; I < w.length; I += 1) {
          let j = w[I].dataCount,
            U = w[I].totalCount - j;
          ($ = Math.max($, j)), (A = Math.max(A, U)), (R[I] = new Array(j));
          for (let te = 0; te < R[I].length; te += 1)
            R[I][te] = 255 & g.getBuffer()[te + m];
          m += j;
          let G = Wt.getErrorCorrectPolynomial(U),
            Y = Gn(R[I], G.getLength() - 1).mod(G);
          E[I] = new Array(G.getLength() - 1);
          for (let te = 0; te < E[I].length; te += 1) {
            let re = te + Y.getLength() - E[I].length;
            E[I][te] = re >= 0 ? Y.getAt(re) : 0;
          }
        }
        let B = 0;
        for (let I = 0; I < w.length; I += 1) B += w[I].totalCount;
        let S = new Array(B),
          D = 0;
        for (let I = 0; I < $; I += 1)
          for (let j = 0; j < w.length; j += 1)
            I < R[j].length && ((S[D] = R[j][I]), (D += 1));
        for (let I = 0; I < A; I += 1)
          for (let j = 0; j < w.length; j += 1)
            I < E[j].length && ((S[D] = E[j][I]), (D += 1));
        return S;
      },
      q = function (g, w, m) {
        let $ = hu.getRSBlocks(g, w),
          A = mu();
        for (let E = 0; E < m.length; E += 1) {
          let B = m[E];
          A.put(B.getMode(), 4),
            A.put(B.getLength(), Wt.getLengthInBits(B.getMode(), g)),
            B.write(A);
        }
        let R = 0;
        for (let E = 0; E < $.length; E += 1) R += $[E].dataCount;
        if (A.getLengthInBits() > R * 8)
          throw (
            "code length overflow. (" + A.getLengthInBits() + ">" + R * 8 + ")"
          );
        for (
          A.getLengthInBits() + 4 <= R * 8 && A.put(0, 4);
          A.getLengthInBits() % 8 != 0;

        )
          A.putBit(!1);
        for (
          ;
          !(
            A.getLengthInBits() >= R * 8 ||
            (A.put(236, 8), A.getLengthInBits() >= R * 8)
          );

        )
          A.put(17, 8);
        return O(A, $);
      };
    (u.addData = function (g, w) {
      w = w || "Byte";
      let m = null;
      switch (w) {
        case "Numeric":
          m = Ph(g);
          break;
        case "Alphanumeric":
          m = Vh(g);
          break;
        case "Byte":
          m = zh(g);
          break;
        case "Kanji":
          m = Gh(g);
          break;
        default:
          throw "mode:" + w;
      }
      l.push(m), (c = null);
    }),
      (u.isDark = function (g, w) {
        if (g < 0 || i <= g || w < 0 || i <= w) throw g + "," + w;
        return r[g][w];
      }),
      (u.getModuleCount = function () {
        return i;
      }),
      (u.make = function () {
        if (s < 1) {
          let g = 1;
          for (; g < 40; g++) {
            let w = hu.getRSBlocks(g, o),
              m = mu();
            for (let A = 0; A < l.length; A++) {
              let R = l[A];
              m.put(R.getMode(), 4),
                m.put(R.getLength(), Wt.getLengthInBits(R.getMode(), g)),
                R.write(m);
            }
            let $ = 0;
            for (let A = 0; A < w.length; A++) $ += w[A].dataCount;
            if (m.getLengthInBits() <= $ * 8) break;
          }
          s = g;
        }
        d(!1, p());
      }),
      (u.createTableTag = function (g, w) {
        (g = g || 2), (w = typeof w > "u" ? g * 4 : w);
        let m = "";
        (m += '<table style="'),
          (m += " border-width: 0px; border-style: none;"),
          (m += " border-collapse: collapse;"),
          (m += " padding: 0px; margin: " + w + "px;"),
          (m += '">'),
          (m += "<tbody>");
        for (let $ = 0; $ < u.getModuleCount(); $ += 1) {
          m += "<tr>";
          for (let A = 0; A < u.getModuleCount(); A += 1)
            (m += '<td style="'),
              (m += " border-width: 0px; border-style: none;"),
              (m += " border-collapse: collapse;"),
              (m += " padding: 0px; margin: 0px;"),
              (m += " width: " + g + "px;"),
              (m += " height: " + g + "px;"),
              (m += " background-color: "),
              (m += u.isDark($, A) ? "#000000" : "#ffffff"),
              (m += ";"),
              (m += '"/>');
          m += "</tr>";
        }
        return (m += "</tbody>"), (m += "</table>"), m;
      }),
      (u.createSvgTag = function (g, w, m, $) {
        let A = {};
        typeof arguments[0] == "object" &&
          ((A = arguments[0]),
          (g = A.cellSize),
          (w = A.margin),
          (m = A.alt),
          ($ = A.title)),
          (g = g || 2),
          (w = typeof w > "u" ? g * 4 : w),
          (m = typeof m == "string" ? { text: m } : m || {}),
          (m.text = m.text || null),
          (m.id = m.text ? m.id || "qrcode-description" : null),
          ($ = typeof $ == "string" ? { text: $ } : $ || {}),
          ($.text = $.text || null),
          ($.id = $.text ? $.id || "qrcode-title" : null);
        let R = u.getModuleCount() * g + w * 2,
          E,
          B,
          S,
          D,
          I = "",
          j;
        for (
          j = "l" + g + ",0 0," + g + " -" + g + ",0 0,-" + g + "z ",
            I += '<svg version="1.1" xmlns="http://www.w3.org/2000/svg"',
            I += A.scalable ? "" : ' width="' + R + 'px" height="' + R + 'px"',
            I += ' viewBox="0 0 ' + R + " " + R + '" ',
            I += ' preserveAspectRatio="xMinYMin meet"',
            I +=
              $.text || m.text
                ? ' role="img" aria-labelledby="' +
                  K([$.id, m.id].join(" ").trim()) +
                  '"'
                : "",
            I += ">",
            I += $.text
              ? '<title id="' + K($.id) + '">' + K($.text) + "</title>"
              : "",
            I += m.text
              ? '<description id="' +
                K(m.id) +
                '">' +
                K(m.text) +
                "</description>"
              : "",
            I +=
              '<rect width="100%" height="100%" fill="white" cx="0" cy="0"/>',
            I += '<path d="',
            S = 0;
          S < u.getModuleCount();
          S += 1
        )
          for (D = S * g + w, E = 0; E < u.getModuleCount(); E += 1)
            u.isDark(S, E) && ((B = E * g + w), (I += "M" + B + "," + D + j));
        return (
          (I += '" stroke="transparent" fill="black"/>'), (I += "</svg>"), I
        );
      }),
      (u.createDataURL = function (g, w) {
        (g = g || 2), (w = typeof w > "u" ? g * 4 : w);
        let m = u.getModuleCount() * g + w * 2,
          $ = w,
          A = m - w;
        return Yh(m, m, function (R, E) {
          if ($ <= R && R < A && $ <= E && E < A) {
            let B = Math.floor((R - $) / g),
              S = Math.floor((E - $) / g);
            return u.isDark(S, B) ? 0 : 1;
          } else return 1;
        });
      }),
      (u.createImgTag = function (g, w, m) {
        (g = g || 2), (w = typeof w > "u" ? g * 4 : w);
        let $ = u.getModuleCount() * g + w * 2,
          A = "";
        return (
          (A += "<img"),
          (A += ' src="'),
          (A += u.createDataURL(g, w)),
          (A += '"'),
          (A += ' width="'),
          (A += $),
          (A += '"'),
          (A += ' height="'),
          (A += $),
          (A += '"'),
          m && ((A += ' alt="'), (A += K(m)), (A += '"')),
          (A += "/>"),
          A
        );
      });
    let K = function (g) {
        let w = "";
        for (let m = 0; m < g.length; m += 1) {
          let $ = g.charAt(m);
          switch ($) {
            case "<":
              w += "&lt;";
              break;
            case ">":
              w += "&gt;";
              break;
            case "&":
              w += "&amp;";
              break;
            case '"':
              w += "&quot;";
              break;
            default:
              w += $;
              break;
          }
        }
        return w;
      },
      T = function (g) {
        g = typeof g > "u" ? 2 : g;
        let m = u.getModuleCount() * 1 + g * 2,
          $ = g,
          A = m - g,
          R,
          E,
          B,
          S,
          D,
          I = {
            "\u2588\u2588": "\u2588",
            "\u2588 ": "\u2580",
            " \u2588": "\u2584",
            "  ": " ",
          },
          j = {
            "\u2588\u2588": "\u2580",
            "\u2588 ": "\u2580",
            " \u2588": " ",
            "  ": " ",
          },
          U = "";
        for (R = 0; R < m; R += 2) {
          for (
            B = Math.floor((R - $) / 1), S = Math.floor((R + 1 - $) / 1), E = 0;
            E < m;
            E += 1
          )
            (D = "\u2588"),
              $ <= E &&
                E < A &&
                $ <= R &&
                R < A &&
                u.isDark(B, Math.floor((E - $) / 1)) &&
                (D = " "),
              $ <= E &&
              E < A &&
              $ <= R + 1 &&
              R + 1 < A &&
              u.isDark(S, Math.floor((E - $) / 1))
                ? (D += " ")
                : (D += "\u2588"),
              (U += g < 1 && R + 1 >= A ? j[D] : I[D]);
          U += `
`;
        }
        return m % 2 && g > 0
          ? U.substring(0, U.length - m - 1) + Array(m + 1).join("\u2580")
          : U.substring(0, U.length - 1);
      };
    return (
      (u.createASCII = function (g, w) {
        if (((g = g || 1), g < 2)) return T(w);
        (g -= 1), (w = typeof w > "u" ? g * 2 : w);
        let m = u.getModuleCount() * g + w * 2,
          $ = w,
          A = m - w,
          R,
          E,
          B,
          S,
          D = Array(g + 1).join("\u2588\u2588"),
          I = Array(g + 1).join("  "),
          j = "",
          U = "";
        for (R = 0; R < m; R += 1) {
          for (B = Math.floor((R - $) / g), U = "", E = 0; E < m; E += 1)
            (S = 1),
              $ <= E &&
                E < A &&
                $ <= R &&
                R < A &&
                u.isDark(B, Math.floor((E - $) / g)) &&
                (S = 0),
              (U += S ? D : I);
          for (B = 0; B < g; B += 1)
            j +=
              U +
              `
`;
        }
        return j.substring(0, j.length - 1);
      }),
      (u.renderTo2dContext = function (g, w) {
        w = w || 2;
        let m = u.getModuleCount();
        for (let $ = 0; $ < m; $++)
          for (let A = 0; A < m; A++)
            (g.fillStyle = u.isDark($, A) ? "black" : "white"),
              g.fillRect(A * w, $ * w, w, w);
      }),
      u
    );
  };
  nn.stringToBytes = function (t) {
    let e = [];
    for (let a = 0; a < t.length; a += 1) {
      let n = t.charCodeAt(a);
      e.push(n & 255);
    }
    return e;
  };
  nn.createStringToBytes = function (t, e) {
    let a = (function () {
        let s = Fh(t),
          o = function () {
            let c = s.read();
            if (c == -1) throw "eof";
            return c;
          },
          r = 0,
          i = {};
        for (;;) {
          let c = s.read();
          if (c == -1) break;
          let l = o(),
            u = o(),
            d = o(),
            f = String.fromCharCode((c << 8) | l),
            p = (u << 8) | d;
          (i[f] = p), (r += 1);
        }
        if (r != e) throw r + " != " + e;
        return i;
      })(),
      n = 63;
    return function (s) {
      let o = [];
      for (let r = 0; r < s.length; r += 1) {
        let i = s.charCodeAt(r);
        if (i < 128) o.push(i);
        else {
          let c = a[s.charAt(r)];
          typeof c == "number"
            ? (c & 255) == c
              ? o.push(c)
              : (o.push(c >>> 8), o.push(c & 255))
            : o.push(n);
        }
      }
      return o;
    };
  };
  var Te = {
      MODE_NUMBER: 1,
      MODE_ALPHA_NUM: 2,
      MODE_8BIT_BYTE: 4,
      MODE_KANJI: 8,
    },
    zn = { L: 1, M: 0, Q: 3, H: 2 },
    Ft = {
      PATTERN000: 0,
      PATTERN001: 1,
      PATTERN010: 2,
      PATTERN011: 3,
      PATTERN100: 4,
      PATTERN101: 5,
      PATTERN110: 6,
      PATTERN111: 7,
    },
    Wt = (function () {
      let t = [
          [],
          [6, 18],
          [6, 22],
          [6, 26],
          [6, 30],
          [6, 34],
          [6, 22, 38],
          [6, 24, 42],
          [6, 26, 46],
          [6, 28, 50],
          [6, 30, 54],
          [6, 32, 58],
          [6, 34, 62],
          [6, 26, 46, 66],
          [6, 26, 48, 70],
          [6, 26, 50, 74],
          [6, 30, 54, 78],
          [6, 30, 56, 82],
          [6, 30, 58, 86],
          [6, 34, 62, 90],
          [6, 28, 50, 72, 94],
          [6, 26, 50, 74, 98],
          [6, 30, 54, 78, 102],
          [6, 28, 54, 80, 106],
          [6, 32, 58, 84, 110],
          [6, 30, 58, 86, 114],
          [6, 34, 62, 90, 118],
          [6, 26, 50, 74, 98, 122],
          [6, 30, 54, 78, 102, 126],
          [6, 26, 52, 78, 104, 130],
          [6, 30, 56, 82, 108, 134],
          [6, 34, 60, 86, 112, 138],
          [6, 30, 58, 86, 114, 142],
          [6, 34, 62, 90, 118, 146],
          [6, 30, 54, 78, 102, 126, 150],
          [6, 24, 50, 76, 102, 128, 154],
          [6, 28, 54, 80, 106, 132, 158],
          [6, 32, 58, 84, 110, 136, 162],
          [6, 26, 54, 82, 110, 138, 166],
          [6, 30, 58, 86, 114, 142, 170],
        ],
        e = 1335,
        a = 7973,
        n = 21522,
        s = {},
        o = function (r) {
          let i = 0;
          for (; r != 0; ) (i += 1), (r >>>= 1);
          return i;
        };
      return (
        (s.getBCHTypeInfo = function (r) {
          let i = r << 10;
          for (; o(i) - o(e) >= 0; ) i ^= e << (o(i) - o(e));
          return ((r << 10) | i) ^ n;
        }),
        (s.getBCHTypeNumber = function (r) {
          let i = r << 12;
          for (; o(i) - o(a) >= 0; ) i ^= a << (o(i) - o(a));
          return (r << 12) | i;
        }),
        (s.getPatternPosition = function (r) {
          return t[r - 1];
        }),
        (s.getMaskFunction = function (r) {
          switch (r) {
            case Ft.PATTERN000:
              return function (i, c) {
                return (i + c) % 2 == 0;
              };
            case Ft.PATTERN001:
              return function (i, c) {
                return i % 2 == 0;
              };
            case Ft.PATTERN010:
              return function (i, c) {
                return c % 3 == 0;
              };
            case Ft.PATTERN011:
              return function (i, c) {
                return (i + c) % 3 == 0;
              };
            case Ft.PATTERN100:
              return function (i, c) {
                return (Math.floor(i / 2) + Math.floor(c / 3)) % 2 == 0;
              };
            case Ft.PATTERN101:
              return function (i, c) {
                return ((i * c) % 2) + ((i * c) % 3) == 0;
              };
            case Ft.PATTERN110:
              return function (i, c) {
                return (((i * c) % 2) + ((i * c) % 3)) % 2 == 0;
              };
            case Ft.PATTERN111:
              return function (i, c) {
                return (((i * c) % 3) + ((i + c) % 2)) % 2 == 0;
              };
            default:
              throw "bad maskPattern:" + r;
          }
        }),
        (s.getErrorCorrectPolynomial = function (r) {
          let i = Gn([1], 0);
          for (let c = 0; c < r; c += 1) i = i.multiply(Gn([1, Yt.gexp(c)], 0));
          return i;
        }),
        (s.getLengthInBits = function (r, i) {
          if (1 <= i && i < 10)
            switch (r) {
              case Te.MODE_NUMBER:
                return 10;
              case Te.MODE_ALPHA_NUM:
                return 9;
              case Te.MODE_8BIT_BYTE:
                return 8;
              case Te.MODE_KANJI:
                return 8;
              default:
                throw "mode:" + r;
            }
          else if (i < 27)
            switch (r) {
              case Te.MODE_NUMBER:
                return 12;
              case Te.MODE_ALPHA_NUM:
                return 11;
              case Te.MODE_8BIT_BYTE:
                return 16;
              case Te.MODE_KANJI:
                return 10;
              default:
                throw "mode:" + r;
            }
          else if (i < 41)
            switch (r) {
              case Te.MODE_NUMBER:
                return 14;
              case Te.MODE_ALPHA_NUM:
                return 13;
              case Te.MODE_8BIT_BYTE:
                return 16;
              case Te.MODE_KANJI:
                return 12;
              default:
                throw "mode:" + r;
            }
          else throw "type:" + i;
        }),
        (s.getLostPoint = function (r) {
          let i = r.getModuleCount(),
            c = 0;
          for (let d = 0; d < i; d += 1)
            for (let f = 0; f < i; f += 1) {
              let p = 0,
                v = r.isDark(d, f);
              for (let k = -1; k <= 1; k += 1)
                if (!(d + k < 0 || i <= d + k))
                  for (let L = -1; L <= 1; L += 1)
                    f + L < 0 ||
                      i <= f + L ||
                      (k == 0 && L == 0) ||
                      (v == r.isDark(d + k, f + L) && (p += 1));
              p > 5 && (c += 3 + p - 5);
            }
          for (let d = 0; d < i - 1; d += 1)
            for (let f = 0; f < i - 1; f += 1) {
              let p = 0;
              r.isDark(d, f) && (p += 1),
                r.isDark(d + 1, f) && (p += 1),
                r.isDark(d, f + 1) && (p += 1),
                r.isDark(d + 1, f + 1) && (p += 1),
                (p == 0 || p == 4) && (c += 3);
            }
          for (let d = 0; d < i; d += 1)
            for (let f = 0; f < i - 6; f += 1)
              r.isDark(d, f) &&
                !r.isDark(d, f + 1) &&
                r.isDark(d, f + 2) &&
                r.isDark(d, f + 3) &&
                r.isDark(d, f + 4) &&
                !r.isDark(d, f + 5) &&
                r.isDark(d, f + 6) &&
                (c += 40);
          for (let d = 0; d < i; d += 1)
            for (let f = 0; f < i - 6; f += 1)
              r.isDark(f, d) &&
                !r.isDark(f + 1, d) &&
                r.isDark(f + 2, d) &&
                r.isDark(f + 3, d) &&
                r.isDark(f + 4, d) &&
                !r.isDark(f + 5, d) &&
                r.isDark(f + 6, d) &&
                (c += 40);
          let l = 0;
          for (let d = 0; d < i; d += 1)
            for (let f = 0; f < i; f += 1) r.isDark(f, d) && (l += 1);
          let u = Math.abs((100 * l) / i / i - 50) / 5;
          return (c += u * 10), c;
        }),
        s
      );
    })(),
    Yt = (function () {
      let t = new Array(256),
        e = new Array(256);
      for (let n = 0; n < 8; n += 1) t[n] = 1 << n;
      for (let n = 8; n < 256; n += 1)
        t[n] = t[n - 4] ^ t[n - 5] ^ t[n - 6] ^ t[n - 8];
      for (let n = 0; n < 255; n += 1) e[t[n]] = n;
      let a = {};
      return (
        (a.glog = function (n) {
          if (n < 1) throw "glog(" + n + ")";
          return e[n];
        }),
        (a.gexp = function (n) {
          for (; n < 0; ) n += 255;
          for (; n >= 256; ) n -= 255;
          return t[n];
        }),
        a
      );
    })(),
    Gn = function (t, e) {
      if (typeof t.length > "u") throw t.length + "/" + e;
      let a = (function () {
          let s = 0;
          for (; s < t.length && t[s] == 0; ) s += 1;
          let o = new Array(t.length - s + e);
          for (let r = 0; r < t.length - s; r += 1) o[r] = t[r + s];
          return o;
        })(),
        n = {};
      return (
        (n.getAt = function (s) {
          return a[s];
        }),
        (n.getLength = function () {
          return a.length;
        }),
        (n.multiply = function (s) {
          let o = new Array(n.getLength() + s.getLength() - 1);
          for (let r = 0; r < n.getLength(); r += 1)
            for (let i = 0; i < s.getLength(); i += 1)
              o[r + i] ^= Yt.gexp(Yt.glog(n.getAt(r)) + Yt.glog(s.getAt(i)));
          return Gn(o, 0);
        }),
        (n.mod = function (s) {
          if (n.getLength() - s.getLength() < 0) return n;
          let o = Yt.glog(n.getAt(0)) - Yt.glog(s.getAt(0)),
            r = new Array(n.getLength());
          for (let i = 0; i < n.getLength(); i += 1) r[i] = n.getAt(i);
          for (let i = 0; i < s.getLength(); i += 1)
            r[i] ^= Yt.gexp(Yt.glog(s.getAt(i)) + o);
          return Gn(r, 0).mod(s);
        }),
        n
      );
    },
    hu = (function () {
      let t = [
          [1, 26, 19],
          [1, 26, 16],
          [1, 26, 13],
          [1, 26, 9],
          [1, 44, 34],
          [1, 44, 28],
          [1, 44, 22],
          [1, 44, 16],
          [1, 70, 55],
          [1, 70, 44],
          [2, 35, 17],
          [2, 35, 13],
          [1, 100, 80],
          [2, 50, 32],
          [2, 50, 24],
          [4, 25, 9],
          [1, 134, 108],
          [2, 67, 43],
          [2, 33, 15, 2, 34, 16],
          [2, 33, 11, 2, 34, 12],
          [2, 86, 68],
          [4, 43, 27],
          [4, 43, 19],
          [4, 43, 15],
          [2, 98, 78],
          [4, 49, 31],
          [2, 32, 14, 4, 33, 15],
          [4, 39, 13, 1, 40, 14],
          [2, 121, 97],
          [2, 60, 38, 2, 61, 39],
          [4, 40, 18, 2, 41, 19],
          [4, 40, 14, 2, 41, 15],
          [2, 146, 116],
          [3, 58, 36, 2, 59, 37],
          [4, 36, 16, 4, 37, 17],
          [4, 36, 12, 4, 37, 13],
          [2, 86, 68, 2, 87, 69],
          [4, 69, 43, 1, 70, 44],
          [6, 43, 19, 2, 44, 20],
          [6, 43, 15, 2, 44, 16],
          [4, 101, 81],
          [1, 80, 50, 4, 81, 51],
          [4, 50, 22, 4, 51, 23],
          [3, 36, 12, 8, 37, 13],
          [2, 116, 92, 2, 117, 93],
          [6, 58, 36, 2, 59, 37],
          [4, 46, 20, 6, 47, 21],
          [7, 42, 14, 4, 43, 15],
          [4, 133, 107],
          [8, 59, 37, 1, 60, 38],
          [8, 44, 20, 4, 45, 21],
          [12, 33, 11, 4, 34, 12],
          [3, 145, 115, 1, 146, 116],
          [4, 64, 40, 5, 65, 41],
          [11, 36, 16, 5, 37, 17],
          [11, 36, 12, 5, 37, 13],
          [5, 109, 87, 1, 110, 88],
          [5, 65, 41, 5, 66, 42],
          [5, 54, 24, 7, 55, 25],
          [11, 36, 12, 7, 37, 13],
          [5, 122, 98, 1, 123, 99],
          [7, 73, 45, 3, 74, 46],
          [15, 43, 19, 2, 44, 20],
          [3, 45, 15, 13, 46, 16],
          [1, 135, 107, 5, 136, 108],
          [10, 74, 46, 1, 75, 47],
          [1, 50, 22, 15, 51, 23],
          [2, 42, 14, 17, 43, 15],
          [5, 150, 120, 1, 151, 121],
          [9, 69, 43, 4, 70, 44],
          [17, 50, 22, 1, 51, 23],
          [2, 42, 14, 19, 43, 15],
          [3, 141, 113, 4, 142, 114],
          [3, 70, 44, 11, 71, 45],
          [17, 47, 21, 4, 48, 22],
          [9, 39, 13, 16, 40, 14],
          [3, 135, 107, 5, 136, 108],
          [3, 67, 41, 13, 68, 42],
          [15, 54, 24, 5, 55, 25],
          [15, 43, 15, 10, 44, 16],
          [4, 144, 116, 4, 145, 117],
          [17, 68, 42],
          [17, 50, 22, 6, 51, 23],
          [19, 46, 16, 6, 47, 17],
          [2, 139, 111, 7, 140, 112],
          [17, 74, 46],
          [7, 54, 24, 16, 55, 25],
          [34, 37, 13],
          [4, 151, 121, 5, 152, 122],
          [4, 75, 47, 14, 76, 48],
          [11, 54, 24, 14, 55, 25],
          [16, 45, 15, 14, 46, 16],
          [6, 147, 117, 4, 148, 118],
          [6, 73, 45, 14, 74, 46],
          [11, 54, 24, 16, 55, 25],
          [30, 46, 16, 2, 47, 17],
          [8, 132, 106, 4, 133, 107],
          [8, 75, 47, 13, 76, 48],
          [7, 54, 24, 22, 55, 25],
          [22, 45, 15, 13, 46, 16],
          [10, 142, 114, 2, 143, 115],
          [19, 74, 46, 4, 75, 47],
          [28, 50, 22, 6, 51, 23],
          [33, 46, 16, 4, 47, 17],
          [8, 152, 122, 4, 153, 123],
          [22, 73, 45, 3, 74, 46],
          [8, 53, 23, 26, 54, 24],
          [12, 45, 15, 28, 46, 16],
          [3, 147, 117, 10, 148, 118],
          [3, 73, 45, 23, 74, 46],
          [4, 54, 24, 31, 55, 25],
          [11, 45, 15, 31, 46, 16],
          [7, 146, 116, 7, 147, 117],
          [21, 73, 45, 7, 74, 46],
          [1, 53, 23, 37, 54, 24],
          [19, 45, 15, 26, 46, 16],
          [5, 145, 115, 10, 146, 116],
          [19, 75, 47, 10, 76, 48],
          [15, 54, 24, 25, 55, 25],
          [23, 45, 15, 25, 46, 16],
          [13, 145, 115, 3, 146, 116],
          [2, 74, 46, 29, 75, 47],
          [42, 54, 24, 1, 55, 25],
          [23, 45, 15, 28, 46, 16],
          [17, 145, 115],
          [10, 74, 46, 23, 75, 47],
          [10, 54, 24, 35, 55, 25],
          [19, 45, 15, 35, 46, 16],
          [17, 145, 115, 1, 146, 116],
          [14, 74, 46, 21, 75, 47],
          [29, 54, 24, 19, 55, 25],
          [11, 45, 15, 46, 46, 16],
          [13, 145, 115, 6, 146, 116],
          [14, 74, 46, 23, 75, 47],
          [44, 54, 24, 7, 55, 25],
          [59, 46, 16, 1, 47, 17],
          [12, 151, 121, 7, 152, 122],
          [12, 75, 47, 26, 76, 48],
          [39, 54, 24, 14, 55, 25],
          [22, 45, 15, 41, 46, 16],
          [6, 151, 121, 14, 152, 122],
          [6, 75, 47, 34, 76, 48],
          [46, 54, 24, 10, 55, 25],
          [2, 45, 15, 64, 46, 16],
          [17, 152, 122, 4, 153, 123],
          [29, 74, 46, 14, 75, 47],
          [49, 54, 24, 10, 55, 25],
          [24, 45, 15, 46, 46, 16],
          [4, 152, 122, 18, 153, 123],
          [13, 74, 46, 32, 75, 47],
          [48, 54, 24, 14, 55, 25],
          [42, 45, 15, 32, 46, 16],
          [20, 147, 117, 4, 148, 118],
          [40, 75, 47, 7, 76, 48],
          [43, 54, 24, 22, 55, 25],
          [10, 45, 15, 67, 46, 16],
          [19, 148, 118, 6, 149, 119],
          [18, 75, 47, 31, 76, 48],
          [34, 54, 24, 34, 55, 25],
          [20, 45, 15, 61, 46, 16],
        ],
        e = function (s, o) {
          let r = {};
          return (r.totalCount = s), (r.dataCount = o), r;
        },
        a = {},
        n = function (s, o) {
          switch (o) {
            case zn.L:
              return t[(s - 1) * 4 + 0];
            case zn.M:
              return t[(s - 1) * 4 + 1];
            case zn.Q:
              return t[(s - 1) * 4 + 2];
            case zn.H:
              return t[(s - 1) * 4 + 3];
            default:
              return;
          }
        };
      return (
        (a.getRSBlocks = function (s, o) {
          let r = n(s, o);
          if (typeof r > "u")
            throw (
              "bad rs block @ typeNumber:" + s + "/errorCorrectionLevel:" + o
            );
          let i = r.length / 3,
            c = [];
          for (let l = 0; l < i; l += 1) {
            let u = r[l * 3 + 0],
              d = r[l * 3 + 1],
              f = r[l * 3 + 2];
            for (let p = 0; p < u; p += 1) c.push(e(d, f));
          }
          return c;
        }),
        a
      );
    })(),
    mu = function () {
      let t = [],
        e = 0,
        a = {};
      return (
        (a.getBuffer = function () {
          return t;
        }),
        (a.getAt = function (n) {
          let s = Math.floor(n / 8);
          return ((t[s] >>> (7 - (n % 8))) & 1) == 1;
        }),
        (a.put = function (n, s) {
          for (let o = 0; o < s; o += 1)
            a.putBit(((n >>> (s - o - 1)) & 1) == 1);
        }),
        (a.getLengthInBits = function () {
          return e;
        }),
        (a.putBit = function (n) {
          let s = Math.floor(e / 8);
          t.length <= s && t.push(0), n && (t[s] |= 128 >>> e % 8), (e += 1);
        }),
        a
      );
    },
    Ph = function (t) {
      let e = Te.MODE_NUMBER,
        a = t,
        n = {};
      (n.getMode = function () {
        return e;
      }),
        (n.getLength = function (r) {
          return a.length;
        }),
        (n.write = function (r) {
          let i = a,
            c = 0;
          for (; c + 2 < i.length; )
            r.put(s(i.substring(c, c + 3)), 10), (c += 3);
          c < i.length &&
            (i.length - c == 1
              ? r.put(s(i.substring(c, c + 1)), 4)
              : i.length - c == 2 && r.put(s(i.substring(c, c + 2)), 7));
        });
      let s = function (r) {
          let i = 0;
          for (let c = 0; c < r.length; c += 1) i = i * 10 + o(r.charAt(c));
          return i;
        },
        o = function (r) {
          if ("0" <= r && r <= "9") return r.charCodeAt(0) - 48;
          throw "illegal char :" + r;
        };
      return n;
    },
    Vh = function (t) {
      let e = Te.MODE_ALPHA_NUM,
        a = t,
        n = {};
      (n.getMode = function () {
        return e;
      }),
        (n.getLength = function (o) {
          return a.length;
        }),
        (n.write = function (o) {
          let r = a,
            i = 0;
          for (; i + 1 < r.length; )
            o.put(s(r.charAt(i)) * 45 + s(r.charAt(i + 1)), 11), (i += 2);
          i < r.length && o.put(s(r.charAt(i)), 6);
        });
      let s = function (o) {
        if ("0" <= o && o <= "9") return o.charCodeAt(0) - 48;
        if ("A" <= o && o <= "Z") return o.charCodeAt(0) - 65 + 10;
        switch (o) {
          case " ":
            return 36;
          case "$":
            return 37;
          case "%":
            return 38;
          case "*":
            return 39;
          case "+":
            return 40;
          case "-":
            return 41;
          case ".":
            return 42;
          case "/":
            return 43;
          case ":":
            return 44;
          default:
            throw "illegal char :" + o;
        }
      };
      return n;
    },
    zh = function (t) {
      let e = Te.MODE_8BIT_BYTE,
        a = t,
        n = nn.stringToBytes(t),
        s = {};
      return (
        (s.getMode = function () {
          return e;
        }),
        (s.getLength = function (o) {
          return n.length;
        }),
        (s.write = function (o) {
          for (let r = 0; r < n.length; r += 1) o.put(n[r], 8);
        }),
        s
      );
    },
    Gh = function (t) {
      let e = Te.MODE_KANJI,
        a = t,
        n = nn.stringToBytes;
      (function (r, i) {
        let c = n(r);
        if (c.length != 2 || ((c[0] << 8) | c[1]) != i)
          throw "sjis not supported.";
      })("\u53CB", 38726);
      let s = n(t),
        o = {};
      return (
        (o.getMode = function () {
          return e;
        }),
        (o.getLength = function (r) {
          return ~~(s.length / 2);
        }),
        (o.write = function (r) {
          let i = s,
            c = 0;
          for (; c + 1 < i.length; ) {
            let l = ((255 & i[c]) << 8) | (255 & i[c + 1]);
            if (33088 <= l && l <= 40956) l -= 33088;
            else if (57408 <= l && l <= 60351) l -= 49472;
            else throw "illegal char at " + (c + 1) + "/" + l;
            (l = ((l >>> 8) & 255) * 192 + (l & 255)), r.put(l, 13), (c += 2);
          }
          if (c < i.length) throw "illegal char at " + (c + 1);
        }),
        o
      );
    },
    bu = function () {
      let t = [],
        e = {};
      return (
        (e.writeByte = function (a) {
          t.push(a & 255);
        }),
        (e.writeShort = function (a) {
          e.writeByte(a), e.writeByte(a >>> 8);
        }),
        (e.writeBytes = function (a, n, s) {
          (n = n || 0), (s = s || a.length);
          for (let o = 0; o < s; o += 1) e.writeByte(a[o + n]);
        }),
        (e.writeString = function (a) {
          for (let n = 0; n < a.length; n += 1) e.writeByte(a.charCodeAt(n));
        }),
        (e.toByteArray = function () {
          return t;
        }),
        (e.toString = function () {
          let a = "";
          a += "[";
          for (let n = 0; n < t.length; n += 1)
            n > 0 && (a += ","), (a += t[n]);
          return (a += "]"), a;
        }),
        e
      );
    },
    Kh = function () {
      let t = 0,
        e = 0,
        a = 0,
        n = "",
        s = {},
        o = function (i) {
          n += String.fromCharCode(r(i & 63));
        },
        r = function (i) {
          if (i < 0) throw "n:" + i;
          if (i < 26) return 65 + i;
          if (i < 52) return 97 + (i - 26);
          if (i < 62) return 48 + (i - 52);
          if (i == 62) return 43;
          if (i == 63) return 47;
          throw "n:" + i;
        };
      return (
        (s.writeByte = function (i) {
          for (t = (t << 8) | (i & 255), e += 8, a += 1; e >= 6; )
            o(t >>> (e - 6)), (e -= 6);
        }),
        (s.flush = function () {
          if ((e > 0 && (o(t << (6 - e)), (t = 0), (e = 0)), a % 3 != 0)) {
            let i = 3 - (a % 3);
            for (let c = 0; c < i; c += 1) n += "=";
          }
        }),
        (s.toString = function () {
          return n;
        }),
        s
      );
    },
    Fh = function (t) {
      let e = t,
        a = 0,
        n = 0,
        s = 0,
        o = {};
      o.read = function () {
        for (; s < 8; ) {
          if (a >= e.length) {
            if (s == 0) return -1;
            throw "unexpected end of file./" + s;
          }
          let c = e.charAt(a);
          if (((a += 1), c == "=")) return (s = 0), -1;
          if (c.match(/^\s$/)) continue;
          (n = (n << 6) | r(c.charCodeAt(0))), (s += 6);
        }
        let i = (n >>> (s - 8)) & 255;
        return (s -= 8), i;
      };
      let r = function (i) {
        if (65 <= i && i <= 90) return i - 65;
        if (97 <= i && i <= 122) return i - 97 + 26;
        if (48 <= i && i <= 57) return i - 48 + 52;
        if (i == 43) return 62;
        if (i == 47) return 63;
        throw "c:" + i;
      };
      return o;
    },
    Wh = function (t, e) {
      let a = t,
        n = e,
        s = new Array(t * e),
        o = {};
      (o.setPixel = function (l, u, d) {
        s[u * a + l] = d;
      }),
        (o.write = function (l) {
          l.writeString("GIF87a"),
            l.writeShort(a),
            l.writeShort(n),
            l.writeByte(128),
            l.writeByte(0),
            l.writeByte(0),
            l.writeByte(0),
            l.writeByte(0),
            l.writeByte(0),
            l.writeByte(255),
            l.writeByte(255),
            l.writeByte(255),
            l.writeString(","),
            l.writeShort(0),
            l.writeShort(0),
            l.writeShort(a),
            l.writeShort(n),
            l.writeByte(0);
          let u = 2,
            d = i(u);
          l.writeByte(u);
          let f = 0;
          for (; d.length - f > 255; )
            l.writeByte(255), l.writeBytes(d, f, 255), (f += 255);
          l.writeByte(d.length - f),
            l.writeBytes(d, f, d.length - f),
            l.writeByte(0),
            l.writeString(";");
        });
      let r = function (l) {
          let u = l,
            d = 0,
            f = 0,
            p = {};
          return (
            (p.write = function (v, k) {
              if (v >>> k) throw "length over";
              for (; d + k >= 8; )
                u.writeByte(255 & ((v << d) | f)),
                  (k -= 8 - d),
                  (v >>>= 8 - d),
                  (f = 0),
                  (d = 0);
              (f = (v << d) | f), (d = d + k);
            }),
            (p.flush = function () {
              d > 0 && u.writeByte(f);
            }),
            p
          );
        },
        i = function (l) {
          let u = 1 << l,
            d = (1 << l) + 1,
            f = l + 1,
            p = c();
          for (let V = 0; V < u; V += 1) p.add(String.fromCharCode(V));
          p.add(String.fromCharCode(u)), p.add(String.fromCharCode(d));
          let v = bu(),
            k = r(v);
          k.write(u, f);
          let L = 0,
            N = String.fromCharCode(s[L]);
          for (L += 1; L < s.length; ) {
            let V = String.fromCharCode(s[L]);
            (L += 1),
              p.contains(N + V)
                ? (N = N + V)
                : (k.write(p.indexOf(N), f),
                  p.size() < 4095 &&
                    (p.size() == 1 << f && (f += 1), p.add(N + V)),
                  (N = V));
          }
          return (
            k.write(p.indexOf(N), f), k.write(d, f), k.flush(), v.toByteArray()
          );
        },
        c = function () {
          let l = {},
            u = 0,
            d = {};
          return (
            (d.add = function (f) {
              if (d.contains(f)) throw "dup key:" + f;
              (l[f] = u), (u += 1);
            }),
            (d.size = function () {
              return u;
            }),
            (d.indexOf = function (f) {
              return l[f];
            }),
            (d.contains = function (f) {
              return typeof l[f] < "u";
            }),
            d
          );
        };
      return o;
    },
    Yh = function (t, e, a) {
      let n = Wh(t, e);
      for (let i = 0; i < e; i += 1)
        for (let c = 0; c < t; c += 1) n.setPixel(c, i, a(c, i));
      let s = bu();
      n.write(s);
      let o = Kh(),
        r = s.toByteArray();
      for (let i = 0; i < r.length; i += 1) o.writeByte(r[i]);
      return o.flush(), "data:image/gif;base64," + o;
    },
    gu = nn,
    t1 = nn.stringToBytes;
  var ki = null;
  function Zh() {
    return window.jsQR
      ? Promise.resolve(window.jsQR)
      : (ki ||
          (ki = new Promise((t, e) => {
            let a = document.createElement("script");
            (a.src = "jsqr.js"),
              (a.onload = () => t(window.jsQR)),
              (a.onerror = e),
              document.head.appendChild(a);
          })),
        ki);
  }
  var $i = { echoCancellation: !0, noiseSuppression: !0, autoGainControl: !0 };
  async function Jh() {
    return navigator.mediaDevices.getUserMedia({ audio: $i });
  }
  function ze(t) {
    let e = t && t.name;
    return e === "NotAllowedError" || e === "SecurityError"
      ? "Permita o acesso nas configura\xE7\xF5es do Android para usar isso."
      : e === "NotFoundError" || e === "OverconstrainedError"
        ? "Nenhum dispositivo encontrado neste aparelho."
        : e === "NotReadableError"
          ? "Outro app est\xE1 usando o microfone ou a c\xE2mera."
          : "N\xE3o foi poss\xEDvel acessar o microfone ou a c\xE2mera.";
  }
  var Xh = [
      "audio/webm;codecs=opus",
      "audio/ogg;codecs=opus",
      "audio/webm",
      "audio/mp4",
    ],
    fo = class {
      async start(e) {
        (this.stream = await Jh()),
          (this.mime =
            Xh.find(
              (n) => window.MediaRecorder && MediaRecorder.isTypeSupported(n),
            ) || "");
        let a = { audioBitsPerSecond: 24e3 };
        this.mime && (a.mimeType = this.mime),
          (this.rec = new MediaRecorder(this.stream, a)),
          (this.parts = []),
          (this.levels = []),
          (this.rec.ondataavailable = (n) => {
            n.data && n.data.size && this.parts.push(n.data);
          }),
          (this.t0 = performance.now()),
          this.rec.start(250);
        try {
          let n = window.AudioContext || window.webkitAudioContext;
          this.ac = new n();
          let s = this.ac.createMediaStreamSource(this.stream);
          (this.an = this.ac.createAnalyser()),
            (this.an.fftSize = 512),
            s.connect(this.an);
          let o = new Uint8Array(this.an.fftSize);
          this.lt = setInterval(() => {
            this.an.getByteTimeDomainData(o);
            let r = 0;
            for (let c = 0; c < o.length; c++)
              r = Math.max(r, Math.abs(o[c] - 128));
            let i = Math.min(1, r / 64);
            this.levels.push(i), e && e(i);
          }, 90);
        } catch {}
      }
      elapsed() {
        return (performance.now() - this.t0) / 1e3;
      }
      cleanup() {
        clearInterval(this.lt);
        try {
          this.ac && this.ac.close();
        } catch {}
        this.stream && this.stream.getTracks().forEach((e) => e.stop());
      }
      stop() {
        return new Promise((e) => {
          let a = this.elapsed();
          this.rec.onstop = () => {
            this.cleanup();
            let n =
                (this.rec.mimeType || this.mime || "audio/webm").split(";")[0] +
                (/(webm|ogg)/.test(this.rec.mimeType || this.mime)
                  ? ";codecs=opus"
                  : ""),
              s = new Blob(this.parts, { type: n.split(";")[0] });
            e({
              blob: s,
              mime: n.split(";")[0],
              dur: a,
              wf: Qh(this.levels, 40),
            });
          };
          try {
            this.rec.stop();
          } catch {
            this.cleanup(), e(null);
          }
        });
      }
      cancel() {
        try {
          (this.rec.onstop = null), this.rec.stop();
        } catch {}
        this.cleanup();
      }
    };
  function Qh(t, e) {
    if (!t.length) return Array(e).fill(4);
    let a = [];
    for (let n = 0; n < e; n++) {
      let s = Math.floor((n * t.length) / e),
        o = Math.max(s + 1, Math.floor(((n + 1) * t.length) / e)),
        r = 0;
      for (let i = s; i < o && i < t.length; i++) r = Math.max(r, t[i]);
      a.push(Math.max(2, Math.min(31, Math.round(Math.sqrt(r) * 31))));
    }
    return a;
  }
  function Ei(t, e, a) {
    return new Promise((n) => t.toBlob((s) => n(s), e, a));
  }
  async function xu(t, { max: e = 1600, target: a = 32e4 } = {}) {
    let n = await createImageBitmap(t),
      s = Math.min(1, e / Math.max(n.width, n.height)),
      o = null,
      r = 0,
      i = 0,
      c = "image/webp",
      l = document.createElement("canvas");
    l.width = l.height = 1;
    let u = ((await Ei(l, "image/webp", 0.8)) || {}).type === "image/webp";
    u || (c = "image/jpeg");
    for (let k = 0; k < 6; k++) {
      (r = Math.max(1, Math.round(n.width * s))),
        (i = Math.max(1, Math.round(n.height * s)));
      let L = document.createElement("canvas");
      (L.width = r),
        (L.height = i),
        L.getContext("2d").drawImage(n, 0, 0, r, i);
      let V = 0.82;
      for (o = await Ei(L, c, V); o.size > a && V > 0.45; )
        (V -= 0.12), (o = await Ei(L, c, V));
      if (o.size <= a) break;
      s *= 0.78;
    }
    let d = Math.max(1, Math.round((r * 24) / Math.max(r, i))),
      f = Math.max(1, Math.round((i * 24) / Math.max(r, i))),
      p = document.createElement("canvas");
    (p.width = d), (p.height = f), p.getContext("2d").drawImage(n, 0, 0, d, f);
    let v = p.toDataURL(u ? "image/webp" : "image/jpeg", 0.5);
    return (
      n.close && n.close(),
      {
        bytes: new Uint8Array(await o.arrayBuffer()),
        mime: c,
        w: r,
        h: i,
        th: v,
      }
    );
  }
  async function Ai(t = "environment", e = !0) {
    return navigator.mediaDevices.getUserMedia({
      video: { facingMode: t, width: { ideal: 1280 }, height: { ideal: 960 } },
      audio: !1,
    });
  }
  async function yu(t) {
    let e = document.createElement("canvas");
    return (
      (e.width = t.videoWidth),
      (e.height = t.videoHeight),
      e.getContext("2d").drawImage(t, 0, 0),
      new Promise((a) => e.toBlob((n) => a(n), "image/jpeg", 0.92))
    );
  }
  function vu(t) {
    let e = gu(0, "M");
    e.addData(t), e.make();
    let a = e.getModuleCount(),
      n = 3,
      s = "";
    for (let o = 0; o < a; o++)
      for (let r = 0; r < a; r++)
        e.isDark(o, r) && (s += `M${r + n} ${o + n}h1v1h-1z`);
    return `<svg viewBox="0 0 ${a + n * 2} ${a + n * 2}" shape-rendering="crispEdges" role="img" aria-label="C\xF3digo QR"><rect width="100%" height="100%" fill="#fff"/><path d="${s}" fill="#08090a"/></svg>`;
  }
  var po = class {
    async start(e, a) {
      let n = await Zh();
      (this.stream = await Ai("environment")),
        (e.srcObject = this.stream),
        e.setAttribute("playsinline", ""),
        (e.muted = !0),
        await e.play().catch(() => {});
      let s = document.createElement("canvas"),
        o = s.getContext("2d", { willReadFrequently: !0 });
      this.t = setInterval(() => {
        if (!e.videoWidth) return;
        let r = Math.min(1, 520 / Math.max(e.videoWidth, e.videoHeight));
        (s.width = Math.round(e.videoWidth * r)),
          (s.height = Math.round(e.videoHeight * r)),
          o.drawImage(e, 0, 0, s.width, s.height);
        let i = o.getImageData(0, 0, s.width, s.height),
          c = n(i.data, s.width, s.height, {
            inversionAttempts: "attemptBoth",
          });
        c && c.data && (this.stop(), a(c.data));
      }, 180);
    }
    stop() {
      clearInterval(this.t),
        this.stream && this.stream.getTracks().forEach((e) => e.stop()),
        (this.stream = null);
    }
  };
  var Si = 192,
    e0 = 22e3;
  function Kn(t) {
    return new Promise((e) => {
      let a = document.createElement("input");
      (a.type = "file"),
        (a.accept = t),
        (a.onchange = () => e((a.files && a.files[0]) || null)),
        (a.oncancel = () => e(null)),
        a.click();
    });
  }
  function Ci(t = "environment") {
    return new Promise((e) => {
      let a = null,
        n = !1,
        s = (r) => {
          n ||
            ((n = !0),
            a && a.getTracks().forEach((i) => i.stop()),
            (a = null),
            e(r));
        },
        o = async () => {
          a && a.getTracks().forEach((i) => i.stop()), (a = await Ai(t));
          let r = y("#camv");
          r &&
            ((r.srcObject = a),
            r.classList.toggle("mirror", t === "user"),
            r.play().catch(() => {}));
        };
      ee(
        `<div class="camsheet"><video id="camv" autoplay playsinline muted></video>
      <div class="ctl"><button class="ibtn" data-act="camClose" aria-label="Fechar">${x("x")}</button><button class="shutter" data-act="camShoot" aria-label="Tirar foto"></button><button class="ibtn" data-act="camFlip" aria-label="Virar c\xE2mera">${x("flip")}</button></div></div>`,
        { onClose: () => s(null) },
      ),
        b("camFlip", async () => {
          t = t === "environment" ? "user" : "environment";
          try {
            await o();
          } catch {}
        }),
        b("camClose", () => F()),
        b("camShoot", async () => {
          let r = y("#camv");
          if (!r || !r.videoWidth) return;
          let i = await yu(r);
          s(i), F();
        }),
        o().catch((r) => {
          F(), _(ze(r));
        });
    });
  }
  function ho({
    title: t = "Foto",
    hasPic: e = !1,
    shape: a = "round",
    facing: n = "user",
  } = {}) {
    return new Promise((s) => {
      let o = !1,
        r = !1,
        i = (l) => {
          o || ((o = !0), s(l));
        },
        c = async (l) => i(l ? await t0(l, { shape: a }) : null);
      ee(
        `<h2 class="sh-title">${M(t)}</h2><div class="agrid" style="grid-template-columns:repeat(${e ? 3 : 2},1fr)">
      <button class="atile" data-act="phGal"><span class="ab">${x("image")}</span>Galeria</button>
      <button class="atile" data-act="phCam"><span class="ab">${x("camera")}</span>C\xE2mera</button>
      ${e ? `<button class="atile" data-act="phDel"><span class="ab">${x("trash")}</span>Remover</button>` : ""}</div>
      <p class="note" style="padding:14px 2px 0">A foto \xE9 recortada e reduzida no aparelho. Ela vai cifrada s\xF3 para quem j\xE1 fala com voc\xEA.</p>`,
        {
          onClose: () => {
            r || i(null);
          },
        },
      ),
        b("phGal", () => {
          (r = !0), F(), Kn("image/*").then(c);
        }),
        b("phCam", () => {
          (r = !0), F(), setTimeout(() => Ci(n).then(c), 260);
        }),
        b("phDel", () => {
          i(""), F();
        });
    });
  }
  async function t0(t, { shape: e = "round" } = {}) {
    let a;
    try {
      a = await createImageBitmap(t);
    } catch {
      return _("N\xE3o foi poss\xEDvel abrir essa imagem."), null;
    }
    let n = URL.createObjectURL(t);
    return new Promise((s) => {
      let o = !1,
        r = (T) => {
          o || ((o = !0), URL.revokeObjectURL(n), a.close && a.close(), s(T));
        };
      ee(
        `<h2 class="sh-title">Ajustar foto</h2><p class="sh-sub">Arraste para posicionar. Use dois dedos ou a barra para aproximar.</p>
      <div class="cropst ${e}" id="cropst"><img id="cropimg" src="${n}" alt="" draggable="false"><i class="cropmask"></i></div>
      <label class="vol">${x("search")}<input type="range" id="cropz" min="100" max="400" value="100" aria-label="Aproximar"></label>
      <div class="two"><button class="btn ghost" data-act="cropNo" style="margin:0">Cancelar</button><button class="btn" data-act="cropOk" style="margin:0">Usar foto</button></div>`,
        { onClose: () => r(null) },
      );
      let i = y("#cropst"),
        c = y("#cropimg"),
        l = y("#cropz"),
        u = a.width,
        d = a.height,
        f = i.clientWidth || 280,
        fh = e === "banner" ? f / 3 : f,
        p = Math.max(f / u, fh / d),
        v = 1,
        k = (f - u * p) / 2,
        L = (fh - d * p) / 2;
      (c.style.width = u * p + "px"), (c.style.height = d * p + "px");
      let N = () => {
          let T = u * p * v,
            g = d * p * v;
          (k = Math.min(0, Math.max(f - T, k))),
            (L = Math.min(0, Math.max(fh - g, L))),
            (c.style.transform = `translate3d(${k}px,${L}px,0) scale(${v})`);
        },
        V = (T, g, w) => {
          T = Math.max(1, Math.min(4, T));
          let m = T / v;
          (k = g - (g - k) * m),
            (L = w - (w - L) * m),
            (v = T),
            N(),
            (l.value = String(Math.round(v * 100)));
        };
      N();
      let O = new Map(),
        q = null;
      i.addEventListener("pointerdown", (T) => {
        try {
          i.setPointerCapture(T.pointerId);
        } catch {}
        if (
          (O.set(T.pointerId, { x: T.clientX, y: T.clientY }), O.size === 2)
        ) {
          let [g, w] = [...O.values()];
          q = { d: Math.hypot(g.x - w.x, g.y - w.y) || 1, z: v };
        }
      }),
        i.addEventListener("pointermove", (T) => {
          let g = O.get(T.pointerId);
          if (!g) return;
          let w = T.clientX - g.x,
            m = T.clientY - g.y;
          if (((g.x = T.clientX), (g.y = T.clientY), O.size === 1))
            (k += w), (L += m), N();
          else if (q) {
            let [$, A] = [...O.values()],
              R = i.getBoundingClientRect();
            V(
              (q.z * Math.hypot($.x - A.x, $.y - A.y)) / q.d,
              ($.x + A.x) / 2 - R.left,
              ($.y + A.y) / 2 - R.top,
            );
          }
        });
      let K = (T) => {
        O.delete(T.pointerId), O.size < 2 && (q = null);
      };
      i.addEventListener("pointerup", K),
        i.addEventListener("pointercancel", K),
        i.addEventListener(
          "wheel",
          (T) => {
            T.preventDefault();
            let g = i.getBoundingClientRect();
            V(
              v * (T.deltaY < 0 ? 1.1 : 0.9),
              T.clientX - g.left,
              T.clientY - g.top,
            );
          },
          { passive: !1 },
        ),
        l.addEventListener("input", () => V(+l.value / 100, f / 2, f / 2)),
        b("cropNo", () => F()),
        b("cropOk", () => {
          let T = p * v,
            g = document.createElement("canvas");
          if (e === "banner") {
            (g.width = 600), (g.height = 200);
            let w = g.getContext("2d");
            (w.imageSmoothingQuality = "high"),
              w.drawImage(a, -k / T, -L / T, f / T, fh / T, 0, 0, 600, 200),
              r(jmBannerDataUrl(g)),
              F();
            return;
          }
          g.width = g.height = Si;
          let w = g.getContext("2d");
          (w.imageSmoothingQuality = "high"),
            w.drawImage(a, -k / T, -L / T, f / T, f / T, 0, 0, Si, Si);
          let m = a0(g);
          r(m), F();
        });
    });
  }
  /** Banner 600×200 que cabe no limite do protocolo (24 000 caracteres). */
  function jmBannerDataUrl(t) {
    let e = t.toDataURL("image/webp", 0.8).startsWith("data:image/webp") ? "image/webp" : "image/jpeg",
      a = t;
    for (let n of [600, 480, 360, 300]) {
      if (n !== a.width) {
        let o = document.createElement("canvas");
        (o.width = n), (o.height = n / 3), o.getContext("2d").drawImage(t, 0, 0, n, n / 3), (a = o);
      }
      for (let o = 0.82; o >= 0.3; o -= 0.08) {
        let r = a.toDataURL(e, o);
        if (r.length <= JM_BANNER_MAX) return r;
      }
    }
    return null;
  }
  function a0(t) {
    let e = t.toDataURL("image/webp", 0.8).startsWith("data:image/webp")
      ? "image/webp"
      : "image/jpeg";
    for (let n = 0.86; n >= 0.4; n -= 0.08) {
      let s = t.toDataURL(e, n);
      if (s.length <= e0) return s;
    }
    let a = document.createElement("canvas");
    return (
      (a.width = a.height = 128),
      a.getContext("2d").drawImage(t, 0, 0, 128, 128),
      a.toDataURL(e, 0.6)
    );
  }
  var Et = 8,
    mo = 0.3,
    wu = 30,
    ku = [
      "\u{1F50A}",
      "\u{1F602}",
      "\u{1F923}",
      "\u{1F631}",
      "\u{1F62E}",
      "\u{1F92F}",
      "\u{1F62D}",
      "\u{1F60E}",
      "\u{1F44F}",
      "\u{1F389}",
      "\u{1F941}",
      "\u{1F3BA}",
      "\u{1F3B8}",
      "\u{1F3B5}",
      "\u{1F4EF}",
      "\u{1F986}",
      "\u{1F436}",
      "\u{1F431}",
      "\u{1F42E}",
      "\u{1F997}",
      "\u{1F4A5}",
      "\u{1F525}",
      "\u{1F480}",
      "\u{1F47B}",
      "\u{1F916}",
      "\u{1F47D}",
      "\u{1F6A8}",
      "\u23F0",
      "\u{1F4DE}",
      "\u{1F355}",
      "\u{1F4A4}",
      "\u2764\uFE0F",
      "\u{1F44D}",
      "\u{1F44E}",
      "\u{1F648}",
      "\u{1F921}",
    ],
    Fn = (t) => (Math.round(t * 10) / 10).toFixed(1).replace(".", ",") + " s",
    n0 = "preview";
  function Eu(t = "preview") {
    (n0 = t),
      ee(`<h2 class="sh-title">Soundboard</h2><p class="sh-sub">${M(t === "call" ? "Todos na chamada ouvem" : t === "chat" ? "Envie um som para a conversa" : "Toque para ouvir")}</p>
    <div class="sgrid" id="sgrid">${s0(t)}</div>
    <div class="sbfoot"><label class="vol">${x("volume")}<input type="range" id="vol" min="0" max="100" value="${Math.round(xi() * 100)}" aria-label="Volume dos sons"></label>
    <button class="ibtn" data-act="sbManage" aria-label="Gerenciar sons">${x("sliders")}</button></div>`),
      yi();
  }
  function s0(t) {
    return (
      co()
        .map(
          (e) =>
            `<button class="stile ${e.builtin ? "" : "mine"}" style="--d:${e.d}s" data-act="sound" data-mode="${t}" data-id="${e.id}"><span class="semo">${M(e.e)}</span><span>${M(e.n)}</span></button>`,
        )
        .join("") +
      `<button class="stile add" data-act="sbAdd"><span class="semo">${x("plus")}</span><span>Adicionar</span></button>`
    );
  }
  document.addEventListener("input", (t) => {
    if (t.target.id === "vol") {
      lo(+t.target.value / 100);
      try {
        localStorage.setItem("jm.vol", t.target.value);
      } catch {}
    }
  });
  b("sound", async (t) => {
    let e = kt(t.dataset.id),
      a = t.dataset.mode;
    if (
      e &&
      (t.classList.remove("playing"),
      t.offsetWidth,
      t.classList.add("playing"),
      setTimeout(() => t.classList.remove("playing"), e.d * 1e3),
      Kt(e.id),
      a === "call" && P.callSound && P.callSound(e.id),
      a === "chat")
    ) {
      let n = P.currentChat && P.currentChat();
      if (!n) return;
      F();
      try {
        e.builtin
          ? await h.engine.sendSound(n.key, e.id)
          : await h.engine.sendMedia(n.key, await uo(e.id), {
              type: "audio",
              mime: e.mime,
              dur: e.d,
              sb: { n: e.n, e: e.e },
            });
      } catch (s) {
        _(s.message || "N\xE3o foi poss\xEDvel enviar o som.");
      }
    }
  });
  b("sbManage", () => {
    F(), Li();
  });
  function Li() {
    J.open(
      "sounds",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Soundboard</h2></div><div class="scroll" id="sbbody"></div></div>`,
    ),
      Wn(),
      yi();
  }
  function Wn() {
    let t = y("#sbbody");
    if (!t) return;
    let e = co(),
      a = eu();
    t.innerHTML = `<div style="padding:6px 16px 0"><button class="btn" data-act="sbAdd" style="margin:0">${x("plus", "sm")}&nbsp;Adicionar som</button></div>
    <label class="vol" style="margin:14px 16px 4px">${x("volume")}<input type="range" id="vol" min="0" max="100" value="${Math.round(xi() * 100)}" aria-label="Volume dos sons"></label>
    <div class="label">Na sua lista \xB7 ${e.length}</div>
    ${
      e.length
        ? e
            .map(
              (n) => `<div class="sbrow" data-id="${n.id}">
      <button class="pbtn" data-act="sbPlay" data-id="${n.id}" aria-label="Ouvir ${M(n.n)}">${x("play")}</button><span class="semo">${M(n.e)}</span>
      <div class="sbm"><b>${M(n.n)}</b><small>${n.builtin ? "Grava\xE7\xE3o de f\xE1brica" : "Criado por voc\xEA"} \xB7 ${Fn(n.d)}</small></div>
      ${n.builtin ? "" : `<button class="ibtn sm" data-act="sbEdit" data-id="${n.id}" aria-label="Editar ${M(n.n)}">${x("edit", "sm")}</button>`}
      <button class="ibtn sm" data-act="sbDel" data-id="${n.id}" aria-label="Excluir ${M(n.n)}">${x("trash", "sm")}</button></div>`,
            )
            .join("")
        : '<div class="empty">Nenhum som na lista. Adicione um ou restaure os de f\xE1brica.</div>'
    }
    ${
      a.length
        ? `<div class="label">Removidos \xB7 ${a.length}</div>
      <p class="note" style="padding:0 18px">${a.map((n) => M(n.e + " " + n.n)).join(" \xB7 ")}</p>
      <div style="padding:10px 16px 0"><button class="btn ghost" data-act="sbRestore" style="margin:0">${x("refresh", "sm")}&nbsp;Restaurar sons de f\xE1brica</button></div>`
        : ""
    }
    <p class="note" style="padding:16px 18px 30px">Os sons de f\xE1brica s\xE3o grava\xE7\xF5es reais de instrumentos e plateia, com licen\xE7a livre. Os sons que voc\xEA cria ficam neste aparelho. Numa chamada, quem est\xE1 com voc\xEA recebe o som cifrado para poder ouvir.</p>`;
  }
  b("sbPlay", (t) => Kt(t.dataset.id));
  b("sbDel", async (t) => {
    let e = kt(t.dataset.id);
    e &&
      ((!e.builtin &&
        !(await ge({
          title: `Excluir "${e.n}"?`,
          text: "Este som foi criado por voc\xEA e n\xE3o pode ser recuperado.",
          ok: "Excluir",
          danger: !0,
        }))) ||
        (await ru(e.id),
        Wn(),
        _(
          e.builtin
            ? "Som removido da lista. D\xE1 para restaurar depois."
            : "Som exclu\xEDdo",
        )));
  });
  b("sbRestore", async () => {
    await iu(), Wn(), _("Sons de f\xE1brica restaurados");
  });
  b("sbEdit", (t) => {
    let e = kt(t.dataset.id);
    if (!e || e.builtin) return;
    let a = e.e;
    ee(`<h2 class="sh-title">Editar som</h2>
    <label class="lbl" for="sbname">Nome</label><input class="inp" id="sbname" maxlength="24" value="${M(e.n)}">
    <span class="lbl">Emoji</span><div class="egrid sm" id="sbemo">${ku.map((n) => `<button class="${n === a ? "on" : ""}" data-act="sbEmo" data-e="${n}">${n}</button>`).join("")}</div>
    <button class="btn" data-act="sbEditSave" data-id="${e.id}">Salvar</button>`),
      b("sbEmo", (n) => {
        (a = n.dataset.e),
          oe("#sbemo button").forEach((s) => s.classList.toggle("on", s === n));
      }),
      b("sbEditSave", async (n) => {
        await ou(n.dataset.id, { n: y("#sbname").value.trim() || e.n, e: a }),
          F(),
          Wn(),
          _("Som atualizado");
      });
  });
  b("sbAdd", () => {
    ee(`<h2 class="sh-title">Adicionar som</h2><p class="sh-sub">At\xE9 ${Et} segundos. Voc\xEA escolhe o trecho antes de salvar.</p>
    <div class="agrid" style="grid-template-columns:1fr 1fr">
      <button class="atile" data-act="sbRec"><span class="ab">${x("mic")}</span>Gravar agora</button>
      <button class="atile" data-act="sbFile"><span class="ab">${x("file")}</span>Arquivo de \xE1udio</button></div>
    <p class="note" style="padding:14px 2px 0">Funciona com MP3, M4A, OGG, WAV e outros formatos comuns.</p>`);
  });
  b("sbFile", async () => {
    F();
    let t = await Kn("audio/*");
    if (!t) return;
    if (t.size > 2e7) return _("Arquivo grande demais (at\xE9 20 MB).");
    let e;
    try {
      e = await Ve().decodeAudioData(await t.arrayBuffer());
    } catch {
      return _("N\xE3o consegui ler esse \xE1udio. Tente outro arquivo.");
    }
    if (!e || e.duration < mo) return _("Esse \xE1udio \xE9 curto demais.");
    $u(
      e,
      t.name
        .replace(/\.[a-z0-9]{1,5}$/i, "")
        .replace(/[_-]+/g, " ")
        .trim()
        .slice(0, 24) || "Meu som",
    );
  });
  b("sbRec", () => o0());
  function o0() {
    let t = {
        stream: null,
        rec: null,
        parts: [],
        t0: 0,
        timer: null,
        ac: null,
        an: null,
        stopping: !1,
      },
      e = () => {
        if ((clearInterval(t.timer), t.rec && t.rec.state !== "inactive")) {
          t.rec.onstop = null;
          try {
            t.rec.stop();
          } catch {}
        }
        t.stream && t.stream.getTracks().forEach((n) => n.stop()),
          t.ac && t.ac.close().catch(() => {}),
          (t.stream = null);
      };
    ee(
      `<h2 class="sh-title">Gravar som</h2><p class="sh-sub" id="sbrecs">Toque no bot\xE3o para come\xE7ar. Grava at\xE9 10 segundos.</p>
    <div class="recbig"><button class="recbtn" id="sbrecb" data-act="sbRecToggle" aria-label="Come\xE7ar a gravar">${x("mic")}</button>
    <div class="rectime" id="sbrect">0,0 s</div><div class="reclvl"><i id="sbrecl"></i></div></div>`,
      { onClose: e },
    );
    let a = () => {
      !t.rec ||
        t.stopping ||
        ((t.stopping = !0),
        clearInterval(t.timer),
        (t.rec.onstop = async () => {
          let n = new Blob(t.parts, { type: t.rec.mimeType || "audio/webm" });
          e();
          let s;
          try {
            s = await Ve().decodeAudioData(await n.arrayBuffer());
          } catch {
            return F(), _("N\xE3o foi poss\xEDvel usar a grava\xE7\xE3o.");
          }
          $u(s, "Meu som");
        }),
        t.rec.stop());
    };
    b("sbRecToggle", async () => {
      if (t.rec) return a();
      try {
        t.stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            echoCancellation: !1,
            noiseSuppression: !1,
            autoGainControl: !1,
          },
        });
      } catch (r) {
        return F(), _(ze(r));
      }
      let n =
        [
          "audio/webm;codecs=opus",
          "audio/ogg;codecs=opus",
          "audio/webm",
          "audio/mp4",
        ].find(
          (r) => window.MediaRecorder && MediaRecorder.isTypeSupported(r),
        ) || "";
      (t.rec = new MediaRecorder(
        t.stream,
        n ? { mimeType: n, audioBitsPerSecond: 128e3 } : {},
      )),
        (t.rec.ondataavailable = (r) => {
          r.data && r.data.size && t.parts.push(r.data);
        }),
        t.rec.start(200),
        (t.t0 = performance.now());
      try {
        let r = window.AudioContext || window.webkitAudioContext;
        (t.ac = new r()),
          (t.an = t.ac.createAnalyser()),
          (t.an.fftSize = 512),
          t.ac.createMediaStreamSource(t.stream).connect(t.an);
      } catch {}
      let s = new Uint8Array(512),
        o = y("#sbrecb");
      o.classList.add("on"),
        (o.innerHTML = x("stop")),
        o.setAttribute("aria-label", "Parar"),
        (y("#sbrecs").textContent = "Gravando\u2026 toque de novo para parar."),
        (t.timer = setInterval(() => {
          let r = (performance.now() - t.t0) / 1e3,
            i = y("#sbrect");
          if ((i && (i.textContent = Fn(r)), t.an)) {
            t.an.getByteTimeDomainData(s);
            let c = 0;
            for (let u = 0; u < s.length; u++)
              c = Math.max(c, Math.abs(s[u] - 128));
            let l = y("#sbrecl");
            l && (l.style.transform = `scaleX(${Math.min(1, c / 90)})`);
          }
          r >= 10 && a();
        }, 80));
    });
  }
  function r0(t) {
    let e = t.sampleRate,
      a = [...Array(t.numberOfChannels).keys()].map((c) => t.getChannelData(c)),
      n = 0;
    for (let c of a)
      for (let l = 0; l < c.length; l += 4) n = Math.max(n, Math.abs(c[l]));
    let s = Math.max(0.008, n * 0.06),
      o = 0,
      r = t.length - 1,
      i = (c) => a.some((l) => Math.abs(l[c]) > s);
    for (; o < t.length && !i(o); ) o += 16;
    for (; r > o && !i(r); ) r -= 16;
    return o >= r
      ? [0, t.duration]
      : [Math.max(0, o / e - 0.04), Math.min(t.duration, r / e + 0.12)];
  }
  function $u(t, e) {
    let a = t.duration,
      [n, s] = r0(t);
    s - n < mo && ((n = 0), (s = Math.min(a, Et))), s - n > Et && (s = n + Et);
    let o = 0,
      r = Math.min(a, wu);
    (n < o || s > o + r) &&
      (o = Math.max(0, Math.min(a - r, n - (r - (s - n)) / 2)));
    let c = "\u{1F50A}",
      l = null,
      u = null;
    ee(
      `<h2 class="sh-title">Cortar som</h2><p class="sh-sub">Arraste as bordas verdes para escolher o trecho (at\xE9 ${Et} s).</p>
    ${a > wu ? '<div class="trimov" id="trimov"><canvas id="trimoc"></canvas><i id="trimow"></i></div>' : ""}
    <div class="trim" id="trim"><canvas id="trimc"></canvas><i class="dim l" id="tdl"></i><i class="dim r" id="tdr"></i><i class="sel" id="tsel"><b class="h l"></b><b class="h r"></b></i><i class="ph" id="tph"></i></div>
    <div class="trimtx"><span id="ttx"></span><button class="pbtn" data-act="trimPlay" id="tplay" aria-label="Ouvir trecho">${x("play")}</button></div>
    <label class="lbl" for="sbname">Nome</label><input class="inp" id="sbname" maxlength="24" value="${M(e)}">
    <span class="lbl">Emoji</span><div class="egrid sm" id="sbemo">${ku.map((T) => `<button class="${T === c ? "on" : ""}" data-act="sbEmo" data-e="${T}">${T}</button>`).join("")}</div>
    <button class="btn" data-act="sbSave" id="sbsave">Salvar som</button>`,
      { onClose: () => K() },
    );
    let d = y("#trim"),
      f = y("#trimc"),
      p = (T, g, w) => {
        let m = t.sampleRate,
          $ = Math.floor(T * m),
          A = Math.floor(g * m),
          R = Math.max(1, Math.floor((A - $) / w)),
          E = [...Array(t.numberOfChannels).keys()].map((D) =>
            t.getChannelData(D),
          ),
          B = new Float32Array(w),
          S = Math.max(1, Math.floor(R / 64));
        for (let D = 0; D < w; D++) {
          let I = 0,
            j = $ + D * R;
          for (let U = j; U < j + R && U < A; U += S)
            for (let G of E) {
              let W = Math.abs(G[U]);
              W > I && (I = W);
            }
          B[D] = I;
        }
        return B;
      },
      v = (T, g, w) => {
        let m = window.devicePixelRatio || 1,
          $ = T.clientWidth,
          A = T.clientHeight;
        (T.width = Math.round($ * m)), (T.height = Math.round(A * m));
        let R = T.getContext("2d"),
          E = Math.max(10, Math.floor($ / 3)),
          B = p(g, w, E),
          S = 0;
        for (let I of B) S = Math.max(S, I);
        R.fillStyle = getComputedStyle(T).color;
        let D = T.width / E;
        for (let I = 0; I < E; I++) {
          let j = Math.max(2 * m, (B[I] / (S || 1)) * T.height * 0.92);
          R.fillRect(I * D + D * 0.18, (T.height - j) / 2, D * 0.64, j);
        }
      },
      k = () => {
        let T = (w) => ((w - o) / r) * 100 + "%";
        (y("#tsel").style.left = T(n)),
          (y("#tsel").style.width = ((s - n) / r) * 100 + "%"),
          (y("#tdl").style.width = T(n)),
          (y("#tdr").style.left = T(s)),
          (y("#ttx").textContent = `${Fn(n)} a ${Fn(s)} \xB7 ${Fn(s - n)}`);
        let g = y("#trimow");
        g &&
          ((g.style.left = (o / a) * 100 + "%"),
          (g.style.width = (r / a) * 100 + "%"));
      },
      L = () => {
        v(f, o, o + r);
        let T = y("#trimoc");
        T && v(T, 0, a), k();
      };
    requestAnimationFrame(L);
    let N = (T) => {
        let g = d.getBoundingClientRect();
        return o + Math.max(0, Math.min(1, (T - g.left) / g.width)) * r;
      },
      V = (T) => {
        if (u.k === "a") (n = Math.min(T, s - mo)), s - n > Et && (s = n + Et);
        else if (u.k === "b")
          (s = Math.max(T, n + mo)), s - n > Et && (n = s - Et);
        else {
          let g = u.len;
          (n = Math.max(o, Math.min(o + r - g, T - u.off))), (s = n + g);
        }
        (n = Math.max(o, n)), (s = Math.min(o + r, s)), K(), k();
      };
    d.addEventListener("pointerdown", (T) => {
      try {
        d.setPointerCapture(T.pointerId);
      } catch {}
      let g = N(T.clientX),
        w = (26 / d.getBoundingClientRect().width) * r;
      Math.abs(g - n) <= w && Math.abs(g - n) <= Math.abs(g - s)
        ? (u = { k: "a" })
        : Math.abs(g - s) <= w
          ? (u = { k: "b" })
          : g > n && g < s
            ? (u = { k: "m", off: g - n, len: s - n })
            : (u = { k: g < n ? "a" : "b" }),
        V(g);
    }),
      d.addEventListener("pointermove", (T) => {
        u && V(N(T.clientX));
      });
    let O = () => {
      u = null;
    };
    d.addEventListener("pointerup", O), d.addEventListener("pointercancel", O);
    let q = y("#trimov");
    if (q) {
      let T = !1,
        g = (w) => {
          let m = q.getBoundingClientRect(),
            $ = s - n;
          (o = Math.max(
            0,
            Math.min(a - r, ((w - m.left) / m.width) * a - r / 2),
          )),
            (n = Math.max(o, Math.min(o + r - $, n))),
            (s = n + $),
            (n < o || s > o + r) && ((n = o), (s = o + $)),
            K(),
            L();
        };
      q.addEventListener("pointerdown", (w) => {
        T = !0;
        try {
          q.setPointerCapture(w.pointerId);
        } catch {}
        g(w.clientX);
      }),
        q.addEventListener("pointermove", (w) => {
          T && g(w.clientX);
        }),
        q.addEventListener("pointerup", () => {
          T = !1;
        }),
        q.addEventListener("pointercancel", () => {
          T = !1;
        });
    }
    function K() {
      if (!l) return;
      try {
        (l.src.onended = null), l.src.stop();
      } catch {}
      cancelAnimationFrame(l.raf), (l = null);
      let T = y("#tph");
      T && (T.style.opacity = "0");
      let g = y("#tplay");
      g && (g.innerHTML = x("play"));
    }
    b("trimPlay", () => {
      if (l) return K();
      let T = Ve();
      if (!T) return;
      let g = T.createBufferSource();
      (g.buffer = t), g.connect(T.destination);
      let w = T.currentTime;
      g.start(0, n, s - n),
        (l = { src: g, raf: 0 }),
        (g.onended = () => K()),
        (y("#tplay").innerHTML = x("pause"));
      let m = y("#tph"),
        $ = () => {
          if (!l) return;
          let A = T.currentTime - w;
          (m.style.left = ((n + Math.min(A, s - n) - o) / r) * 100 + "%"),
            (m.style.opacity = "1"),
            (l.raf = requestAnimationFrame($));
        };
      $();
    }),
      b("sbEmo", (T) => {
        (c = T.dataset.e),
          oe("#sbemo button").forEach((g) => g.classList.toggle("on", g === T));
      }),
      b("sbSave", async (T) => {
        if (T.classList.contains("busy")) return;
        K();
        let g = y("#sbname").value.trim() || "Meu som";
        T.classList.add("busy"), (T.textContent = "Salvando\u2026");
        try {
          let w = await pu(t, n, s),
            { bytes: m, mime: $ } = await fu(w, 64e3);
          if (m.length > Vn) throw new Error("grande");
          let A = await an({ bytes: m, mime: $, n: g, e: c, d: s - n });
          F(), _(`"${A.n}" est\xE1 no seu soundboard`), Wn(), Kt(A.id);
        } catch (w) {
          console.warn("[soundboard]", w),
            _("N\xE3o foi poss\xEDvel salvar o som."),
            T.classList.remove("busy"),
            (T.textContent = "Salvar som");
        }
      });
  }
  var Au = ["chats", "groups", "calls", "settings"],
    c0 = {
      chats: { i: "edit", l: "Nova conversa" },
      groups: { i: "plus", l: "Criar grupo" },
      calls: { i: "phone", l: "Nova chamada" },
    };
  function Ea(t, e) {
    let a = h.tab,
      n = Au.indexOf(t) - Au.indexOf(a);
    (h.tab = t),
      oe(".panel").forEach((r) => {
        let i = r.id === "tab-" + t;
        r.classList.remove("in-r", "in-l"),
          r.classList.toggle("on", i),
          i &&
            !e &&
            n !== 0 &&
            (r.offsetWidth, r.classList.add(n > 0 ? "in-r" : "in-l"));
      }),
      oe(".navbtn").forEach((r) =>
        r.classList.toggle("on", r.dataset.tab === t),
      );
    let s = y("#fab"),
      o = c0[t];
    (s.hidden = !o),
      s.classList.remove("away"),
      o && ((s.innerHTML = x(o.i)), s.setAttribute("aria-label", o.l)),
      t === "calls" &&
        ((h.missedSeen = Date.now()),
        localStorage.setItem("jm.missedSeen", String(h.missedSeen)),
        Bi()),
      t === "settings" && P.renderSettings && P.renderSettings(),
      !e &&
        n !== 0 &&
        qt(
          y(
            {
              chats: "#chatList",
              groups: "#groupList",
              calls: "#callList",
              settings: "#setList",
            }[t],
          ),
        );
  }
  b("tab", (t) => {
    t.dataset.tab !== h.tab && Ea(t.dataset.tab);
  });
  b("fab", () => {
    h.tab === "chats"
      ? P.newChat()
      : h.tab === "groups"
        ? P.newGroup()
        : h.tab === "calls" && P.newCall();
  });
  b("filter", (t) => {
    (h.filter = t.dataset.f),
      oe("#chips .chip").forEach((e) => e.classList.toggle("on", e === t)),
      Ti(),
      qt(y("#chatList"));
  });
  function l0(t, e) {
    let a = h.engine,
      n = Pt(t.pk),
      s = ya(t.key),
      o = t.lastFrom === a.pk,
      r = t.lastStatus,
      i =
        o && !s && t.lastText && t.lastKind !== "sys"
          ? `<span class="tick ${r === "read" ? "read" : ""}">${x(r === "pending" ? "clock" : r === "delivered" || r === "read" ? "checks" : "check", "xs")}</span>`
          : "",
      c = s
        ? '<span class="typ">digitando\u2026</span>'
        : `${i}<span class="t">${M(t.lastText || (n.contact && n.contact.note) || "Toque para conversar")}</span>`,
      l = t.unread
        ? `<span class="rbadge ${t.muted ? "mute" : ""}">${t.unread}</span>`
        : t.pinned
          ? `<span class="rico">${x("pin", "xs")}</span>`
          : t.muted
            ? `<span class="rico">${x("belloff", "xs")}</span>`
            : t.ttl
              ? `<span class="rico">${x("timer", "xs")}</span>`
              : "";
    return `<div class="row" style="--i:${Math.min(e, 10)}" role="button" tabindex="0" data-act="openChat" data-key="${t.key}" data-long="rowMenu">
    ${pe(t.pk, 54)}
    <div class="rmain"><div class="rtop"><b class="rname">${M(n.name)}</b><span class="rtime ${t.unread && !t.muted ? "un" : ""}">${M(Xa(t.lastTs))}</span></div>
    <div class="rbot"><span class="rprev">${c}</span>${l}</div></div></div>`;
  }
  function d0(t, e) {
    let a = h.engine,
      n = a.groupChats(t.lid),
      s = n.slice().sort((u, d) => (d.lastTs || 0) - (u.lastTs || 0))[0],
      o = n.reduce((u, d) => u + (d.unread || 0), 0),
      r = s ? t.meta.channels.find((u) => u.id === s.ch) : null,
      i =
        s && s.lastFrom && s.lastText && s.lastKind !== "sys"
          ? (s.lastFrom === a.pk ? "Voc\xEA" : Ue(s.lastFrom)) + ": "
          : "",
      c = r ? `<span class="chtag"># ${M(r.name)}</span>` : "",
      l = t.meta.channels.filter(
        (u) => u.type === "voice" && a.roomMembers(t.lid, u.id).length,
      ).length;
    return `<div class="row" style="--i:${Math.min(e, 10)}" role="button" tabindex="0" data-act="openGroupChat" data-lid="${t.lid}" data-key="${s ? s.key : ""}">
    ${Pe(t, 54)}
    <div class="rmain"><div class="rtop"><b class="rname">${M(t.meta.name)}</b><span class="rtime ${o && !t.mute ? "un" : ""}">${M(Xa(s ? s.lastTs : t.joinedAt))}</span></div>
    <div class="rbot"><span class="rprev">${l ? `<span class="typ">${x("volume", "xs")} sala de voz ativa</span>` : `${c}<span class="t">${M(i + ((s && s.lastText) || "Grupo novo"))}</span>`}</span>${o ? `<span class="rbadge ${t.mute ? "mute" : ""}">${o}</span>` : ""}</div></div></div>`;
  }
  function Ti() {
    let t = h.engine;
    if (!t || !t.me) return;
    let e = Ja(h.q.trim()),
      a = [],
      n = [];
    for (let i of t.chatList()) {
      if (i.kind !== "dm") continue;
      let c = t.contacts.get(i.pk);
      if (!(c && c.status === "blocked")) {
        if (c && c.status === "request") {
          i.lastText && a.push(i);
          continue;
        }
        n.push({
          ts: i.lastTs || 0,
          pin: !!i.pinned,
          kind: "dm",
          c: i,
          unread: i.unread || 0,
          name: ne(i.pk),
          text: i.lastText || "",
        });
      }
    }
    for (let i of t.groups.values()) {
      if (i.left) continue;
      let c = t.groupChats(i.lid),
        l = Math.max(i.joinedAt || 0, ...c.map((u) => u.lastTs || 0));
      n.push({
        ts: l,
        pin: !!i.pinned,
        kind: "group",
        g: i,
        unread: t.groupUnread(i.lid),
        name: i.meta.name,
        text: c.map((u) => u.lastText || "").join(" "),
      });
    }
    let s = n.filter(
      (i) =>
        !(
          (h.filter === "unread" && !i.unread) ||
          (h.filter === "people" && i.kind !== "dm") ||
          (h.filter === "groups" && i.kind !== "group") ||
          (e && !Ja(i.name + " " + i.text).includes(e))
        ),
    );
    s.sort((i, c) => c.pin - i.pin || c.ts - i.ts);
    let r =
      (a.length && !e && h.filter !== "groups"
        ? `<button class="reqhead" data-act="openRequests">${x("mail")}<b>Pedidos de mensagem</b><span class="rbadge">${a.length}</span>${x("chev", "sm")}</button>`
        : "") +
      s.map((i, c) => (i.kind === "dm" ? l0(i.c, c) : d0(i.g, c))).join("");
    s.length
      ? (r += '<div style="height:96px"></div>')
      : (r +=
          e || h.filter !== "all"
            ? `<div class="empty">${e ? "Nada encontrado para essa busca." : "Nada por aqui neste filtro."}</div>`
            : `<div class="estate">${qe(72)}<h3>Nenhuma conversa ainda</h3><p>Adicione algu\xE9m pelo c\xF3digo ou mostre o seu QR para come\xE7ar a conversar.</p>
          <button class="btn" data-act="addContact">Adicionar contato</button><button class="btn ghost" data-act="myCode">Mostrar meu c\xF3digo</button></div>`),
      (y("#chatList").innerHTML = r);
  }
  b("openChat", (t) => P.openChat(t.dataset.key));
  b("openGroupChat", (t) => {
    let e = h.engine.groups.get(t.dataset.lid);
    e && P.openGroup(e.lid);
  });
  b("openRequests", () => P.openRequests());
  document.addEventListener("input", (t) => {
    t.target.id === "q" && ((h.q = t.target.value), Ti());
  });
  function u0() {
    let t = h.engine;
    if (!t || !t.me) return;
    let e = [...t.groups.values()]
        .filter((s) => !s.left)
        .sort((s, o) => {
          let r = Math.max(0, ...t.groupChats(s.lid).map((c) => c.lastTs || 0));
          return (
            Math.max(0, ...t.groupChats(o.lid).map((c) => c.lastTs || 0)) - r
          );
        }),
      a = h.invites.filter((s) => !t.groups.has(s.lid)),
      n = "";
    a.length &&
      (n +=
        '<div class="label">Convites</div>' +
        a
          .map(
            (
              s,
            ) => `<div class="mrow2">${Pe({ meta: s.meta }, 52)}<div><div class="nmx">${M(s.meta.name)}</div><small>Convite de ${M(Ue(s.from))} \xB7 ${s.meta.members.length} ${s.meta.members.length === 1 ? "membro" : "membros"}</small></div>
      <button class="joinbtn" data-act="joinInvite" data-id="${s.msgId}">Entrar</button></div>`,
          )
          .join("")),
      e.length
        ? (n +=
            '<div class="label">Seus grupos</div>' +
            e
              .map((s, o) => {
                let r = t.groupUnread(s.lid),
                  i = s.meta.channels
                    .filter((l) => l.type === "voice")
                    .map((l) => t.roomMembers(s.lid, l.id).length)
                    .reduce((l, u) => l + u, 0),
                  c = s.meta.members.length;
                return `<div class="row" style="--i:${Math.min(o + 1, 10)}" role="button" tabindex="0" data-act="openGroup" data-lid="${s.lid}">${Pe(s, 56)}
        <div class="rmain"><div class="rtop"><b class="rname">${M(s.meta.name)}</b></div>
        <div class="rbot"><span class="rprev"><span class="t">${c} ${c === 1 ? "membro" : "membros"}${i ? ` \xB7 ${i} na sala de voz` : ""}${s.meta.owner === t.pk ? " \xB7 voc\xEA criou" : ""}</span></span>${r ? `<span class="rbadge ${s.mute ? "mute" : ""}">${r}</span>` : ""}</div></div></div>`;
              })
              .join("") +
            '<div style="height:100px"></div>')
        : a.length ||
          (n += `<div class="estate">${qe(72)}<h3>Crie seu primeiro grupo</h3><p>Grupos t\xEAm canais de texto e salas de voz, como um servidor. S\xF3 quem voc\xEA convidar consegue ler.</p>
      <button class="btn" data-act="newGroup">Criar grupo</button></div>`),
      (y("#groupList").innerHTML = n);
  }
  b("openGroup", (t) => P.openGroup(t.dataset.lid));
  b("joinInvite", async (t) => {
    let e = await h.engine.acceptInvite(t.dataset.id);
    e &&
      (_("Voc\xEA entrou em " + e.meta.name),
      (h.invites = h.invites.filter((a) => a.msgId !== t.dataset.id)),
      go(),
      P.openGroup(e.lid));
  });
  function go() {
    h.engine.setKv("invites", h.invites.slice(-30));
  }
  async function Cu() {
    let t = h.engine;
    if (!t || !t.me) return;
    let e = (await t.db.all("calls")).sort((s, o) => o.ts - s.ts).slice(0, 80),
      a = [];
    for (let s of t.groups.values())
      for (let o of s.meta.channels)
        if (o.type === "voice") {
          let r = t.roomMembers(s.lid, o.id);
          r.length && a.push({ g: s, c: o, m: r });
        }
    let n = "";
    a.length &&
      (n +=
        '<div class="label">Salas de voz ativas</div>' +
        a
          .slice(0, 3)
          .map(
            ({ g: s, c: o, m: r }) => `
    <button class="roomcard" data-act="joinRoom" data-lid="${s.lid}" data-ch="${o.id}">
      <span class="avs" style="display:flex">${r
        .slice(0, 3)
        .map(
          (i, c) =>
            `<span style="margin-left:${c ? -10 : 0}px">${pe(i, 36, { badge: !1 })}</span>`,
        )
        .join("")}</span>
      <span class="rc"><b>${M(o.name)}</b><small>${M(s.meta.name)} \xB7 ${r.length} na sala</small></span><span class="go">Entrar</span></button>`,
          )
          .join("")),
      (n += `<div class="row" role="button" tabindex="0" data-act="fab"><span class="cico acc" style="width:54px;height:54px;border-radius:50%">${x("phone")}</span>
    <div class="rmain"><b class="rname">Nova chamada</b><span class="sub">Voz ou v\xEDdeo, cifrada de ponta a ponta</span></div></div>`),
      e.length
        ? (n +=
            '<div class="label">Recentes</div>' +
            e
              .map((s, o) => {
                let r = s.status === "missed" && s.dir === "in",
                  i =
                    s.status === "missed"
                      ? s.dir === "in"
                        ? "Perdida"
                        : "Sem resposta"
                      : s.status === "declined"
                        ? "Recusada"
                        : s.status === "busy"
                          ? "Ocupado"
                          : s.dur
                            ? xa(s.dur)
                            : "";
                return `<div class="row" style="--i:${Math.min(o, 10)}" role="button" tabindex="0" data-act="openChat" data-key="dm:${s.pk}">${pe(s.pk, 54)}
        <div class="rmain"><b class="rname" style="${r ? "color:var(--danger)" : ""}">${M(ne(s.pk))}</b>
          <span class="cdir ${r ? "miss" : ""}">${x(s.dir === "in" ? "indown" : "inup")}${s.video ? x("video", "xs") : ""}${M(Xa(s.ts))}${i ? " \xB7 " + M(i) : ""}</span></div>
        <button class="ibtn" data-act="callBack" data-pk="${s.pk}" data-video="${s.video ? 1 : ""}" aria-label="Ligar para ${M(ne(s.pk))}">${x(s.video ? "video" : "phone")}</button></div>`;
              })
              .join(""))
        : (n += '<div class="empty">Suas chamadas aparecem aqui.</div>'),
      (n += '<div style="height:100px"></div>'),
      (y("#callList").innerHTML = n);
  }
  b("callBack", (t) => P.startCall(t.dataset.pk, !!t.dataset.video));
  b("joinRoom", (t) => P.joinRoom(t.dataset.lid, t.dataset.ch));
  b("clearCalls", async () => {
    await h.engine.db.clearStore("calls"),
      Cu(),
      Bi(),
      _("Hist\xF3rico de chamadas apagado");
  });
  async function Bi() {
    let t = h.engine;
    if (!t || !t.me) return;
    let e = 0;
    for (let r of t.chats.values()) {
      if (r.kind !== "dm" || r.muted) continue;
      let i = t.contacts.get(r.pk);
      (i && i.status !== "ok") || (e += r.unread || 0);
    }
    let a = 0;
    for (let r of t.groups.values()) r.mute || (a += t.groupUnread(r.lid));
    let n = h.missedSeen || +(localStorage.getItem("jm.missedSeen") || 0),
      s = (await t.db.all("calls")).filter(
        (r) => r.status === "missed" && r.dir === "in" && r.ts > n,
      ).length,
      o = (r, i) => {
        let c = y(r);
        (c.hidden = !i), (c.textContent = i > 99 ? "99+" : i);
      };
    o("#nb-chats", e + a),
      o("#nb-groups", h.invites.filter((r) => !t.groups.has(r.lid)).length),
      o("#nb-calls", s),
      P.nativeBadge && P.nativeBadge(e + a);
  }
  var bo = 0;
  function Mi() {
    let t = h.engine;
    if (!t || !t.pool) return;
    let e = t.pool.openCount(),
      a = y("#netbar");
    if (e > 0) {
      (a.innerHTML = ""), (bo = 0);
      return;
    }
    bo || (bo = Date.now());
    let n = Date.now() - bo > 9e3 || !navigator.onLine;
    a.innerHTML = `<div class="netbar ${n ? "off" : ""}">${n ? x("alert", "sm") : '<span class="spin"></span>'}<span>${n ? "Sem conex\xE3o com os servidores. O que voc\xEA enviar sai assim que a internet voltar." : "Conectando aos servidores\u2026"}</span></div>`;
  }
  var Ae = Pd(() => {
    Ti(), u0(), Cu(), Bi();
  }, 60);
  b("mainMenu", () =>
    ee(`<div class="mlist">
  <button class="mitem" data-act="menuGo" data-go="newChat">${x("edit")}Nova conversa</button>
  <button class="mitem" data-act="menuGo" data-go="addContact">${x("userplus")}Adicionar contato</button>
  <button class="mitem" data-act="menuGo" data-go="newGroup">${x("users")}Novo grupo</button>
  <button class="mitem" data-act="markAllRead">${x("checks")}Marcar tudo como lido</button>
  <button class="mitem" data-act="menuGo" data-go="settings">${x("sliders")}Ajustes</button></div>`),
  );
  b("menuGo", (t) => {
    F();
    let e = t.dataset.go;
    e === "settings" ? Ea("settings") : setTimeout(() => P[e](), 30);
  });
  b("markAllRead", async () => {
    F();
    for (let t of h.engine.chats.values())
      t.unread && (await h.engine.markRead(t.key));
    Ae(), _("Tudo marcado como lido");
  });
  var Su = new WeakMap();
  document.addEventListener(
    "scroll",
    (t) => {
      let e = t.target;
      if (!e.classList || !e.classList.contains("scroll")) return;
      let a = e.parentElement;
      if (
        (a && a.classList.toggle("scrolled", e.scrollTop > 4),
        e.id === "chatList" || e.id === "groupList" || e.id === "callList")
      ) {
        let n = Su.get(e) || 0;
        Su.set(e, e.scrollTop),
          Math.abs(e.scrollTop - n) > 3 &&
            y("#fab").classList.toggle(
              "away",
              e.scrollTop > n && e.scrollTop > 60,
            );
      }
    },
    !0,
  );
  var C = null,
    Yn = new Map(),
    xo = 60,
    p0 = () => window.matchMedia && matchMedia("(pointer:fine)").matches;
  async function Zn(t, e = {}) {
    let a = h.engine;
    if (J.find("chat")) for (; J.find("chat"); ) J.pop(!0);
    let n = a.chatFor(t);
    if (n.kind === "group" && !a.groups.get(n.lid)) {
      _("Esse grupo n\xE3o existe mais neste aparelho.");
      return;
    }
    C = {
      key: t,
      chat: n,
      msgs: [],
      more: !0,
      reply: null,
      rec: null,
      pop: null,
      sending: new Map(),
    };
    let s = J.open(
      "chat",
      `<header class="chathead">${Tu()}</header>
    <div id="pinbar"></div>
    <div class="msgs" id="msgs"></div>
    <div id="cfoot">${Ri()}</div>
    <div class="popbar" id="pop" hidden></div>`,
      { onClose: h0 },
    );
    C.el = s;
    let o = y("#msgin", s);
    o && n.draft && ((o.value = n.draft), vo(o), wo());
    let r = await a.messages(t, 1 / 0, xo);
    !C ||
      C.key !== t ||
      ((C.msgs = r),
      (C.more = r.length >= xo),
      Zt({ end: !0, intro: !0 }),
      jmPinBar(),
      a.setActive(t, h.visible),
      X.cancelNotify(t),
      P.refreshLists(),
      y("#msgs", s).addEventListener("scroll", y0, { passive: !0 }),
      e.focus &&
        setTimeout(() => {
          let i = y("#msgin", s);
          i && i.focus();
        }, 450));
  }
  function h0() {
    let t = h.engine;
    if (!C) return;
    let e = C.el && y("#msgin", C.el),
      a = t.chats.get(C.key);
    a &&
      e &&
      (a.draft || "") !== e.value &&
      ((a.draft = e.value), t.saveChat(a)),
      Ou(),
      C.rec && (C.rec.cancel(), clearInterval(C.recT)),
      t.setActive(null, h.visible),
      (C = null),
      P.refreshLists();
  }
  var yo = () => C;
  function Lu() {
    let t = h.engine,
      e = C.chat;
    if (e.kind === "dm") {
      if (ya(C.key)) return { live: !0, html: "digitando\u2026" };
      let o = t.contacts.get(e.pk),
        lis = t.listeningOf && t.listeningOf(e.pk);
      if (lis)
        return {
          html: `<span class="lstat">${x("music", "xs")} Ouvindo ${M(lis.s)}${lis.a ? " \u2014 " + M(lis.a) : ""}</span>`,
        };
      let jmPr = t.presenceOf && t.presenceOf(e.pk),
        jmSt = t.statusOf && t.statusOf(e.pk);
      if (jmSt)
        return { html: `<span class="pres ${jmSt.dnd ? "dnd" : jmPr || "off"}"><i></i>${M(((jmSt.e || "") + " " + (jmSt.t || (jmSt.dnd ? "N\xE3o perturbe" : ""))).trim())}</span>` };
      if (jmPr === "dnd") return { html: '<span class="pres dnd"><i></i>n\xE3o perturbe</span>' };
      if (jmPr)
        return { live: jmPr === "on", html: `<span class="pres ${jmPr}"><i></i>${jmPr === "on" ? "online" : "ausente"}</span>` };
      return o && o.status === "request"
        ? { html: "Fora dos seus contatos" }
        : e.ttl
          ? { html: `${x("timer", "xs")} Tempor\xE1rias: ${M(yt(e.ttl))}` }
          : { html: `${x("lock", "xs")} Conversa cifrada` };
    }
    let a = t.groups.get(e.lid),
      n = a && a.meta.channels.find((o) => o.id === e.ch);
    if (n && n.topic) return { html: M(n.topic) };
    let s = a ? a.meta.members.length : 0;
    return {
      html: `${M(a ? a.meta.name : "")} \xB7 ${s} ${s === 1 ? "membro" : "membros"}`,
    };
  }
  function Tu() {
    let t = h.engine,
      e = C.chat,
      a = Lu();
    if (e.kind === "dm") {
      let o = Pt(e.pk);
      return `<button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button>
      <div class="who" data-act="openProfile" data-pk="${e.pk}" role="button" tabindex="0">${pe(e.pk, 40, { ring: "var(--s1)" })}
        <div><div class="nm">${M(o.name)}</div><div class="st ${a.live ? "live" : ""}" id="chatSub">${a.html}</div></div></div>
      <button class="ibtn" data-act="chatCall" data-video="1" aria-label="Chamada de v\xEDdeo">${x("video")}</button>
      <button class="ibtn" data-act="chatCall" aria-label="Chamada de voz">${x("phone")}</button>
      <button class="ibtn" data-act="chatMenu" aria-label="Mais op\xE7\xF5es">${x("more")}</button>`;
    }
    let n = t.groups.get(e.lid),
      s = n.meta.channels.find((o) => o.id === e.ch) || { name: e.ch };
    return `<button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button>
    <div class="who" data-act="chatToGroup" role="button" tabindex="0"><span class="hashbox">${x("hash")}</span>
      <div><div class="nm">${M(s.name)}</div><div class="st" id="chatSub">${a.html}</div></div></div>
    <button class="ibtn" data-act="chatToGroup" aria-label="Canais e membros">${x("users")}</button>
    <button class="ibtn" data-act="chatMenu" aria-label="Mais op\xE7\xF5es">${x("more")}</button>`;
  }
  function Ri() {
    let t = h.engine,
      e = C.chat;
    if (e.kind === "dm") {
      let a = t.contacts.get(e.pk);
      if (a && a.status === "request")
        return `<div class="reqbar"><p><b>${M(ne(e.pk))}</b> n\xE3o est\xE1 nos seus contatos. Ningu\xE9m sabe que voc\xEA viu esta conversa at\xE9 voc\xEA aceitar.</p>
      <div class="two"><button class="btn ghost" data-act="reqBlock">Bloquear</button><button class="btn" data-act="reqAccept">Aceitar</button></div></div>`;
      if (a && a.status === "blocked")
        return '<div class="reqbar"><p>Voc\xEA bloqueou este contato.</p><div class="two"><button class="btn" data-act="reqUnblock">Desbloquear</button></div></div>';
    }
    return `<div class="composer" id="composer"><div id="mentslot"></div><div id="replyslot"></div><div id="cbody">${Ii()}</div></div>`;
  }
  function Ii() {
    return C && C.rec
      ? `<div class="crow"><div class="recbar"><i class="rd"></i><span class="tm" id="rectm">0:00</span><span class="lvl" id="lvl"></span>
      <button class="ibtn" data-act="recCancel" aria-label="Cancelar grava\xE7\xE3o">${x("trash")}</button></div>
      <button class="sendbtn has" data-act="recSend" aria-label="Enviar mensagem de voz"><span class="ic ic-send">${x("send")}</span></button></div>`
      : `<div class="crow"><div class="field">
      <button class="ibtn sm" data-act="emoji" aria-label="Emojis">${x("smile")}</button>
      <textarea id="msgin" rows="1" placeholder="Mensagem" aria-label="Escrever mensagem" enterkeyhint="send"></textarea>
      <button class="ibtn sm" data-act="attach" aria-label="Anexar">${x("clip")}</button></div>
      <button class="sendbtn" id="sendbtn" data-act="send" aria-label="Gravar mensagem de voz"><span class="ic ic-mic">${x("mic")}</span><span class="ic ic-send">${x("send")}</span></button></div>`;
  }
  function sn() {
    if (!C) return;
    let t = y(".chathead", C.el);
    t && (t.innerHTML = Tu());
  }
  function m0(t) {
    if (t.from !== h.engine.pk || t.kind === "sys" || t.kind === "deleted")
      return "";
    let e = t.status || "sent";
    return e === "pending"
      ? `<span class="tick" title="Enviando">${x("clock", "xs")}</span>`
      : e === "failed"
        ? `<span class="tick fail" data-act="retry" data-id="${t.id}">${x("alert", "xs")}</span>`
        : C.chat.kind === "group"
          ? `<span class="tick">${x("check", "xs")}</span>`
          : `<span class="tick ${e === "read" ? "read" : ""}">${x(e === "delivered" || e === "read" ? "checks" : "check", "xs")}</span>`;
  }
  function b0(t) {
    let e = t.from === h.engine.pk ? "Voc\xEA" : Ue(t.from);
    switch (t.sys) {
      case "ttl":
        return t.v
          ? `${e} ativou mensagens tempor\xE1rias: ${yt(t.v)}`
          : `${e} desativou as mensagens tempor\xE1rias`;
      case "missed":
        return `${t.v ? "Chamada de v\xEDdeo" : "Chamada de voz"} perdida \xE0s ${so(t.ts)}`;
      case "created":
        return "Voc\xEA criou o grupo";
      case "joined":
        return "Voc\xEA entrou no grupo";
      case "join":
        return `${e} entrou no grupo`;
      case "leave":
        return `${e} saiu do grupo`;
      case "removed":
        return `${Ue(t.from)} foi removido do grupo`;
      case "kicked":
        return "Voc\xEA foi removido deste grupo";
      case "gpic":
        return `${e} mudou a foto do grupo`;
      case "gpicdel":
        return `${e} removeu a foto do grupo`;
      default:
        return "";
    }
  }
  var g0 = (t, e = 40) =>
    (t && t.length
      ? t
      : Array.from(
          { length: e },
          (a, n) => 6 + Math.round(Math.abs(Math.sin(n * 1.7)) * 16),
        )
    )
      .map(
        (a) => `<i style="height:${Math.max(3, Math.round(a * 0.85))}px"></i>`,
      )
      .join("");
  function x0(t) {
    let e = h.engine;
    if (t.kind === "deleted") return `${x("ban", "xs")} Mensagem apagada`;
    if (t.kind === "poll") return jmPollHtml(t);
    if (t.kind === "event") return jmEventHtml(t);
    if (t.kind === "sound") {
      let a = kt(t.snd) || { id: t.snd, e: "\u{1F50A}", n: "Som" };
      return `<div class="snd"><span class="semo">${M(a.e)}</span><div class="stx"><b>${M(a.n)}</b><small>${x("wave", "xs")} Soundboard</small></div><button class="pbtn" data-act="playSnd" data-id="${M(a.id)}" aria-label="Tocar ${M(a.n)}">${x("play")}</button></div>`;
    }
    if (t.kind === "invite") {
      let a = t.inv,
        n = t.from === e.pk,
        s = e.groups.has(a.lid),
        o = (a.meta.members || []).length;
      return `<div class="invite"><div class="ih">${Pe({ meta: a.meta, pic: a.pic }, 44)}<div><b>${M(a.meta.name)}</b><small>Convite para grupo \xB7 ${o} ${o === 1 ? "membro" : "membros"}</small></div></div>
      ${n ? "" : s ? `<button class="btn ghost" data-act="openGroupLid" data-lid="${a.lid}">Abrir grupo</button>` : `<button class="btn" data-act="acceptInvite" data-id="${t.id}">Entrar no grupo</button>`}</div>`;
    }
    if (t.kind === "media" && t.media.vo) return jmVoHtml(t);
    if (t.kind === "media" && t.media.type === "file") return jmFileHtml(t);
    if (t.kind === "media") {
      let a = t.media,
        n = t.from === e.pk,
        s = C.sending.get(t.id);
      if (a.type === "audio" && a.sb) {
        let u = !a.ready;
        return `<div class="snd"><span class="semo">${M(a.sb.e)}</span><div class="stx"><b>${M(a.sb.n)}</b><small>${x("wave", "xs")} Som personalizado</small></div>
        ${n || u ? "" : `<button class="ibtn sm" data-act="saveSnd" data-id="${t.id}" aria-label="Salvar no meu soundboard">${x("download", "sm")}</button>`}
        <button class="pbtn" data-act="playSbMedia" data-mid="${a.mid}" ${u ? "disabled" : ""} aria-label="Tocar ${M(a.sb.n)}">${u ? '<span class="spin"></span>' : x("play")}</button></div>`;
      }
      if (a.type === "audio") {
        let u = !a.ready;
        return `<div class="voice" data-mid="${a.mid}" data-dur="${a.dur || 0}"><button class="pbtn" data-act="playVoice" data-mid="${a.mid}" aria-label="Reproduzir mensagem de voz" ${u ? "disabled" : ""}>${u ? '<span class="spin"></span>' : x("play")}</button><div class="wv">${g0(a.wf)}</div><span class="vdur">${u ? `${a.got || 0}/${a.n}` : xa(a.dur || 0)}</span></div>`;
      }
      let o = a.w || 4,
        r = a.h || 3,
        c = `width:250px;aspect-ratio:${1 / Math.min(1.5, Math.max(0.6, r / o))}`;
      if (a.ready)
        return `<span class="photo" style="${c}" data-act="viewPhoto" data-mid="${a.mid}"><img data-mid="${a.mid}" alt="Foto">${/^image\/gif/i.test(a.mime || "") ? '<i class="gifbadge">GIF</i>' : ""}</span>`;
      let l = a.err
        ? "Foto corrompida"
        : n && !a.ready
          ? "M\xEDdia n\xE3o dispon\xEDvel neste aparelho"
          : `Recebendo ${a.got || 0}/${a.n}`;
      return `<span class="photo" style="${c}"><span class="ph" style="${a.th ? `background-image:url(${a.th})` : ""}"></span><span class="dl"><span>${a.err ? x("alert", "xs") : '<span class="spin"></span>'}${M(l)}</span></span></span>`;
    }
    return jmMentionHtml(Od(t.text || ""), C && C.chat) + (t.lp ? jmLpHtml(t.lp) : "");
  }
  function Bu(t, e, a, n = {}) {
    let s = h.engine;
    if (t.kind === "sys")
      return `<div class="sysmsg ${t.sys === "missed" ? "call" : ""}" data-mid="${t.id}">${t.sys === "missed" ? x("phone", "xs") : ""}${M(b0(t))}</div>`;
    let o = a[e - 1],
      r = a[e + 1],
      i = t.from === s.pk,
      c = (q, K) =>
        q &&
        K &&
        new Date(q.ts).toDateString() === new Date(K.ts).toDateString(),
      l =
        !o ||
        o.kind === "sys" ||
        o.from !== t.from ||
        !c(o, t) ||
        t.ts - o.ts > 6e5,
      u =
        !r ||
        r.kind === "sys" ||
        r.from !== t.from ||
        !c(r, t) ||
        r.ts - t.ts > 6e5,
      d = C.chat.kind === "group",
      f = "";
    if (d && !i && l) {
      let q = s.groups.get(C.chat.lid),
        K =
          q &&
          (q.meta.owner === t.from
            ? "Dono"
            : (q.meta.admins || []).includes(t.from)
              ? "Admin"
              : "");
      let rc = q && jmRolesOf(q, t.from)[0];
      f = `<div class="mname ${K ? "role" : ""}" ${rc ? `style="color:${JM_ROLE_CORES[rc.c]}"` : ""}>${M(ne(t.from))}${K ? `<span class="rolepill">${K}</span>` : ""}</div>`;
    }
    let p = t.fwd ? `<div class="fwdtag">${x("share", "xs")} Encaminhada</div>` : "";
    if (t.reply) {
      let q = a.find((K) => K.id === t.reply);
      p += `<div class="quote" data-act="jumpTo" data-id="${t.reply}"><b>${q ? M(q.from === s.pk ? "Voc\xEA" : ne(q.from)) : "Mensagem"}</b>${q ? M(s.preview(q)).slice(0, 140) : "Mensagem anterior"}</div>`;
    }
    let v =
        t.reacts && Object.keys(t.reacts).length
          ? (() => {
              let q = {};
              for (let [K, T] of Object.entries(t.reacts))
                (q[T] = q[T] || { n: 0, me: !1 }),
                  q[T].n++,
                  K === s.pk && (q[T].me = !0);
              return `<div class="reacts">${Object.entries(q)
                .map(
                  ([K, T]) =>
                    `<span class="rx ${T.me ? "me" : ""}">${K}${T.n > 1 ? `<small>${T.n}</small>` : ""}</span>`,
                )
                .join("")}</div>`;
            })()
          : "",
      k = t.kind === "text" && !t.reply && jd(t.text),
      L = [
        "msg",
        i ? "me" : "them",
        t.kind === "deleted" ? "del" : "",
        k ? "emoji" : "",
        !i && d && t.kind === "text" && jmMentionsMe(t.text, C.chat, t.from) ? "mentioned" : "",
        t.kind === "poll" || t.kind === "event" ? "pollm" : "",
        t.kind === "media" && t.media.type === "image" && !t.media.vo ? "img" : "",
      ].join(" "),
      N =
        d && !i
          ? u
            ? pe(t.from, 28, { badge: !1 })
            : '<span class="mslot"></span>'
          : "",
      V = `<span class="meta">${t.edited && t.kind === "text" ? '<i class="edtag">editada</i>' : ""}${t.exp ? x("timer", "timer") : ""}${so(t.ts)}${m0(t)}</span>`;
    return `<div class="${["mrow", i ? "me" : "them", l ? "first" : "", u ? "last" : "", n.fresh ? "newm" : ""].join(" ")}" data-mid="${t.id}">${N}<div class="${L}" data-act="msgTap" data-id="${t.id}">${f}${p}${x0(t)}${V}${v}</div></div>`;
  }
  function Zt(t = {}) {
    if (!C) return;
    let e = y("#msgs", C.el);
    if (!e) return;
    let a = e.scrollHeight - e.scrollTop - e.clientHeight < 120,
      n = e.scrollHeight - e.scrollTop,
      s = C.msgs,
      o = C.more
        ? '<div class="more"><span class="spin"></span></div>'
        : `<div class="e2e">${x("lock", "sm")}<span>Mensagens e chamadas s\xE3o cifradas de ponta a ponta. Ningu\xE9m fora desta conversa, nem os servidores, consegue ler ou ouvir.</span></div>`,
      r = "";
    s.forEach((i, c) => {
      let l = new Date(i.ts).toDateString();
      l !== r && ((o += `<div class="divider">${M(_d(i.ts))}</div>`), (r = l)),
        (o += Bu(i, c, s, { fresh: t.fresh === i.id }));
    }),
      C.chat.kind === "dm" &&
        ya(C.key) &&
        (o +=
          '<div class="mrow them first last newm"><div class="msg them"><span class="typing"><i></i><i></i><i></i></span></div></div>'),
      (e.innerHTML = o),
      t.intro &&
        oe(".mrow", e)
          .slice(-8)
          .forEach((i, c, l) => {
            (i.style.animation = `rise .5s var(--ease) both ${120 + (l.length - c) * 0}ms`),
              (i.style.animationDelay = 120 + c * 35 + "ms");
          }),
      t.end || (a && !t.keep)
        ? t.smooth
          ? e.scrollTo({ top: e.scrollHeight, behavior: "smooth" })
          : (e.scrollTop = e.scrollHeight)
        : (e.scrollTop = e.scrollHeight - n),
      Ru(e);
  }
  function Mu(t) {
    if (!C) return;
    let e = C.msgs.findIndex((r) => r.id === t.id);
    if (e < 0) return;
    C.msgs[e] = t;
    let a = y(`[data-mid="${t.id}"]`, y("#msgs", C.el));
    if (!a) return Zt({ keep: !0 });
    let n = document.createElement("div");
    n.innerHTML = Bu(t, e, C.msgs);
    let s = n.firstElementChild;
    a.replaceWith(s);
    let o = y(".tick", s);
    o && t.status === "read" && o.classList.add("tpop"), Ru(s);
  }
  async function Ru(t) {
    for (let e of oe("img[data-mid]", t)) {
      let a = e.dataset.mid,
        n = Yn.get(a);
      if (!n) {
        let s = await h.engine.mediaBlob(a);
        if (!s) continue;
        (n = URL.createObjectURL(s)), Yn.set(a, n);
      }
      e.src = n;
    }
    $a();
  }
  async function y0(t) {
    let e = t.target;
    if (($t(), !C || !C.more || C.loading || e.scrollTop > 120)) return;
    C.loading = !0;
    let a = C.msgs[0],
      n = await h.engine.messages(C.key, a ? a.ts : 1 / 0, xo);
    if (!C) return;
    let s = new Set(C.msgs.map((r) => r.id)),
      o = n.filter((r) => !s.has(r.id));
    (C.more = n.length >= xo),
      (C.msgs = [...o, ...C.msgs]),
      Zt({ keep: !0 }),
      (C.loading = !1);
  }
  function Iu(t, e) {
    if (!C || C.key !== t) return;
    if (C.msgs.some((n) => n.id === e.id)) return Mu(e);
    C.msgs.push(e), C.msgs.sort((n, s) => n.ts - s.ts);
    let a = e.from === h.engine.pk;
    Zt({ fresh: e.id, smooth: !0, end: a });
  }
  function Hu(t, e) {
    C && C.key === t && Mu(e);
  }
  function Uu(t, e) {
    !C ||
      C.key !== t ||
      ((C.msgs = C.msgs.filter((a) => a.id !== e)), Zt({ keep: !0 }));
  }
  function Hi(t) {
    if (!C || C.key !== t) return;
    let e = y("#chatSub", C.el);
    if (e) {
      let s = Lu();
      (e.innerHTML = s.html), e.classList.toggle("live", !!s.live);
    }
    let a = y("#msgs", C.el),
      n = y(".typing", a);
    ya(t) && !n && Zt({ keep: !1 }), !ya(t) && n && Zt({ keep: !0 });
  }
  function Du(t, e, a) {
    if (!C) return;
    C.sending.set(t, e < a ? `${e}/${a}` : null);
    let n = y(`[data-mid="${t}"] .vdur`, C.el);
    n && e < a && (n.textContent = `${e}/${a}`);
  }
  function vo(t) {
    (t.style.height = "auto"),
      (t.style.height = Math.min(t.scrollHeight, 120) + "px");
  }
  function wo() {
    if (!C) return;
    let t = y("#msgin", C.el),
      e = y("#sendbtn", C.el);
    if (!t || !e) return;
    let a = !!t.value.trim();
    e.classList.toggle("has", a),
      e.setAttribute(
        "aria-label",
        a ? "Enviar mensagem" : "Gravar mensagem de voz",
      );
  }
  document.addEventListener("input", (t) => {
    t.target.id !== "msgin" ||
      !C ||
      (vo(t.target), wo(), t.target.value.trim() && h.engine.sendTyping(C.key));
  });
  document.addEventListener("keydown", (t) => {
    t.target.id === "msgin" &&
      t.key === "Enter" &&
      !t.shiftKey &&
      p0() &&
      (t.preventDefault(), Nu());
  });
  async function Nu() {
    if (!C) return;
    let t = y("#msgin", C.el),
      e = t ? t.value.trim() : "";
    if (!e && C.edit) return jmEditCancel();
    if (!e) return _u();
    if (C.edit) {
      let r = C.edit.id;
      return (
        (t.value = ""), vo(t), wo(), jmEditCancel(),
        h.engine.editMsg(C.key, r, e).catch(() => _("N\xE3o foi poss\xEDvel editar."))
      );
    }
    let a = C.reply,
      jmFoco = document.activeElement === t;
    (t.value = ""),
      vo(t),
      jmFoco && t.focus({ preventScroll: !0 }),
      wo(),
      (C.reply = null),
      (y("#replyslot", C.el).innerHTML = "");
    let n = h.engine.chats.get(C.key);
    n && (n.draft = "");
    try {
      let jmU = h.engine.settings.linkPreviews !== !1 && X.available && (e.match(/https?:\/\/[^\s<]+/) || [])[0],
        jmLp = jmU ? await jmLinkPreview(jmU.replace(/[.,)\]!?;:'"]+$/, "")) : null;
      await h.engine.sendText(C.key, e, { reply: a && a.id, lp: jmLp });
    } catch (s) {
      _(s.message || "N\xE3o foi poss\xEDvel enviar.");
    }
  }
  b("send", () => Nu());
  // Vídeo do outro lado: toque alterna entre preencher a tela e ver inteiro
  // (para a tela transmitida do computador não ficar cortada).
  b("rvidFit", () => {
    let t = y("#rvid");
    if (!t) return;
    (h.rvidFit = !t.classList.contains("fit")), t.classList.toggle("fit", h.rvidFit), _(h.rvidFit ? "Mostrando o v\xEDdeo inteiro" : "V\xEDdeo preenchendo a tela", 1200);
  });
  /*
   * Teclado ao enviar: tocar no botão tirava o foco do campo, o teclado
   * fechava e a tela pulava. Com o campo em foco, o toque no botão não
   * rouba o foco, e depois de enviar o campo continua pronto para a próxima.
   */
  document.addEventListener(
    "pointerdown",
    (t) => {
      let e = t.target.closest && t.target.closest("#sendbtn"),
        a = document.activeElement;
      e && a && a.id === "msgin" && a.value.trim() && t.preventDefault();
    },
    !0,
  );
  async function _u() {
    if (!C || C.rec) return;
    let t = new fo();
    try {
      await t.start((e) => {
        let a = C && y("#lvl", C.el);
        if (!a) return;
        let n = document.createElement("i");
        for (
          n.style.height = Math.max(3, Math.round(e * 24)) + "px",
            a.appendChild(n);
          a.children.length > 60;

        )
          a.firstChild.remove();
      });
    } catch (e) {
      _(ze(e));
      return;
    }
    if (!C) {
      t.cancel();
      return;
    }
    (C.rec = t),
      (y("#cbody", C.el).innerHTML = Ii()),
      X.vibrate(20),
      (C.recT = setInterval(() => {
        if (!C || !C.rec) return;
        let e = C.rec.elapsed(),
          a = y("#rectm", C.el);
        a && (a.textContent = xa(e)), e >= 180 && Ui(!0);
      }, 250));
  }
  async function Ui(t) {
    if (!C || !C.rec) return;
    let e = C.rec,
      a = C.key;
    if (
      ((C.rec = null),
      clearInterval(C.recT),
      (y("#cbody", C.el).innerHTML = Ii()),
      !t)
    ) {
      e.cancel();
      return;
    }
    let n = await e.stop();
    if (!n || n.dur < 0.8) {
      _("\xC1udio muito curto");
      return;
    }
    let s = new Uint8Array(await n.blob.arrayBuffer());
    try {
      await h.engine.sendMedia(a, s, {
        type: "audio",
        mime: n.mime,
        dur: n.dur,
        wf: n.wf,
      });
    } catch (o) {
      _(o.message || "N\xE3o foi poss\xEDvel enviar o \xE1udio.");
    }
  }
  b("recCancel", () => Ui(!1));
  b("recSend", () => Ui(!0));
  var xe = null;
  function Ou() {
    xe && (xe.a.pause(), (xe = null), $a());
  }
  async function v0(t) {
    if (xe && xe.mid === t) {
      xe.a.paused ? xe.a.play() : xe.a.pause(), $a();
      return;
    }
    Ou();
    let e = Yn.get(t);
    if (!e) {
      let n = await h.engine.mediaBlob(t);
      if (!n) return _("\xC1udio indispon\xEDvel");
      (e = URL.createObjectURL(n)), Yn.set(t, e);
    }
    let a = new Audio(e);
    (xe = { mid: t, a }),
      (a.ontimeupdate = $a),
      (a.onended = () => {
        (xe = null), $a();
      }),
      (a.onerror = () => {
        _("Este aparelho n\xE3o conseguiu tocar o \xE1udio."),
          (xe = null),
          $a();
      }),
      a.play().catch(() => {}),
      $a();
  }
  function $a() {
    C &&
      oe(".voice", C.el).forEach((t) => {
        let e = xe && xe.mid === t.dataset.mid,
          a = +t.dataset.dur || 0,
          n = e
            ? xe.a.currentTime /
              (xe.a.duration && isFinite(xe.a.duration)
                ? xe.a.duration
                : a || 1)
            : 0,
          s = oe(".wv i", t);
        s.forEach((i, c) =>
          i.classList.toggle("on", e && c < Math.round(n * s.length)),
        );
        let o = y(".pbtn", t);
        o &&
          !o.disabled &&
          (o.innerHTML = x(e && !xe.a.paused ? "pause" : "play"));
        let r = y(".vdur", t);
        r && !o.disabled && (r.textContent = e ? xa(xe.a.currentTime) : xa(a));
      });
  }
  b("playVoice", (t) => v0(t.dataset.mid));
  b("playSnd", (t) => Kt(t.dataset.id));
  b("playSbMedia", async (t) => {
    let e = await h.engine.mediaBlob(t.dataset.mid);
    e && nu("m:" + t.dataset.mid, await e.arrayBuffer());
  });
  b("saveSnd", async (t) => {
    let e = C && C.msgs.find((n) => n.id === t.dataset.id);
    if (!e || !e.media || !e.media.sb) return;
    let a = await h.engine.mediaBlob(e.media.mid);
    if (!a) return _("O som ainda n\xE3o chegou inteiro.");
    await an({
      bytes: new Uint8Array(await a.arrayBuffer()),
      mime: e.media.mime,
      n: e.media.sb.n,
      e: e.media.sb.e,
      d: e.media.dur || 2,
    }),
      _(`"${e.media.sb.n}" est\xE1 no seu soundboard`);
  });
  function w0(t, e) {
    let a = h.engine,
      n = C.msgs.find((v) => v.id === t);
    if (!n || n.kind === "sys") return;
    let s = y("#pop", C.el),
      o = C.el;
    C.pop = t;
    let r = n.kind !== "deleted",
      i = n.kind === "text";
    (s.innerHTML =
      (r
        ? [
            "\u{1F44D}",
            "\u2764\uFE0F",
            "\u{1F602}",
            "\u{1F62E}",
            "\u{1F622}",
            "\u{1F525}",
          ]
            .map(
              (v, k) =>
                `<button style="--i:${k}" data-act="react" data-e="${v}" aria-label="Reagir com ${v}">${v}</button>`,
            )
            .join("") + '<i class="sep"></i>'
        : "") +
      (r
        ? `<button style="--i:6" data-act="replyTo" aria-label="Responder">${x("reply", "sm")}</button>`
        : "") +
      (i
        ? `<button style="--i:7" data-act="copyMsg" aria-label="Copiar">${x("copy", "sm")}</button>`
        : "") +
      `<button style="--i:8" data-act="msgMore" aria-label="Mais op\xE7\xF5es">${x("more", "sm")}</button>`),
      (s.hidden = !1);
    let c = e.getBoundingClientRect(),
      l = o.getBoundingClientRect(),
      u = s.offsetWidth,
      d = c.top - l.top - s.offsetHeight - 8,
      f = !1;
    d < 64 && ((d = c.bottom - l.top + 8), (f = !0));
    let p = n.from === a.pk ? c.right - l.left - u : c.left - l.left;
    (p = Math.max(8, Math.min(p, l.width - u - 8))),
      (s.style.top = d + "px"),
      (s.style.left = p + "px"),
      (s.style.transformOrigin = `${n.from === a.pk ? "right" : "left"} ${f ? "top" : "bottom"}`),
      (s.style.animation = "none"),
      s.offsetWidth,
      (s.style.animation = ""),
      X.vibrate(8);
  }
  function $t() {
    if (!C) return !1;
    let t = y("#pop", C.el),
      e = t && !t.hidden;
    return t && (t.hidden = !0), (C.pop = null), e;
  }
  document.addEventListener(
    "click",
    (t) => {
      C &&
        C.pop &&
        !t.target.closest("#pop") &&
        !t.target.closest(".msg") &&
        $t();
    },
    !0,
  );
  // ===== Recursos menores: editar, fixar, buscar, enquetes, menções, online e cargos =====
  var JM_ROLE_CORES = ["#ef5350", "#ff9800", "#fdd835", "#4caf50", "#26a69a", "#42a5f5", "#ab47bc", "#ec407a"];
  function jmRolesOf(t, e) {
    let a = ((t.meta.mroles || {})[e] || []);
    return (t.meta.roles || []).filter((n) => a.includes(n.id));
  }
  function jmMyNames(t) {
    let e = h.engine,
      a = [(e.me && e.me.name) || ""];
    if (t && t.kind === "group") {
      let n = e.groups.get(t.lid);
      n && n.meta.names && n.meta.names[e.pk] && a.push(n.meta.names[e.pk]);
    }
    return a.map((n) => n.trim().toLowerCase()).filter(Boolean);
  }
  function jmMentionsMe(t, e, jmA) {
    if (!t || !e || e.kind !== "group") return !1;
    let a = String(t).toLowerCase(),
      jmG = h.engine.groups.get(e.lid);
    return (/(^|\s)@(todos|everyone)\b/.test(a) && !!jmG && !!jmA && jmCan(jmG.meta, jmA, "todos")) || jmMyNames(e).some((n) => a.includes("@" + n));
  }
  var jmRx = (t) => t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  function jmMentionHtml(t, e) {
    if (!e || e.kind !== "group" || t.indexOf("@") < 0) return t;
    let a = h.engine.groups.get(e.lid);
    if (!a) return t;
    let n = jmMyNames(e),
      s = [...new Set([...a.meta.members.map((o) => ne(o)), "todos"])].filter(Boolean).sort((o, r) => r.length - o.length);
    for (let o of s) {
      let r = new RegExp("(^|[\\s>(])@(" + jmRx(M(o)) + ")(?![\\p{L}\\p{N}])", "giu"),
        i = o === "todos" || n.includes(o.toLowerCase());
      t = t.replace(r, (c, l, u) => `${l}<span class="mention ${i ? "me" : ""}">@${u}</span>`);
    }
    return t;
  }
  // Sugestões ao digitar @ num grupo
  function jmMentionSuggest(t) {
    let e = y("#mentslot", C && C.el);
    if (!e) return;
    if (!C || C.chat.kind !== "group") return void (e.innerHTML = "");
    let a = t.value.slice(0, t.selectionStart || t.value.length).match(/(^|\s)@([^\s@]{0,20})$/);
    if (!a) return void (e.innerHTML = "");
    let n = a[2].toLowerCase(),
      s = h.engine.groups.get(C.chat.lid);
    if (!s) return;
    let o = s.meta.members
      .filter((r) => r !== h.engine.pk)
      .map((r) => ({ pk: r, n: ne(r) }))
      .filter((r) => r.n.toLowerCase().includes(n))
      .slice(0, 5);
    "todos".startsWith(n) && o.push({ pk: "", n: "todos" }),
      (e.innerHTML = o.length
        ? `<div class="mentsug">${o.map((r) => `<button data-act="mentPick" data-n="${M(r.n)}">${r.pk ? pe(r.pk, 22, { badge: !1 }) : x("users", "xs")}<span>${M(r.n)}</span></button>`).join("")}</div>`
        : "");
  }
  document.addEventListener("input", (t) => {
    t.target.id === "msgin" && jmMentionSuggest(t.target);
  });
  b("mentPick", (t) => {
    let e = y("#msgin", C.el);
    if (!e) return;
    let a = e.selectionStart || e.value.length,
      n = e.value.slice(0, a).replace(/@([^\s@]{0,20})$/, "@" + t.dataset.n + " ");
    (e.value = n + e.value.slice(a)), e.setSelectionRange(n.length, n.length), e.focus({ preventScroll: !0 }), vo(e), wo(), jmMentionSuggest(e);
  });
  // ---- Status personalizado e aparelhos conectados ----
  var jmStDur = 0;
  b("statusOpen", () => {
    let t = jmCleanStatus(h.engine.me.st) || {};
    (jmStDur = 0),
      ee(`<h2 class="sh-title">Definir status</h2><p class="sh-sub">Aparece para os seus contatos, junto com a m\xFAsica do Spotify.</p>
    <div style="display:flex;gap:8px"><input class="inp" id="ste" maxlength="8" placeholder="\u{1F3AE}" value="${M(t.e || "")}" style="width:64px;text-align:center"><input class="inp" id="stt" maxlength="80" placeholder="Ex.: Jogando Valorant" value="${M(t.t || "")}" autocomplete="off"></div>
    <div class="stsug">${[["\u{1F3AE}", "Jogando"], ["\u{1F4DA}", "Estudando"], ["\u{1F4BC}", "Trabalhando"], ["\u{1F319}", "Dormindo"]].map(([e, a]) => `<button data-act="stSug" data-e="${e}" data-t="${a}">${e} ${a}</button>`).join("")}</div>
    <label class="pick" style="padding:10px 2px"><span style="flex:1">N\xE3o perturbe<small style="display:block;opacity:.65">Sem som nem aviso de mensagem</small></span><input type="checkbox" id="stdnd" ${t.dnd ? "checked" : ""}></label>
    <span class="lbl">Limpar depois de</span><div class="stdur">${["N\xE3o limpar", "1 hora", "4 horas", "Hoje"].map((e, a) => `<button class="${a === 0 ? "on" : ""}" data-act="stDur" data-d="${a}">${e}</button>`).join("")}</div>
    <div class="two" style="margin-top:12px"><button class="btn ghost" data-act="stClear" style="margin:0">Tirar status</button><button class="btn" data-act="stSave" style="margin:0">Salvar</button></div>`);
  });
  b("stSug", (t) => {
    (y("#ste").value = t.dataset.e), (y("#stt").value = t.dataset.t);
  });
  b("stDur", (t) => {
    (jmStDur = +t.dataset.d), oe(".stdur button").forEach((e) => e.classList.toggle("on", e === t));
  });
  b("stSave", async () => {
    let t = Date.now(),
      e = new Date();
    e.setHours(23, 59, 59, 0);
    let a = [0, t + 36e5, t + 144e5, e.getTime()][jmStDur] || 0,
      n = { e: y("#ste").value.trim(), t: y("#stt").value.trim(), dnd: y("#stdnd").checked, x: a };
    F(), await h.engine.setMyStatus(n), _("Status atualizado"), P.refreshSettings && P.refreshSettings();
  });
  b("stClear", async () => {
    F(), await h.engine.setMyStatus(null), _("Status removido");
  });
  b("devicesOpen", async () => {
    let t = await h.engine.devices(),
      e = Date.now(),
      a = (n) => (e - n < 252e5 ? "Ativo recentemente" : e - n < 864e5 ? "Visto hoje" : `Visto h\xE1 ${Math.floor((e - n) / 864e5)} dias`);
    J.open(
      "devices",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Aparelhos conectados</h2></div>
    <div class="scroll"><div class="label">Este aparelho</div>
      ${le({ i: "phone", n: "Este celular", s: "Android", act: "noop" })}
      <div class="label">Outros aparelhos</div>
      ${t.length ? t.map((n) => le({ i: n.p === "Android" || n.p === "iOS" ? "phone" : "server", n: M(n.n), s: `${M(n.p)} \xB7 ${a(n.last || 0)}`, act: "devOff", data: `data-id="${M(n.id)}" data-n="${M(n.n)}"` })).join("") : '<p class="note">Nenhum outro aparelho visto ainda. Eles aparecem aqui quando abrem o app.</p>'}
      <p class="note">Toque num aparelho para desconectar: ele sai e apaga a conta assim que ficar on-line. Se algu\xE9m roubou suas 12 palavras, isso n\xE3o basta \u2014 crie uma conta nova e avise seus contatos.</p></div></div>`,
    );
  });
  b("devOff", async (t) => {
    (await ge({ title: "Desconectar " + t.dataset.n + "?", text: "O aparelho sai e apaga a conta dele assim que ficar on-line.", ok: "Desconectar", danger: !0 })) &&
      (await h.engine.removeDevice(t.dataset.id), J.pop(), _("Pedido enviado"));
  });
  // ---- Volume de cada pessoa (só o que você ouve; fica guardado) ----
  var jmVols = {};
  function jmVolOf(t) {
    let e = jmVols[t];
    return typeof e == "number" ? Math.max(0, Math.min(1, e)) : 1;
  }
  async function jmVolLoad() {
    try {
      jmVols = (await h.engine.kv("vols", {})) || {};
    } catch {}
  }
  function jmVolApply(t) {
    let e = jmVolOf(t);
    for (let a of oe(`audio[data-aud="${t}"], video[data-vid="${t}"]`)) a.volume = e;
    let a = y("#rvid");
    a && H && H.pk === t && (a.volume = e);
  }
  b("peerVol", (t) => {
    let e = t.dataset.pk,
      a = Math.round(jmVolOf(e) * 100);
    ee(`<h2 class="sh-title">Volume de ${M(ne(e))}</h2><p class="sh-sub">S\xF3 muda o que voc\xEA ouve. Fica guardado para as pr\xF3ximas chamadas.</p>
    <div class="volrow">${x("volume")}<input type="range" id="pvol" min="0" max="100" step="5" value="${a}" data-pk="${e}"><b id="pvolv">${a}%</b></div>
    <button class="btn ghost" data-act="peerVolReset" data-pk="${e}">Voltar ao normal</button>`);
  });
  document.addEventListener("input", (t) => {
    if (t.target.id !== "pvol") return;
    let e = t.target.dataset.pk,
      a = +t.target.value;
    (jmVols[e] = a / 100), jmVolApply(e);
    let n = y("#pvolv");
    n && (n.textContent = a + "%"), clearTimeout(jmVolLoad.t), (jmVolLoad.t = setTimeout(() => h.engine.setKv("vols", jmVols), 400));
  });
  b("peerVolReset", (t) => {
    delete jmVols[t.dataset.pk], jmVolApply(t.dataset.pk), h.engine.setKv("vols", jmVols), F();
  });
  // ---- Eventos ----
  function jmEventHtml(t) {
    let e = t.evt || {},
      a = new Date(e.at),
      n = ["dom", "seg", "ter", "qua", "qui", "sex", "s\xE1b"][a.getDay()] + ", " + a.toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" }) + " \xE0s " + a.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" }),
      s = e.at - Date.now(),
      o = s < -108e5 ? "J\xE1 aconteceu" : s < 0 ? "Acontecendo agora" : s < 36e5 ? `Em ${Math.floor(s / 6e4) + 1} min` : s < 864e5 ? `Em ${Math.floor(s / 36e5)} h` : s < 1728e5 ? "Amanh\xE3" : `Em ${Math.floor(s / 864e5)} dias`,
      r = t.rsvp || {},
      i = (u) => Object.values(r).filter((d) => d === u).length,
      c = r[h.engine.pk] || "",
      l = (u, d) => `<button class="rsvpb ${c === d ? "on" : ""}" data-act="rsvp" data-id="${t.id}" data-r="${c === d ? "" : d}">${u}</button>`;
    return `<div class="evcard"><div class="evh"><span class="evi">${x("clock", "sm")}</span><div><b>${M(e.ti)}</b><small>${M(n)} \xB7 ${o}</small></div></div>${e.d ? `<p>${M(e.d)}</p>` : ""}<small class="evc">${i("y")} v\xE3o \xB7 ${i("m")} talvez \xB7 ${i("n")} n\xE3o v\xE3o</small><div class="evb">${l("Vou", "y")}${l("Talvez", "m")}${l("N\xE3o vou", "n")}</div></div>`;
  }
  b("rsvp", async (t) => {
    X.vibrate(10), await h.engine.rsvp(C.key, t.dataset.id, t.dataset.r);
  });
  b("evtNew", () => {
    F();
    let t = new Date(Date.now() + 864e5);
    t.setHours(20, 0, 0, 0);
    let e = new Date(t.getTime() - t.getTimezoneOffset() * 6e4).toISOString().slice(0, 16);
    setTimeout(
      () =>
        ee(`<h2 class="sh-title">Novo evento</h2><p class="sh-sub">Todos podem responder Vou, Talvez ou N\xE3o vou. Quem vai recebe um lembrete 15 minutos antes.</p>
    <label class="lbl" for="evt">Nome</label><input class="inp" id="evt" maxlength="120" placeholder="Ex.: Jogatina de s\xE1bado" autocomplete="off">
    <label class="lbl" for="evq">Quando</label><input class="inp" id="evq" type="datetime-local" value="${e}">
    <label class="lbl" for="evd">Descri\xE7\xE3o (opcional)</label><input class="inp" id="evd" maxlength="500" placeholder="Onde, o que levar\u2026" autocomplete="off">
    <button class="btn" data-act="evtSend">Criar evento</button>`),
      200,
    );
  });
  b("evtSend", async () => {
    let t = y("#evt").value.trim(),
      e = new Date(y("#evq").value).getTime(),
      a = y("#evd").value.trim();
    if (!t) return y("#evt").focus();
    if (!Number.isFinite(e)) return _("Escolha a data e a hora");
    F();
    try {
      await h.engine.sendEvent(C.key, t, e, a);
    } catch (n) {
      _(n.message || "N\xE3o foi poss\xEDvel criar o evento.");
    }
  });
  /** Lembrete 15 min antes, para quem não respondeu "não vou". Uma vez por evento. */
  async function jmEventReminders() {
    let t = h.engine;
    if (!t || !t.me) return;
    let e = new Set(await t.kv("evtAvisados", [])),
      a = Date.now(),
      n = !1;
    for (let s of await t.db.all("msgs")) {
      if (s.kind !== "event" || !s.evt || e.has(s.id)) continue;
      let o = s.evt.at - a;
      if (o > 9e5 || o < -6e4 || (s.rsvp || {})[t.pk] === "n") continue;
      e.add(s.id), (n = !0);
      let r = Math.max(0, Math.round(o / 6e4));
      t.settings.notifications &&
        X.notify({ tag: "evt:" + s.id, chat: s.chat, title: "\u{1F4C5} " + s.evt.ti, body: r <= 1 ? "Come\xE7a agora." : `Come\xE7a em ${r} minutos.`, silent: !t.settings.sounds, vibrate: !!t.settings.vibrate });
    }
    n && (await t.setKv("evtAvisados", [...e].slice(-300)));
  }
  // ---- Responder e marcar como lida pela notificação ----
  async function jmNotifAct(t, e) {
    // Pode chegar com a página acabando de abrir (sem tela): espera o motor.
    for (let a = 0; a < 60 && !(h.engine && h.engine.pool && h.engine.me); a++) await new Promise((n) => setTimeout(n, 250));
    let a = h.engine;
    if (!a || typeof t != "string" || a.hasPin) return;
    try {
      e && e.trim() && (await a.sendText(t, e.trim())), await a.markRead(t), X.cancelNotify(t), Ae();
    } catch {}
  }
  // ---- Arquivos, encaminhar, visualização única e prévia de links ----
  var jmLpWait = new Map(),
    jmLpSeq = 0;
  /** Prévia feita pelo lado Android (o site só vê este aparelho). Até 7 s. */
  function jmLinkPreview(t) {
    return new Promise((e) => {
      let a = "lp" + ++jmLpSeq;
      jmLpWait.set(a, (n) => e(jmCleanLp(n))),
        setTimeout(() => jmLpWait.has(a) && (jmLpWait.delete(a), e(null)), 7e3),
        X.linkPreview(a, t);
    });
  }
  function jmLpHtml(t) {
    let e = (t.u.split("://")[1] || "").split("/")[0].replace(/^www\./, "");
    return `<div class="lpcard" data-act="openUrl" data-u="${M(t.u)}">${t.i ? `<img src="${t.i}" alt="">` : ""}<div><small>${M(e)}</small><b>${M(t.t)}</b>${t.d ? `<span>${M(t.d)}</span>` : ""}</div></div>`;
  }
  var jmBytes = (t) => (t < 1024 ? t + " B" : t < 1048576 ? (t / 1024).toFixed(0) + " KB" : (t / 1048576).toFixed(1) + " MB");
  function jmFileHtml(t) {
    let e = t.media,
      a = e.name || "arquivo",
      n = ((a.match(/\.([a-z0-9]{1,5})$/i) || [])[1] || "ARQ").toUpperCase();
    return `<div class="filetile"><span class="fext">${M(n)}</span><div class="fmeta"><b>${M(a)}</b><small>${jmBytes(e.size || 0)}${e.ready ? "" : ` \xB7 recebendo ${e.got || 0}/${e.n}`}</small></div>${e.ready ? `<button class="ibtn sm" data-act="fileSave" data-mid="${e.mid}" data-n="${M(a)}" aria-label="Salvar">${x("download", "sm")}</button>` : '<span class="spin"></span>'}</div>`;
  }
  function jmVoHtml(t) {
    let e = t.media,
      a = e.type === "audio",
      n = t.from === h.engine.pk,
      s = e.opened ? (a ? "\xC1udio ouvido" : "Foto aberta") : a ? "\xC1udio" : "Foto",
      o = e.opened ? "Visualiza\xE7\xE3o \xFAnica" + (n ? " \xB7 j\xE1 foi aberta" : "") : n ? "Visualiza\xE7\xE3o \xFAnica \xB7 ainda n\xE3o aberta" : e.ready ? "Visualiza\xE7\xE3o \xFAnica \xB7 toque para abrir" : "Visualiza\xE7\xE3o \xFAnica \xB7 chegando\u2026",
      r = !e.opened && !n && e.ready;
    return `<div class="votile ${e.opened ? "done" : ""}" ${r ? `data-act="voOpen" data-id="${t.id}"` : ""}><span class="vo1">1</span><div><b>${s}</b><small>${o}</small></div></div>`;
  }
  b("voOpen", async (t) => {
    let e = C && C.key;
    if (!e) return;
    let a = C.msgs.find((o) => o.id === t.dataset.id),
      n;
    try {
      n = await h.engine.openViewOnce(e, t.dataset.id);
    } catch (o) {
      return _(o.message);
    }
    if (a && a.media.type === "audio") return nu("vo:" + t.dataset.id, await n.arrayBuffer()), _("Tocando \u2014 este \xE1udio n\xE3o fica salvo");
    let s = URL.createObjectURL(n),
      r = y("#viewer");
    (r.innerHTML = `<div class="vtop"><button class="ibtn" data-act="closeViewer" aria-label="Fechar">${x("back")}</button><span style="color:#fff;font-weight:600">Visualiza\xE7\xE3o \xFAnica</span></div><img src="${s}" alt="Foto">`),
      (r.hidden = !1),
      X.setSecure(!0),
      (jmVoUrl = s);
  });
  var jmVoUrl = null;
  b("pickFile", () => {
    F(),
      Kn("*/*").then(async (t) => {
        if (!t || !C) return;
        if (t.size > JM_FILE_MAX) return _("Arquivo grande demais: o limite \xE9 5 MB.");
        let e = C.key,
          a = JM_FILE_MIME.test(t.type || "") ? t.type : "application/octet-stream";
        _("Enviando " + t.name + "\u2026");
        try {
          await h.engine.sendMedia(e, new Uint8Array(await t.arrayBuffer()), { type: "file", mime: a, name: t.name });
        } catch (n) {
          _(n.message || "N\xE3o foi poss\xEDvel enviar o arquivo.");
        }
      });
  });
  b("pickPhotoOnce", () => {
    F(),
      Kn("image/*").then(async (t) => {
        if (!t || !C) return;
        let e = C.key;
        try {
          let a = await xu(t);
          await h.engine.sendMedia(e, a.bytes, { type: "image", mime: a.mime, w: a.w, h: a.h, vo: !0 }), _("Foto de visualiza\xE7\xE3o \xFAnica enviada");
        } catch (a) {
          _(a.message || "N\xE3o foi poss\xEDvel enviar a foto.");
        }
      });
  });
  b("fileSave", async (t) => {
    F();
    let e = await h.engine.mediaBlob(t.dataset.mid);
    if (!e) return _("O arquivo ainda n\xE3o chegou inteiro.");
    let a = new Uint8Array(await e.arrayBuffer()),
      n = X.saveFile(t.dataset.n, Fa.enc(a));
    if (n === void 0) {
      let s = document.createElement("a");
      (s.href = URL.createObjectURL(e)), (s.download = t.dataset.n), s.click(), setTimeout(() => URL.revokeObjectURL(s.href), 5e3), _("Arquivo salvo");
    } else _(n ? "Salvo em " + n : "N\xE3o foi poss\xEDvel salvar o arquivo.", 3500);
  });
  b("fwdOpen", (t) => {
    F();
    let e = t.dataset.id,
      a = h.engine,
      n = [...a.chats.values()]
        .filter((s) => s.kind === "group" ? a.groups.has(s.lid) : (a.contacts.get(s.pk) || {}).status === "ok")
        .sort((s, o) => (o.lastTs || 0) - (s.lastTs || 0))
        .slice(0, 60);
    setTimeout(
      () =>
        ee(`<h2 class="sh-title">Encaminhar para\u2026</h2><div class="mlist">${n
          .map((s) => {
            let o = s.kind === "group" ? (a.groups.get(s.lid).meta.channels.find((r) => r.id === s.ch) || {}).name : ne(s.pk),
              r = s.kind === "group" ? a.groups.get(s.lid).meta.name : "";
            return `<button class="mitem" data-act="fwdTo" data-id="${e}" data-k="${s.key}">${x(s.kind === "group" ? "hash" : "user")}<span>${M(o || "")}${r ? `<small>${M(r)}</small>` : ""}</span></button>`;
          })
          .join("") || '<p class="sh-sub">Nenhuma conversa.</p>'}</div>`),
      200,
    );
  });
  b("fwdTo", async (t) => {
    F();
    try {
      await h.engine.forward(t.dataset.id, t.dataset.k), _("Encaminhada");
    } catch (e) {
      _(e.message || "N\xE3o foi poss\xEDvel encaminhar.");
    }
  });
  // Mais opções da mensagem
  function jmCanPin() {
    if (!C) return !1;
    if (C.chat.kind === "dm") return !0;
    let t = h.engine.groups.get(C.chat.lid);
    return !!t && jmCan(t.meta, h.engine.pk, "pin");
  }
  b("msgMore", () => {
    let t = C.msgs.find((o) => o.id === C.pop);
    if (($t(), !t)) return;
    let e = h.engine,
      a = t.from === e.pk && t.kind === "text",
      n = jmCanPin() && t.kind !== "deleted",
      s = (e.chatFor(C.key).pins || []).includes(t.id);
    ee(`<div class="mlist">
    ${a ? `<button class="mitem" data-act="editStart" data-id="${t.id}">${x("edit")}Editar</button>` : ""}
    ${n ? `<button class="mitem" data-act="pinMsg" data-id="${t.id}" data-on="${s ? 0 : 1}">${x("pin")}${s ? "Desafixar" : "Fixar na conversa"}</button>` : ""}
    ${t.kind === "poll" ? `<button class="mitem" data-act="pollWho" data-id="${t.id}">${x("users")}Ver quem votou</button>` : ""}
    ${t.kind === "text" || t.kind === "sound" || (t.kind === "media" && !t.media.vo && t.media.ready) ? `<button class="mitem" data-act="fwdOpen" data-id="${t.id}">${x("share")}Encaminhar</button>` : ""}
    ${t.kind === "media" && t.media.type === "file" && t.media.ready ? `<button class="mitem" data-act="fileSave" data-mid="${t.media.mid}" data-n="${M(t.media.name || "arquivo")}">${x("download")}Salvar arquivo</button>` : ""}
    <button class="mitem danger" data-act="delMsgId" data-id="${t.id}">${x("trash")}Apagar</button></div>`);
  });
  b("delMsgId", (t) => {
    F();
    let e = t.dataset.id;
    setTimeout(() => {
      C && ((C.pop = e), vt.delMsg());
    }, 260);
  });
  b("editStart", (t) => {
    F();
    let e = C.msgs.find((n) => n.id === t.dataset.id);
    if (!e) return;
    (C.edit = { id: e.id }), (C.reply = null);
    y("#replyslot", C.el).innerHTML = `<div class="replybar edit"><div><b>${x("edit", "xs")} Editando mensagem</b>${M(e.text).slice(0, 140)}</div><button class="ibtn sm" data-act="cancelEdit" aria-label="Cancelar edi\xE7\xE3o">${x("x")}</button></div>`;
    let a = y("#msgin", C.el);
    a && ((a.value = e.text), vo(a), wo(), setTimeout(() => a.focus(), 280));
  });
  function jmEditCancel() {
    C && ((C.edit = null), (y("#replyslot", C.el).innerHTML = ""));
  }
  b("pinMsg", async (t) => {
    F();
    let e = t.dataset.on === "1";
    await h.engine.setPin(C.key, t.dataset.id, e), _(e ? "Mensagem fixada" : "Mensagem desafixada");
  });
  async function jmPinBar() {
    if (!C) return;
    let t = y("#pinbar", C.el);
    if (!t) return;
    let e = h.engine,
      a = e.chatFor(C.key).pins || [],
      n = null;
    for (let s of a) {
      let o = C.msgs.find((r) => r.id === s) || (await e.db.get("msgs", s));
      if (o && o.chat === C.key && o.kind !== "deleted") {
        n = o;
        break;
      }
    }
    t.innerHTML = n
      ? `<button class="pinbar" data-act="pinJump" data-id="${n.id}">${x("pin", "sm")}<span><b>Mensagem fixada${a.length > 1 ? ` \xB7 ${a.length}` : ""}</b>${M(e.preview(n)).slice(0, 120)}</span>${a.length > 1 ? `<i data-act="pinsOpen">Ver todas</i>` : ""}</button>`
      : "";
  }
  b("pinJump", (t, e) => {
    e && e.target.closest('[data-act="pinsOpen"]') ? vt.pinsOpen() : jmJump(t.dataset.id);
  });
  b("pinsOpen", async () => {
    F();
    let t = h.engine,
      e = t.chatFor(C.key).pins || [],
      a = [];
    for (let s of e) {
      let o = await t.db.get("msgs", s);
      o && o.chat === C.key && o.kind !== "deleted" && a.push(o);
    }
    let n = jmCanPin();
    setTimeout(
      () =>
        ee(`<h2 class="sh-title">Mensagens fixadas</h2>${a.length ? `<div class="mlist">${a
          .map(
            (s) => `<div class="pinrow"><button class="mitem" data-act="pinGo" data-id="${s.id}"><span><b>${M(s.from === t.pk ? "Voc\xEA" : ne(s.from))}</b><small>${M(t.preview(s)).slice(0, 160)}</small></span></button>${n ? `<button class="ibtn sm" data-act="pinMsg" data-id="${s.id}" data-on="0" aria-label="Desafixar">${x("x")}</button>` : ""}</div>`,
          )
          .join("")}</div>` : `<p class="sh-sub">Nenhuma mensagem fixada. Toque numa mensagem, depois em ${x("more", "xs")} e em “Fixar na conversa”.</p>`}`),
      200,
    );
  });
  b("pinGo", (t) => {
    F(), jmJump(t.dataset.id);
  });
  async function jmJump(t) {
    if (!C) return;
    if (!C.msgs.some((a) => a.id === t)) {
      let a = await h.engine.messages(C.key, 1 / 0, 3e3);
      if (!a.some((n) => n.id === t)) return _("Essa mensagem n\xE3o est\xE1 mais neste aparelho");
      (C.msgs = a), (C.more = a.length >= 3e3), Zt({ keep: !0 });
    }
    setTimeout(() => vt.jumpTo({ dataset: { id: t } }), 60);
  }
  // Buscar na conversa
  var jmNorm = (t) => String(t || "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  b("chatSearch", async () => {
    F();
    let t = await h.engine.messages(C.key, 1 / 0, 5e3);
    (C.searchAll = t.filter((e) => e.kind === "text" || e.kind === "poll")),
      setTimeout(() => {
        ee(`<h2 class="sh-title">Buscar na conversa</h2><input class="inp" id="srchin" placeholder="Digite uma palavra" autocomplete="off" enterkeyhint="search"><div id="srchres" class="srchres"><p class="sh-sub">${C.searchAll.length} mensagens de texto neste aparelho.</p></div>`);
        setTimeout(() => {
          let e = y("#srchin");
          e && e.focus();
        }, 380);
      }, 200);
  });
  document.addEventListener("input", (t) => {
    if (t.target.id !== "srchin" || !C) return;
    let e = jmNorm(t.target.value.trim()),
      a = y("#srchres");
    if (!a) return;
    if (e.length < 2) return void (a.innerHTML = '<p class="sh-sub">Digite pelo menos 2 letras.</p>');
    let n = (C.searchAll || []).filter((s) => jmNorm(s.text || (s.poll && s.poll.q)).includes(e)).reverse().slice(0, 60);
    a.innerHTML = n.length
      ? n
          .map((s) => {
            let o = s.text || s.poll.q,
              r = jmNorm(o).indexOf(e),
              i = Math.max(0, r - 30),
              c = (i ? "…" : "") + o.slice(i, r) + "\u0001" + o.slice(r, r + e.length) + "\u0002" + o.slice(r + e.length, r + e.length + 80);
            return `<button class="srchrow" data-act="pinGo" data-id="${s.id}"><b>${M(s.from === h.engine.pk ? "Voc\xEA" : ne(s.from))} <small>${M(_d(s.ts))} ${so(s.ts)}</small></b><span>${M(c).replace("\u0001", "<mark>").replace("\u0002", "</mark>")}</span></button>`;
          })
          .join("")
      : '<p class="sh-sub">Nada encontrado.</p>';
  });
  // Enquetes
  function jmPollHtml(t) {
    let e = h.engine,
      a = t.poll || { q: "", o: [] },
      n = t.votes || {},
      s = Object.keys(n).length,
      o = a.o.map(() => 0);
    for (let i of Object.values(n)) for (let c of i) o[c] != null && o[c]++;
    let r = n[e.pk] || [];
    return `<div class="poll"><div class="pq">${x("checks", "xs")} ${M(a.q)}</div><small class="pm">${a.m ? "Escolha uma ou mais op\xE7\xF5es" : "Escolha uma op\xE7\xE3o"}</small>
    ${a.o
      .map((i, c) => {
        let l = s ? Math.round((o[c] * 100) / s) : 0,
          u = r.includes(c);
        return `<button class="popt ${u ? "on" : ""}" data-act="pollVote" data-id="${t.id}" data-i="${c}"><i class="pbar" style="width:${l}%"></i><span class="pck">${u ? x("check", "xs") : ""}</span><span class="ptx">${M(i)}</span><span class="pn">${o[c]}</span></button>`;
      })
      .join("")}<button class="pfoot" data-act="pollWho" data-id="${t.id}">${s} ${s === 1 ? "voto" : "votos"} \xB7 ver quem votou</button></div>`;
  }
  b("pollVote", async (t) => {
    let e = C.msgs.find((o) => o.id === t.dataset.id);
    if (!e) return;
    let a = +t.dataset.i,
      n = (e.votes || {})[h.engine.pk] || [],
      s = e.poll.m ? (n.includes(a) ? n.filter((o) => o !== a) : [...n, a].sort()) : n.includes(a) ? [] : [a];
    X.vibrate(10), await h.engine.vote(C.key, e.id, s);
  });
  b("pollWho", async (t) => {
    F();
    let e = C.msgs.find((n) => n.id === t.dataset.id) || (await h.engine.db.get("msgs", t.dataset.id));
    if (!e) return;
    let a = e.votes || {};
    setTimeout(
      () =>
        ee(`<h2 class="sh-title">${M(e.poll.q)}</h2>${e.poll.o
          .map((n, s) => {
            let o = Object.entries(a).filter(([, r]) => r.includes(s)).map(([r]) => r);
            return `<div class="label" style="padding-left:0">${M(n)} \xB7 ${o.length}</div>${o.length ? o.map((r) => `<div class="voter">${pe(r, 28, { badge: !1 })}<span>${M(r === h.engine.pk ? "Voc\xEA" : ne(r))}</span></div>`).join("") : '<p class="sh-sub">Ningu\xE9m ainda</p>'}`;
          })
          .join("")}`),
      200,
    );
  });
  b("pollNew", () => {
    F(),
      setTimeout(
        () =>
          ee(`<h2 class="sh-title">Nova enquete</h2>
    <label class="lbl" for="pq">Pergunta</label><input class="inp" id="pq" maxlength="200" placeholder="Ex.: Que dia fica melhor?" autocomplete="off">
    <span class="lbl">Op\xE7\xF5es</span><div id="popts"><input class="inp popin" maxlength="80" placeholder="Op\xE7\xE3o 1" autocomplete="off"><input class="inp popin" maxlength="80" placeholder="Op\xE7\xE3o 2" autocomplete="off"></div>
    <button class="btn ghost" data-act="pollAddOpt" style="margin-top:6px">${x("plus", "sm")}&nbsp;Adicionar op\xE7\xE3o</button>
    <label class="pick" style="padding:12px 2px"><span style="flex:1">Permitir mais de uma resposta</span><input type="checkbox" id="pmulti"></label>
    <div class="errtx" id="perr" hidden>Escreva a pergunta e pelo menos duas op\xE7\xF5es.</div>
    <button class="btn" data-act="pollSend">Enviar enquete</button>`),
        200,
      );
  });
  b("pollAddOpt", () => {
    let t = y("#popts"),
      e = oe(".popin", t).length;
    if (e >= 10) return _("No m\xE1ximo 10 op\xE7\xF5es");
    t.insertAdjacentHTML("beforeend", `<input class="inp popin" maxlength="80" placeholder="Op\xE7\xE3o ${e + 1}" autocomplete="off">`), t.lastElementChild.focus();
  });
  b("pollSend", async () => {
    let t = y("#pq").value.trim(),
      e = oe(".popin").map((a) => a.value.trim()).filter(Boolean);
    if (!t || e.length < 2) return void (y("#perr").hidden = !1);
    F();
    try {
      await h.engine.sendPoll(C.key, t, e, y("#pmulti") && y("#pmulti").checked);
    } catch (a) {
      _(a.message || "N\xE3o foi poss\xEDvel enviar a enquete.");
    }
  });
  // Online / ausente
  function jmPresTick(t = !1) {
    let e = h.engine;
    e && e.pool && e.setPresence(document.hidden ? "away" : "on", t).catch(() => {});
  }
  document.addEventListener("visibilitychange", () => jmPresTick(!0));
  // Cargos do grupo
  function jmRolesSheet() {
    let t = h.engine.groups.get(se.lid);
    if (!t) return;
    let e = cn(t, h.engine.pk),
      a = t.meta.roles || [];
    ee(`<h2 class="sh-title">Cargos</h2><p class="sh-sub">Cargos d\xE3o nome e cor \xE0s pessoas do grupo. ${e ? "Toque num cargo para editar; para dar um cargo, toque na pessoa em Membros." : "S\xF3 admins criam e d\xE3o cargos."}</p>
    <div class="mlist">${a.length ? a.map((n) => {
      let s = Object.values(t.meta.mroles || {}).filter((o) => o.includes(n.id)).length;
      return `<button class="mitem" ${e ? `data-act="roleEdit" data-id="${n.id}"` : ""}><i class="rdot" style="background:${JM_ROLE_CORES[n.c]}"></i><span>${M(n.n)}<small>${s} ${s === 1 ? "pessoa" : "pessoas"}</small></span></button>`;
    }).join("") : '<p class="sh-sub">Nenhum cargo ainda.</p>'}</div>
    ${e && a.length < 12 ? `<button class="btn" data-act="roleEdit" data-id="">${x("plus", "sm")}&nbsp;Novo cargo</button>` : ""}`);
  }
  b("rolesOpen", () => jmRolesSheet());
  var jmRoleCor = 0;
  b("roleEdit", (t) => {
    let e = h.engine.groups.get(se.lid),
      a = (e.meta.roles || []).find((n) => n.id === t.dataset.id);
    (jmRoleCor = a ? a.c : (e.meta.roles || []).length % 8),
      F(),
      setTimeout(
        () =>
          ee(`<h2 class="sh-title">${a ? "Editar cargo" : "Novo cargo"}</h2>
    <label class="lbl" for="rname">Nome</label><input class="inp" id="rname" maxlength="24" placeholder="Ex.: Moderador" value="${a ? M(a.n) : ""}" autocomplete="off">
    <span class="lbl">Cor</span><div class="swatches">${JM_ROLE_CORES.map((n, s) => `<button class="swatch ${s === jmRoleCor ? "on" : ""}" data-act="roleColor" data-c="${s}" style="background:${n}" aria-label="Cor ${s + 1}"></button>`).join("")}</div>
    <span class="lbl">Poderes</span>${JM_PERMS.map(([n, s]) => `<label class="pick" style="padding:8px 2px"><span style="flex:1">${s}</span><input type="checkbox" class="rperm" value="${n}" ${a && (a.p || []).includes(n) ? "checked" : ""}></label>`).join("")}
    <p class="note" style="padding:4px 2px 10px">Mudar canais, membros e cargos continua s\xF3 com admins.</p>
    <button class="btn" data-act="roleSave" data-id="${a ? a.id : ""}">Salvar</button>
    ${a ? `<button class="btn ghost danger" data-act="roleDel" data-id="${a.id}">Apagar cargo</button>` : ""}`),
        200,
      );
  });
  b("roleColor", (t) => {
    (jmRoleCor = +t.dataset.c), oe(".swatch").forEach((e) => e.classList.toggle("on", e === t));
  });
  b("roleSave", async (t) => {
    let e = y("#rname").value.trim();
    if (!e) return y("#rname").focus();
    let a = t.dataset.id;
    let jmP = oe(".rperm").filter((n) => n.checked).map((n) => n.value);
    F();
    await h.engine.editGroup(se.lid, a ? { editRole: { id: a, n: e, c: jmRoleCor, p: jmP } } : { addRole: { n: e, c: jmRoleCor, p: jmP } }), Be(), _(a ? "Cargo salvo" : "Cargo criado");
  });
  b("roleDel", async (t) => {
    F(), await h.engine.editGroup(se.lid, { removeRole: t.dataset.id }), Be(), _("Cargo apagado");
  });
  b("memberRoles", (t) => {
    let e = h.engine.groups.get(se.lid),
      a = t.dataset.pk,
      n = e.meta.roles || [];
    F(),
      setTimeout(
        () =>
          ee(`<h2 class="sh-title">Cargos de ${M(ne(a))}</h2>${n.length ? `<div class="rolechips">${n.map((s) => `<button class="rchip ${jmRolesOf(e, a).includes(s) ? "on" : ""}" data-act="roleToggle" data-pk="${a}" data-id="${s.id}" style="--rc:${JM_ROLE_CORES[s.c]}"><i></i>${M(s.n)}</button>`).join("")}</div><p class="note">At\xE9 4 cargos por pessoa.</p>` : `<p class="sh-sub">O grupo ainda n\xE3o tem cargos.</p><button class="btn" data-act="roleEdit" data-id="">Criar cargo</button>`}`),
        200,
      );
  });
  b("roleToggle", async (t) => {
    let e = h.engine.groups.get(se.lid),
      a = t.dataset.pk,
      n = !jmRolesOf(e, a).some((s) => s.id === t.dataset.id);
    if (n && jmRolesOf(e, a).length >= 4) return _("No m\xE1ximo 4 cargos por pessoa");
    t.classList.toggle("on", n), await h.engine.editGroup(se.lid, { memberRole: { pk: a, id: t.dataset.id, on: n } }), Be();
  });
  b("msgTap", (t, e) => {
    (e.target.closest("[data-act]:not(.msg)") &&
      e.target.closest("[data-act]") !== t) ||
      (C.pop === t.dataset.id ? $t() : w0(t.dataset.id, t));
  });
  b("react", async (t) => {
    let e = C.pop;
    $t(), await h.engine.sendReaction(C.key, e, t.dataset.e);
  });
  b("replyTo", () => {
    let t = C.msgs.find((a) => a.id === C.pop);
    if (($t(), !t)) return;
    (C.reply = { id: t.id, from: t.from }),
      (y("#replyslot", C.el).innerHTML =
        `<div class="replybar"><div><b>${M(t.from === h.engine.pk ? "Voc\xEA" : ne(t.from))}</b>${M(h.engine.preview(t))}</div><button class="ibtn sm" data-act="cancelReply" aria-label="Cancelar resposta">${x("x")}</button></div>`);
    let e = y("#msgin", C.el);
    e && e.focus({ preventScroll: !0 });
  });
  b("cancelReply", () => {
    (C.reply = null), (y("#replyslot", C.el).innerHTML = "");
  });
  b("cancelEdit", () => {
    let t = y("#msgin", C.el);
    t && ((t.value = ""), vo(t), wo()), jmEditCancel();
  });
  b("copyMsg", async () => {
    let t = C.msgs.find((e) => e.id === C.pop);
    $t(),
      t && (await X.copy(t.text || ""))
        ? _("Mensagem copiada")
        : _("N\xE3o foi poss\xEDvel copiar");
  });
  b("delMsg", () => {
    let t = C.msgs.find((a) => a.id === C.pop);
    if (($t(), !t)) return;
    let jmG = C.chat.kind === "group" && h.engine.groups.get(C.chat.lid),
      e = (t.from === h.engine.pk && Date.now() - t.ts < 48 * 36e5 || (!!jmG && jmCan(jmG.meta, h.engine.pk, "del"))) && t.kind !== "deleted";
    ee(`<h2 class="sh-title">Apagar mensagem?</h2><div class="mlist">
    ${e ? `<button class="mitem danger" data-act="delAll" data-id="${t.id}">${x("trash")}<span>Apagar para todos<small>Some tamb\xE9m no aparelho de quem recebeu</small></span></button>` : ""}
    <button class="mitem" data-act="delMe" data-id="${t.id}">${x("trash")}<span>Apagar s\xF3 para mim</span></button></div>`);
  });
  b("delAll", async (t) => {
    F(), await h.engine.deleteForAll(C.key, [t.dataset.id]);
  });
  b("delMe", async (t) => {
    F(), await h.engine.deleteForMe(C.key, [t.dataset.id]);
  });
  b("retry", async () => {
    await h.engine.flushOutbox(), _("Tentando enviar de novo");
  });
  b("jumpTo", (t) => {
    let e = y(`[data-mid="${t.dataset.id}"]`, C.el);
    if (!e) return _("Mensagem antiga demais para mostrar aqui");
    e.scrollIntoView({ behavior: "smooth", block: "center" }),
      e.classList.remove("hl"),
      e.offsetWidth,
      e.classList.add("hl");
  });
  b("openUrl", async (t) => {
    let e = t.dataset.u;
    (await ge({ title: "Abrir link?", text: e, ok: "Abrir no navegador" })) &&
      X.openUrl(e);
  });
  b("attach", () =>
    ee(`<h2 class="sh-title">Anexar</h2><div class="agrid">
  <button class="atile" data-act="pickPhoto"><span class="ab">${x("image")}</span>Galeria</button>
  <button class="atile" data-act="cameraPhoto"><span class="ab">${x("camera")}</span>C\xE2mera</button>
  <button class="atile hot" data-act="attachSound"><span class="ab">${x("wave")}</span>Som</button>
  <button class="atile" data-act="attachVoice"><span class="ab">${x("mic")}</span>\xC1udio</button>
  <button class="atile" data-act="gifSheet"><span class="ab">${x("gif")}</span>GIF</button>
  <button class="atile" data-act="pollNew"><span class="ab">${x("checks")}</span>Enquete</button>
  <button class="atile" data-act="pickFile"><span class="ab">${x("file")}</span>Arquivo</button>
  <button class="atile" data-act="evtNew"><span class="ab">${x("clock")}</span>Evento</button>
  <button class="atile" data-act="pickPhotoOnce"><span class="ab">${x("eye")}</span>Foto \xFAnica</button></div>
  <p class="note" style="padding:14px 2px 0">Fotos s\xE3o reduzidas e os metadados (como localiza\xE7\xE3o) s\xE3o removidos antes de sair do aparelho.</p>`),
  );
  b("attachVoice", () => {
    F(), _u();
  });
  b("pickPhoto", () => {
    F(),
      Kn("image/*").then((t) => {
        t && ju(t);
      });
  });
  b("cameraPhoto", () => {
    F(), setTimeout(k0, 250);
  });
  async function k0() {
    let t = await Ci("environment");
    t && ju(t);
  }
  async function ju(t) {
    let e = C && C.key;
    if (!e) return;
    let a;
    try {
      a = t.type === "image/gif" && t.size <= JM_GIF_MAX ? await jmGifPrep(t) : await xu(t);
    } catch {
      _("N\xE3o foi poss\xEDvel abrir essa imagem.");
      return;
    }
    let n = URL.createObjectURL(new Blob([a.bytes], { type: a.mime }));
    ee(
      `<div class="camsheet"><img src="${n}" alt="Pr\xE9via da foto"><p class="note" style="padding:10px 2px 0">${Math.round(a.bytes.length / 1024)} KB \xB7 ${a.w}\xD7${a.h} \xB7 sem metadados</p>
    <div class="two" style="margin-top:12px"><button class="btn ghost" data-act="closeSheet" style="margin:0">Cancelar</button><button class="btn" data-act="sendPhoto" style="margin:0">Enviar foto</button></div></div>`,
      { onClose: () => URL.revokeObjectURL(n) },
    ),
      b("sendPhoto", async () => {
        F();
        try {
          await h.engine.sendMedia(e, a.bytes, {
            type: "image",
            mime: a.mime,
            w: a.w,
            h: a.h,
            th: a.th,
          });
        } catch (s) {
          _(s.message || "N\xE3o foi poss\xEDvel enviar a foto.");
        }
      });
  }
  b("viewPhoto", (t) => {
    let e = Yn.get(t.dataset.mid);
    if (!e) return;
    let a = y("#viewer");
    (a.innerHTML = `<div class="vtop"><button class="ibtn" data-act="closeViewer" aria-label="Fechar">${x("back")}</button></div><img src="${e}" alt="Foto">`),
      (a.hidden = !1);
  });
  b("closeViewer", () => {
    jmVoUrl && (URL.revokeObjectURL(jmVoUrl), (jmVoUrl = null), X.setSecure(!!h.engine.settings.lockScreenshots));
    (y("#viewer").hidden = !0), (y("#viewer").innerHTML = "");
  });
  b("attachSound", () => P.soundSheet("chat"));
  /* ---------- GIFs (GIPHY com a chave do app; GIF da galeria) ---------- */
  var JM_GIF_MAX = 2e6,
    JM_GIPHY_KEY = "HPMOkQnffVZSf1lRx8PmAhJ5j2D0UMgj",
    jmGif = { seq: 0, t: null, hits: [], sending: !1 };
  /** GIF pronto para mandar: os bytes como vieram, tamanho e miniatura do 1º quadro. */
  async function jmGifPrep(t) {
    let e = new Uint8Array(await t.arrayBuffer());
    if (e.length > JM_GIF_MAX) throw new Error("GIF grande demais (at\xE9 2 MB)");
    let a = await createImageBitmap(t),
      n = a.width || 1,
      s = a.height || 1,
      o = 24 / Math.max(n, s),
      r = document.createElement("canvas");
    (r.width = Math.max(1, Math.round(n * o))), (r.height = Math.max(1, Math.round(s * o))), r.getContext("2d").drawImage(a, 0, 0, r.width, r.height), a.close && a.close();
    return { bytes: e, mime: "image/gif", w: n, h: s, th: r.toDataURL("image/jpeg", 0.5) };
  }
  function jmGiphyHits(t) {
    return ((t && t.data) || [])
      .map((e) => {
        let a = e.images || {},
          n = a.fixed_width_downsampled || a.fixed_width_small || a.fixed_width,
          s = [a.downsized, a.fixed_width].find((o) => o && o.url && +o.size > 0 && +o.size <= JM_GIF_MAX) || n;
        return n && n.url ? { id: e.id, title: e.title || "", prev: n.url, send: s.url, w: +n.width || 200, h: +n.height || 150 } : null;
      })
      .filter(Boolean);
  }
  async function jmGiphySearch(t) {
    let e = ++jmGif.seq,
      a = y("#gifgrid");
    a && (a.innerHTML = '<div class="note" style="padding:20px;column-span:all">Buscando\u2026</div>');
    let n = t.trim()
      ? "https://api.giphy.com/v1/gifs/search?api_key=" + JM_GIPHY_KEY + "&q=" + encodeURIComponent(t.trim()) + "&limit=24&rating=pg-13&lang=pt"
      : "https://api.giphy.com/v1/gifs/trending?api_key=" + JM_GIPHY_KEY + "&limit=24&rating=pg-13";
    try {
      let s = await fetch(n, { referrerPolicy: "no-referrer", credentials: "omit" });
      if (!s.ok) throw new Error(s.status === 429 ? "Limite de buscas do GIPHY atingido. Tente daqui a pouco." : "O GIPHY respondeu com erro " + s.status + ".");
      let o = jmGiphyHits(await s.json());
      if (e !== jmGif.seq) return;
      (jmGif.hits = o), (a = y("#gifgrid"));
      a &&
        (a.innerHTML = o.length
          ? o.map((r, i) => `<button data-act="gifPick" data-i="${i}" style="aspect-ratio:${r.w}/${r.h}"><img src="${M(r.prev)}" alt="${M(r.title)}" loading="lazy" referrerpolicy="no-referrer"></button>`).join("")
          : '<div class="note" style="padding:20px;column-span:all">Nenhum GIF encontrado.</div>');
    } catch (s) {
      e === jmGif.seq && (a = y("#gifgrid")) && (a.innerHTML = `<div class="errtx" style="padding:20px;column-span:all">${M(s.message && !/fetch/i.test(s.message) ? s.message : "Sem conex\xE3o com o GIPHY. Confira a internet.")}</div>`);
    }
  }
  b("gifSheet", () => {
    F(),
      setTimeout(() => {
        ee(`<h2 class="sh-title">GIFs</h2>
      <label class="search" style="margin:4px 0 0"><svg class="i sm"><use href="#i-search"/></svg><input id="gifq" type="search" placeholder="Buscar no GIPHY" autocomplete="off"></label>
      <div class="gifgrid" id="gifgrid"></div>
      <div class="gifcred">Powered by GIPHY</div>
      <button class="btn ghost" data-act="gifFromGallery">${x("image", "sm")}&nbsp;GIF da galeria</button>`),
          jmGiphySearch("");
      }, 240);
  });
  document.addEventListener("input", (t) => {
    t.target.id === "gifq" && (clearTimeout(jmGif.t), (jmGif.t = setTimeout(() => jmGiphySearch(t.target.value), 380)));
  });
  b("gifFromGallery", () => {
    F(),
      Kn("image/gif").then((t) => {
        t && ju(t);
      });
  });
  b("gifPick", async (t) => {
    let e = jmGif.hits[+t.dataset.i],
      a = C && C.key;
    if (!e || !a || jmGif.sending) return;
    (jmGif.sending = !0), t.insertAdjacentHTML("beforeend", '<span class="sending">Enviando\u2026</span>');
    try {
      let n = await fetch(e.send, { referrerPolicy: "no-referrer", credentials: "omit" });
      if (!n.ok) throw new Error("N\xE3o deu para baixar o GIF.");
      let s = await n.blob();
      if (s.size > JM_GIF_MAX) throw new Error("GIF grande demais.");
      let o = await jmGifPrep(new Blob([s], { type: "image/gif" }));
      F(), await h.engine.sendMedia(a, o.bytes, { type: "image", mime: o.mime, w: o.w, h: o.h, th: o.th });
    } catch (n) {
      _(n.message || "N\xE3o deu para mandar o GIF.");
      let s = t.querySelector(".sending");
      s && s.remove();
    } finally {
      jmGif.sending = !1;
    }
  });
  b("emoji", () => {
    ee(
      `<h2 class="sh-title">Emojis</h2><div class="egrid">${["\u{1F600}", "\u{1F602}", "\u{1F979}", "\u{1F60D}", "\u{1F60E}", "\u{1F914}", "\u{1F62E}", "\u{1F622}", "\u{1F621}", "\u{1F64F}", "\u{1F44D}", "\u{1F44F}", "\u{1F525}", "\u2764\uFE0F", "\u{1F389}", "\u{1F3B8}", "\u{1F3AE}", "\u2615", "\u{1F355}", "\u2728", "\u{1F4AC}", "\u{1F440}", "\u{1F91D}", "\u{1F4AF}", "\u{1F634}", "\u{1F92F}", "\u{1F973}", "\u{1F605}"].map((e) => `<button data-act="putEmoji" data-e="${e}">${e}</button>`).join("")}</div>`,
    );
  });
  b("putEmoji", (t) => {
    let e = C && y("#msgin", C.el);
    e && ((e.value += t.dataset.e), vo(e), wo());
  });
  b("acceptInvite", async (t) => {
    let e = await h.engine.acceptInvite(t.dataset.id);
    e &&
      ((h.invites = h.invites.filter((a) => a.msgId !== t.dataset.id)),
      P.saveInvites(),
      _("Voc\xEA entrou em " + e.meta.name),
      P.openGroup(e.lid));
  });
  b("openGroupLid", (t) => P.openGroup(t.dataset.lid));
  b("reqAccept", async () => {
    await h.engine.acceptContact(C.chat.pk),
      (y("#cfoot", C.el).innerHTML = Ri()),
      sn(),
      _("Contato aceito");
  });
  b("reqBlock", async () => {
    (await ge({
      title: "Bloquear este contato?",
      text: "Voc\xEA n\xE3o vai receber mensagens nem chamadas dele. Ele n\xE3o \xE9 avisado.",
      ok: "Bloquear",
      danger: !0,
    })) &&
      (await h.engine.setContactStatus(C.chat.pk, "blocked"),
      J.pop(),
      _("Contato bloqueado"));
  });
  b("reqUnblock", async () => {
    await h.engine.setContactStatus(C.chat.pk, "ok"),
      (y("#cfoot", C.el).innerHTML = Ri());
  });
  b("chatCall", (t) => P.startCall(C.chat.pk, !!t.dataset.video));
  b("chatToGroup", () => {
    let t = C.chat.lid;
    J.find("group") ? J.pop() : (J.pop(!0), P.openGroup(t));
  });
  b("chatMenu", () => {
    let t = h.engine,
      e = t.chatFor(C.key);
    e.kind === "dm"
      ? ee(`<div class="mlist">
      <button class="mitem" data-act="chatSearch">${x("search")}Buscar na conversa</button>
      <button class="mitem" data-act="pinsOpen">${x("pin")}Mensagens fixadas</button>
      <button class="mitem" data-act="openProfileCur">${x("user")}Ver perfil e c\xF3digo de seguran\xE7a</button>
      <button class="mitem" data-act="ttlSheet">${x("timer")}<span>Mensagens tempor\xE1rias<small>${M(yt(e.ttl))}</small></span></button>
      <button class="mitem" data-act="toggleMute">${x(e.muted ? "bell" : "belloff")}${e.muted ? "Ativar notifica\xE7\xF5es" : "Silenciar"}</button>
      <button class="mitem" data-act="togglePin">${x("pin")}${e.pinned ? "Desafixar" : "Fixar no topo"}</button>
      <button class="mitem" data-act="clearChat">${x("trash")}Limpar conversa</button>
      <button class="mitem danger" data-act="blockCur">${x("ban")}Bloquear</button></div>`)
      : ee(`<div class="mlist">
      <button class="mitem" data-act="chatSearch">${x("search")}Buscar no canal</button>
      <button class="mitem" data-act="pinsOpen">${x("pin")}Mensagens fixadas</button>
      <button class="mitem" data-act="chatToGroupS">${x("users")}Canais e membros</button>
      <button class="mitem" data-act="toggleMute">${x(e.muted ? "bell" : "belloff")}${e.muted ? "Ativar notifica\xE7\xF5es do canal" : "Silenciar canal"}</button>
      <button class="mitem" data-act="clearChat">${x("trash")}Limpar canal neste aparelho</button></div>`);
  });
  b("openProfileCur", () => {
    F(), P.openProfile(C.chat.pk);
  });
  b("chatToGroupS", () => {
    F(), E0();
  });
  function E0() {
    let t = C.chat.lid;
    J.find("group") ? J.pop() : (J.pop(!0), P.openGroup(t));
  }
  b("toggleMute", async () => {
    let t = h.engine.chatFor(C.key);
    (t.muted = !t.muted),
      await h.engine.saveChat(t),
      F(),
      _(t.muted ? "Silenciada" : "Notifica\xE7\xF5es ativadas"),
      P.refreshLists();
  });
  b("togglePin", async () => {
    let t = h.engine.chatFor(C.key);
    (t.pinned = !t.pinned), await h.engine.saveChat(t), F(), P.refreshLists();
  });
  b("clearChat", async () => {
    F(),
      (await ge({
        title: "Limpar esta conversa?",
        text: "As mensagens somem s\xF3 deste aparelho.",
        ok: "Limpar",
        danger: !0,
      })) &&
        (await h.engine.deleteChat(C.key),
        (C.msgs = []),
        (C.more = !1),
        Zt({ end: !0 }),
        P.refreshLists());
  });
  b("blockCur", async () => {
    F(),
      (await ge({
        title: "Bloquear " + ne(C.chat.pk) + "?",
        text: "Voc\xEA n\xE3o vai receber mensagens nem chamadas. A pessoa n\xE3o \xE9 avisada.",
        ok: "Bloquear",
        danger: !0,
      })) &&
        (await h.engine.setContactStatus(C.chat.pk, "blocked"),
        J.pop(),
        _("Contato bloqueado"));
  });
  function Di(t) {
    let e = h.engine.chatFor(t);
    ee(`<h2 class="sh-title">Mensagens tempor\xE1rias</h2><p class="sh-sub">Novas mensagens somem dos dois aparelhos e dos servidores depois do tempo escolhido.</p>
    <div class="opts">${[
      [0, "Desligadas"],
      [300, "5 minutos"],
      [3600, "1 hora"],
      [86400, "1 dia"],
      [604800, "7 dias"],
    ]
      .map(
        ([n, s]) =>
          `<button class="opt ${e.ttl === n ? "on" : ""}" data-act="setTtl" data-key="${t}" data-v="${n}"><span class="rad"></span>${s}</button>`,
      )
      .join("")}</div>`);
  }
  b("ttlSheet", () => Di(C.key));
  b("setTtl", async (t) => {
    F(),
      await h.engine.setTTL(t.dataset.key, +t.dataset.v),
      sn(),
      _(
        +t.dataset.v
          ? "Mensagens tempor\xE1rias: " + yt(+t.dataset.v)
          : "Mensagens tempor\xE1rias desligadas",
      );
  });
  function A0() {
    let t = h.engine;
    return `jmenseger://add?k=${t.myNpub()}&n=${encodeURIComponent(t.me.name)}`;
  }
  function S0() {
    return `Fale comigo no J-Menseger (mensagens cifradas de ponta a ponta).
Meu c\xF3digo: ${h.engine.myNpub()}

No app: Adicionar contato \u2192 cole este c\xF3digo.`;
  }
  function qu() {
    let t = J.open(
      "newchat",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Nova conversa</h2></div>
    <label class="search"><svg class="i sm"><use href="#i-search"/></svg><input id="nq" type="search" placeholder="Buscar contato" autocomplete="off"></label>
    <div class="scroll" id="nlist"></div></div>`,
    );
    Pu(""), qt(y("#nlist", t));
  }
  function Pu(t) {
    let a = [...h.engine.contacts.values()]
        .filter((s) => s.status === "ok" && Ja(ne(s.pk)).includes(Ja(t)))
        .sort((s, o) => ne(s.pk).localeCompare(ne(o.pk))),
      n = y("#nlist");
    n &&
      (n.innerHTML =
        (t
          ? ""
          : `
    <div class="row" role="button" tabindex="0" data-act="addContact"><span class="cico acc" style="width:52px;height:52px;border-radius:50%">${x("userplus")}</span><div class="rmain"><b class="rname">Adicionar contato</b><span class="sub">Por c\xF3digo, link ou QR</span></div></div>
    <div class="row" role="button" tabindex="0" data-act="newGroupFromList"><span class="cico" style="width:52px;height:52px;border-radius:32%">${x("users")}</span><div class="rmain"><b class="rname">Novo grupo</b><span class="sub">Canais de texto e salas de voz</span></div></div>
    <div class="row" role="button" tabindex="0" data-act="myCode"><span class="cico" style="width:52px;height:52px;border-radius:50%">${x("qr")}</span><div class="rmain"><b class="rname">Meu c\xF3digo</b><span class="sub">Mostre para algu\xE9m te adicionar</span></div></div>`) +
        '<div class="label">Contatos</div>' +
        (a.length
          ? a
              .map(
                (s) =>
                  `<div class="row" role="button" tabindex="0" data-act="startChat" data-pk="${s.pk}">${pe(s.pk, 50)}<div class="rmain"><b class="rname">${M(ne(s.pk))}</b><span class="sub">${M(s.note || "J-Menseger")}</span></div></div>`,
              )
              .join("")
          : '<div class="empty">Voc\xEA ainda n\xE3o tem contatos. Adicione algu\xE9m pelo c\xF3digo.</div>'));
  }
  document.addEventListener("input", (t) => {
    t.target.id === "nq" && Pu(t.target.value);
  });
  b("startChat", (t) => {
    J.pop(!0), P.openChat("dm:" + t.dataset.pk, { focus: !0 });
  });
  b("newGroupFromList", () => {
    J.pop(!0), P.newGroup();
  });
  function on(t = "") {
    ee(`<h2 class="sh-title">Adicionar contato</h2><p class="sh-sub">Cole o c\xF3digo que a pessoa compartilhou (come\xE7a com <b>npub1</b>), digite o <b>c\xF3digo curto</b> dela (como K7QF-9X2M) ou leia o QR.</p>
    <label class="lbl" for="addcode">C\xF3digo</label>
    <textarea class="inp mono" id="addcode" rows="3" placeholder="npub1\u2026 ou K7QF-9X2M" spellcheck="false" autocapitalize="off">${M(t)}</textarea>
    <div class="errtx" id="adderr" hidden></div>
    <label class="lbl" for="addnick">Nome (opcional)</label>
    <input class="inp" id="addnick" maxlength="40" placeholder="Como voc\xEA quer ver esse contato" autocomplete="off">
    <div class="two" style="margin-top:16px"><button class="btn ghost" data-act="pasteCode" style="margin:0">${x("copy", "sm")}&nbsp;Colar</button><button class="btn ghost" data-act="scanCode" style="margin:0">${x("scan", "sm")}&nbsp;Ler QR</button></div>
    <button class="btn" data-act="confirmAdd">Adicionar e conversar</button>`);
  }
  b("addContact", () => {
    J.find("newchat") && J.pop(!0), on();
  });
  b("pasteCode", async () => {
    let t = X.paste();
    if (!t)
      try {
        t = await navigator.clipboard.readText();
      } catch {}
    t
      ? ((y("#addcode").value = t.trim()), (y("#adderr").hidden = !0))
      : _("Nada para colar. Copie o c\xF3digo primeiro.");
  });
  b("scanCode", () => Vu((t) => on(t)));
  b("confirmAdd", async () => {
    let t = y("#addcode").value.trim(),
      e = y("#addnick").value.trim(),
      a = y("#adderr"),
      n = (t.match(/[?&]n=([^&\s]+)/) || [])[1];
    try {
      let s = to(t);
      if (!s && jmShortParse(t)) {
        let o = y("[data-act=confirmAdd]");
        o && ((o.disabled = !0), (o.textContent = "Procurando o c\xF3digo\u2026"));
        try {
          s = await h.engine.resolveShortCode(t);
        } finally {
          o && ((o.disabled = !1), (o.textContent = "Adicionar e conversar"));
        }
      }
      if (!s)
        throw new Error(
          "C\xF3digo inv\xE1lido. Confira se copiou tudo, come\xE7ando por npub1.",
        );
      let o = await h.engine.addContact(
        s,
        e || (n ? decodeURIComponent(n) : ""),
      );
      F(), _("Contato adicionado"), P.openChat("dm:" + o.pk, { focus: !0 });
    } catch (s) {
      (a.textContent = s.message),
        (a.hidden = !1),
        y("#addcode").classList.add("err"),
        setTimeout(
          () => y("#addcode") && y("#addcode").classList.remove("err"),
          500,
        );
    }
  });
  function Vu(t) {
    let e = new po();
    ee(
      `<h2 class="sh-title">Ler c\xF3digo QR</h2><p class="sh-sub">Aponte para o QR que aparece em "Meu c\xF3digo" no aparelho da outra pessoa.</p>
    <div class="scanbox"><video id="scanv" autoplay playsinline muted></video></div><p class="scanmsg" id="scanmsg">Procurando um c\xF3digo\u2026</p>`,
      { onClose: () => e.stop() },
    ),
      e
        .start(y("#scanv"), (a) => {
          if ((X.vibrate(30), !to(a))) {
            _("Esse QR n\xE3o \xE9 um c\xF3digo do J-Menseger.");
            return;
          }
          F(), setTimeout(() => t(a), 300);
        })
        .catch((a) => {
          F(), _(ze(a));
        });
  }
  function Ni() {
    let t = h.engine;
    ee(`<h2 class="sh-title">Meu c\xF3digo</h2><p class="sh-sub">Quem ler este QR ou receber seu c\xF3digo consegue te mandar mensagens. N\xE3o \xE9 preciso telefone nem e-mail.</p>
    <div class="qrcard">${vu(A0())}<span class="qn">${M(t.me.name)}</span><small>J-Menseger</small></div>
    <div class="idbox selectable"><span>${M(t.myNpub())}</span><button class="ibtn sm" data-act="copyMyCode" aria-label="Copiar c\xF3digo">${x("copy", "sm")}</button></div>
    <div class="two" style="margin-top:14px"><button class="btn ghost" data-act="scanFromMine" style="margin:0">${x("scan", "sm")}&nbsp;Ler QR</button><button class="btn" data-act="shareMyCode" style="margin:0">${x("share", "sm")}&nbsp;Compartilhar</button></div>
    <label class="lbl">C\xF3digo curto</label>
    <p class="note" style="padding:0">Mais f\xE1cil de ditar ou digitar. Vale por 7 dias; depois \xE9 s\xF3 gerar outro.</p>
    <div id="scbox"><button class="btn ghost" data-act="makeShortCode">${x("key", "sm")}&nbsp;Gerar c\xF3digo curto</button></div>`),
      t.shortCode().then((e) => e && jmShowShort(e));
  }
  function jmShowShort(t) {
    let e = y("#scbox");
    if (!e) return;
    let a = Math.max(1, Math.round((t.exp - be()) / Za));
    e.innerHTML = `<div class="sccode selectable">${M(t.code)}</div><p class="note" style="text-align:center;padding:6px 0 0">Vale por mais ${a} ${a === 1 ? "dia" : "dias"}</p>
    <button class="btn ghost" data-act="copyShortCode" data-code="${M(t.code)}">${x("copy", "sm")}&nbsp;Copiar c\xF3digo curto</button>`;
  }
  b("makeShortCode", async (t) => {
    (t.disabled = !0), (t.textContent = "Gerando\u2026");
    try {
      jmShowShort(await h.engine.createShortCode());
    } catch (e) {
      _(e.message), (t.disabled = !1), (t.textContent = "Gerar c\xF3digo curto");
    }
  });
  b("copyShortCode", async (t) => {
    (await X.copy(t.dataset.code)) && _("C\xF3digo copiado");
  });
  b("myCode", () => Ni());
  b("copyMyCode", async () => {
    (await X.copy(h.engine.myNpub())) && _("C\xF3digo copiado");
  });
  b("shareMyCode", () => X.share(S0()));
  b("scanFromMine", () => {
    F(), setTimeout(() => Vu((t) => on(t)), 250);
  });
  function rn(t) {
    let e = h.engine,
      a = Pt(t),
      n = e.contacts.get(t) || {},
      s = e.chatFor("dm:" + t),
      o = [...e.groups.values()].filter((i) => i.meta.members.includes(t));
    let jmEl = J.open(
      "profile",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Perfil</h2></div>
    <div class="scroll">
      <div class="pbanwrap">${n.banner ? `<img class="pbanner" src="${n.banner}" alt="">` : `<div class="pbanner empty"></div>`}</div>
      <div class="cinfo" style="padding:12px 18px 0">${pe(t, 96, { ring: "var(--bg)" })}<h3>${M(a.name)}</h3><p>${M(n.note || "")}</p>
        ${n.nick && n.name && n.nick !== n.name ? `<p style="font-size:13px;color:var(--fg3)">Nome no perfil: ${M(n.name)}</p>` : ""}</div>
      <div class="agrid" style="padding:0 12px">
        <button class="atile" data-act="profMsg" data-pk="${t}"><span class="ab">${x("chat")}</span>Mensagem</button>
        <button class="atile" data-act="profCall" data-pk="${t}"><span class="ab">${x("phone")}</span>Voz</button>
        <button class="atile" data-act="profCall" data-pk="${t}" data-video="1"><span class="ab">${x("video")}</span>V\xEDdeo</button>
        <button class="atile" data-act="profNick" data-pk="${t}"><span class="ab">${x("edit")}</span>Renomear</button></div>
      <div id="jmprofx">${jmProfileExtras(t)}</div>
      <div class="label">Identidade</div>
      <div style="padding:0 18px"><div class="idbox selectable"><span>${M(ao(t))}</span><button class="ibtn sm" data-act="copyPk" data-pk="${t}" aria-label="Copiar c\xF3digo">${x("copy", "sm")}</button></div></div>
      <div class="srow" role="button" tabindex="0" data-act="safety" data-pk="${t}"><span class="sic">${x("shield")}</span><span class="sm"><b>C\xF3digo de seguran\xE7a</b><small>${n.verified ? "Verificado por voc\xEA" : "Compare com a outra pessoa para confirmar que \xE9 ela"}</small></span><span class="chev">${x("chev", "sm")}</span></div>
      <div class="label">Conversa</div>
      <div class="srow" role="button" tabindex="0" data-act="profTtl" data-pk="${t}"><span class="sic">${x("timer")}</span><span class="sm"><b>Mensagens tempor\xE1rias</b><small>${M(yt(s.ttl))}</small></span><span class="chev">${x("chev", "sm")}</span></div>
      ${o.length ? `<div class="label">Grupos em comum</div>${o.map((i) => `<div class="row" role="button" tabindex="0" data-act="openGroup" data-lid="${i.lid}">${Pe(i, 44)}<div class="rmain"><b class="rname">${M(i.meta.name)}</b></div></div>`).join("")}` : ""}
      <div class="label">Privacidade</div>
      <div class="srow danger" role="button" tabindex="0" data-act="profBlock" data-pk="${t}"><span class="sic">${x("ban")}</span><span class="sm"><b>${n.status === "blocked" ? "Desbloquear" : "Bloquear"}</b><small>${n.status === "blocked" ? "Voltar a receber mensagens" : "Parar de receber mensagens e chamadas"}</small></span></div>
      <div class="srow danger" role="button" tabindex="0" data-act="profDelete" data-pk="${t}"><span class="sic">${x("trash")}</span><span class="sm"><b>Apagar conversa</b><small>Remove as mensagens deste aparelho</small></span></div>
      <div style="height:40px"></div></div></div>`,
    );
    return jmEl && jmEl.dataset && (jmEl.dataset.pk = t), jmEl;
  }
  /** Cartão "Ouvindo Spotify" e jogos favoritos de alguém. */
  function jmProfileExtras(t) {
    let e = h.engine,
      a = t === e.pk ? e.me : e.contacts.get(t) || {},
      n = t === e.pk ? e.myListening && e.myListening.l : e.listeningOf(t),
      s = "";
    if (n) {
      let o = Math.max(0, (Date.now() - (n.st || Date.now())) / 1e3),
        r = (c) => Math.floor(c / 60) + ":" + String(Math.floor(c % 60)).padStart(2, "0");
      s += `<div class="lcard"><div class="lhd">${x("music", "xs")} OUVINDO SPOTIFY</div>
      <div class="lbody"><span class="lcover">${x("music")}</span><div style="min-width:0"><b>${M(n.s)}</b>${n.a ? `<small>de ${M(n.a)}</small>` : ""}${n.al ? `<small>em ${M(n.al)}</small>` : ""}</div></div>
      ${n.d && n.st ? `<div class="lbar"><i style="width:${Math.min(100, (o / n.d) * 100).toFixed(1)}%"></i></div><div class="ltimes"><span>${r(Math.min(o, n.d))}</span><span>${r(n.d)}</span></div>` : n.st ? `<div class="ltimes"><span>Tocando h\xE1 ${r(o)}</span></div>` : ""}
      <button class="lbtn" data-act="openSpotifyTrack" data-url="${M(jmListeningUrl(n))}">Ouvir no Spotify</button></div>`;
    }
    let o = a.games || [];
    return (
      o.length &&
        (s += `<div class="label">Jogos favoritos</div><div class="games">${o.map((r) => `<span class="chip">${x("game", "xs")}${M(r)}</span>`).join("")}</div>`),
      s
    );
  }
  function jmRefreshProfile(t) {
    let e = y("#jmprofx");
    e && (e.innerHTML = jmProfileExtras(t));
  }
  b("openSpotifyTrack", (t) => X.openUrl(t.dataset.url));
  b("openProfile", (t) => rn(t.dataset.pk));
  b("profMsg", (t) => {
    J.pop(), J.find("chat") || P.openChat("dm:" + t.dataset.pk);
  });
  b("profCall", (t) => P.startCall(t.dataset.pk, !!t.dataset.video));
  b("copyPk", async (t) => {
    (await X.copy(ao(t.dataset.pk))) && _("C\xF3digo copiado");
  });
  b("profTtl", (t) => P.ttlSheet("dm:" + t.dataset.pk));
  b("profNick", (t) => {
    let e = t.dataset.pk,
      a = h.engine.contacts.get(e) || {};
    ee(`<h2 class="sh-title">Renomear contato</h2><p class="sh-sub">S\xF3 voc\xEA v\xEA esse nome.</p>
    <input class="inp" id="nickin" maxlength="40" value="${M(a.nick || a.name || "")}" placeholder="Nome" style="margin-top:14px">
    <button class="btn" data-act="saveNick" data-pk="${e}">Salvar</button>`),
      setTimeout(() => {
        let n = y("#nickin");
        n && n.focus();
      }, 350);
  });
  b("saveNick", async (t) => {
    await h.engine.updateContact(t.dataset.pk, {
      nick: y("#nickin").value.trim(),
    }),
      F(),
      J.pop(!0),
      rn(t.dataset.pk),
      P.refreshLists(),
      P.refreshChatHead();
  });
  b("safety", (t) => {
    let e = t.dataset.pk,
      a = h.engine,
      n = a.contacts.get(e) || {},
      s = kd(a.pk, e);
    ee(`<h2 class="sh-title">C\xF3digo de seguran\xE7a</h2><p class="sh-sub">Os dois aparelhos mostram estes mesmos 60 n\xFAmeros. Compare pessoalmente ou por uma liga\xE7\xE3o. Se forem iguais, ningu\xE9m est\xE1 no meio da conversa.</p>
    <div class="safety selectable">${s.map((o) => `<span>${o}</span>`).join("")}</div>
    <button class="btn ${n.verified ? "ghost" : ""}" data-act="toggleVerify" data-pk="${e}">${n.verified ? "Remover verifica\xE7\xE3o" : "Marcar como verificado"}</button>`);
  });
  b("toggleVerify", async (t) => {
    let e = t.dataset.pk,
      a = h.engine.contacts.get(e);
    await h.engine.updateContact(e, { verified: !(a && a.verified) }),
      F(),
      J.pop(!0),
      rn(e),
      _(a && a.verified ? "Verifica\xE7\xE3o removida" : "Contato verificado");
  });
  b("profBlock", async (t) => {
    let e = t.dataset.pk;
    if ((h.engine.contacts.get(e) || {}).status === "blocked")
      await h.engine.setContactStatus(e, "ok"), _("Contato desbloqueado");
    else {
      if (
        !(await ge({
          title: "Bloquear " + ne(e) + "?",
          text: "Voc\xEA n\xE3o vai receber mensagens nem chamadas. A pessoa n\xE3o \xE9 avisada.",
          ok: "Bloquear",
          danger: !0,
        }))
      )
        return;
      await h.engine.setContactStatus(e, "blocked"), _("Contato bloqueado");
    }
    J.pop(!0), rn(e), P.refreshLists();
  });
  b("profDelete", async (t) => {
    (await ge({
      title: "Apagar conversa?",
      text: "As mensagens somem deste aparelho.",
      ok: "Apagar",
      danger: !0,
    })) &&
      (await h.engine.deleteChat("dm:" + t.dataset.pk),
      J.clear(),
      P.refreshLists(),
      _("Conversa apagada"));
  });
  function zu() {
    let t = h.engine,
      e = t
        .chatList()
        .filter(
          (a) =>
            a.kind === "dm" &&
            (t.contacts.get(a.pk) || {}).status === "request" &&
            a.lastText,
        );
    J.open(
      "requests",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Pedidos de mensagem</h2></div>
    <div class="scroll"><p class="note" style="padding-bottom:6px">Pessoas que ainda n\xE3o est\xE3o nos seus contatos. Elas n\xE3o sabem se voc\xEA leu at\xE9 voc\xEA aceitar.</p>
    ${e.length ? e.map((a) => `<div class="row" role="button" tabindex="0" data-act="openReq" data-key="${a.key}">${pe(a.pk, 52)}<div class="rmain"><div class="rtop"><b class="rname">${M(ne(a.pk))}</b><span class="rtime">${M(Xa(a.lastTs))}</span></div><div class="rbot"><span class="rprev"><span class="t">${M(a.lastText)}</span></span></div></div></div>`).join("") : '<div class="empty">Nenhum pedido no momento.</div>'}</div></div>`,
    );
  }
  b("openReq", (t) => P.openChat(t.dataset.key));
  function _i() {
    let e = [...h.engine.contacts.values()].filter(
      (a) => a.status === "blocked",
    );
    J.open(
      "blocked",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Bloqueados</h2></div>
    <div class="scroll">${e.length ? e.map((a) => `<div class="mrow2">${pe(a.pk, 44)}<div><div class="nmx">${M(ne(a.pk))}</div><small>${M(ao(a.pk).slice(0, 18))}\u2026</small></div><button class="joinbtn" data-act="unblock" data-pk="${a.pk}">Desbloquear</button></div>`).join("") : '<div class="empty">Ningu\xE9m bloqueado.</div>'}</div></div>`,
    );
  }
  b("unblock", async (t) => {
    await h.engine.setContactStatus(t.dataset.pk, "ok"),
      J.pop(!0),
      _i(),
      _("Desbloqueado");
  });
  var se = null,
    cn = (t, e) => t.meta.owner === e || (t.meta.admins || []).includes(e);
  function Oi(t, e) {
    let a = h.engine,
      n = a.groups.get(t);
    if (!n) {
      _("Grupo n\xE3o encontrado");
      return;
    }
    for (; J.find("group"); ) J.pop(!0);
    (se = { lid: t, tab: e || "canais", shut: {} }),
      J.open("group", Gu(n), {
        onClose: () => {
          (se = null), P.refreshLists();
        },
      }),
      qt(y("#gbody"));
  }
  function Gu(t) {
    let e = h.engine,
      a = t.meta.members.length,
      n = t.meta.channels
        .filter((o) => o.type === "voice")
        .reduce((o, r) => o + e.roomMembers(t.lid, r.id).length, 0),
      s = cn(t, e.pk);
    return `<div class="gtop"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><div class="grow"></div>
      <button class="ibtn" data-act="groupMenu" aria-label="Op\xE7\xF5es do grupo">${x("more")}</button></div>
    <div class="scroll" id="gscroll">
      <div class="hero"><div class="big"><span class="pfwrap" role="button" tabindex="0" data-act="groupPhoto" aria-label="${s ? "Mudar foto do grupo" : "Ver foto do grupo"}">${Pe(t, 76)}${s ? `<i class="pfcam">${x("camera")}</i>` : ""}</span><div style="min-width:0"><h2>${M(t.meta.name)}</h2>
        <div class="stats"><span>${a} ${a === 1 ? "membro" : "membros"}</span>${n ? `<span style="color:var(--accent-t)">${n} na sala de voz</span>` : ""}</div></div></div>
        ${t.meta.desc ? `<p class="desc">${M(t.meta.desc)}</p>` : ""}
        <div class="pills">
          <button class="pbt" data-act="groupInvite">${x("userplus", "sm")}Convidar</button>
          <button class="pbt ${t.mute ? "on" : ""}" data-act="groupMute">${x(t.mute ? "belloff" : "bell", "sm")}${t.mute ? "Silenciado" : "Silenciar"}</button>
          ${s || (t.meta.roles || []).length ? `<button class="pbt" data-act="rolesOpen">${x("sparkle", "sm")}Cargos</button>` : ""}
        </div></div>
      <div class="seg ${se.tab === "membros" ? "m1" : ""}" role="tablist"><i class="ind"></i><button class="${se.tab === "canais" ? "on" : ""}" data-act="gtab" data-t="canais">Canais</button><button class="${se.tab === "membros" ? "on" : ""}" data-act="gtab" data-t="membros">Membros</button></div>
      <div id="gbody">${se.tab === "canais" ? L0(t) : T0(t)}</div><div style="height:28px"></div></div>`;
  }
  function L0(t) {
    let e = h.engine,
      a = cn(t, e.pk);
    return (
      [
        ["text", "Canais de texto"],
        ["voice", "Canais de voz"],
      ]
        .map(([s, o], r) => {
          let i = se.shut[s],
            c = t.meta.channels.filter((u) => u.type === s),
            l = i
              ? ""
              : c
                  .map((u) => {
                    if (s === "text") {
                      let v = e.chatFor(`g:${t.lid}:${u.id}`),
                        k = v.lastText
                          ? (v.lastKind === "sys"
                              ? ""
                              : (v.lastFrom === e.pk
                                  ? "Voc\xEA"
                                  : Ue(v.lastFrom)) + ": ") + v.lastText
                          : "Nenhuma mensagem ainda";
                      return `<div class="chan ${v.unread ? "unr" : ""}" role="button" tabindex="0" data-act="openChannel" data-key="${v.key}" ${a ? `data-long="chanMenu" data-ch="${u.id}"` : ""}>
          <span class="cico">${x("hash", "sm")}</span><div class="cmain"><span class="cname">${M(u.name)}</span><span class="cprev">${M(k)}</span></div>
          ${v.unread ? `<span class="rbadge">${v.unread}</span>` : ""}</div>`;
                    }
                    let d = e.roomMembers(t.lid, u.id),
                      f = P.inRoom && P.inRoom(t.lid, u.id),
                      p = f ? [e.pk, ...d.filter((v) => v !== e.pk)] : d;
                    return `<div class="chan ${p.length ? "live" : ""}" role="button" tabindex="0" data-act="joinRoom" data-lid="${t.lid}" data-ch="${u.id}" ${a ? 'data-long="chanMenu"' : ""}>
        <span class="cico">${x("volume", "sm")}</span><div class="cmain"><span class="cname">${M(u.name)}</span>
        ${
          p.length
            ? `<span class="stack"><span class="avs">${p
                .slice(0, 5)
                .map((v) => pe(v, 22, { badge: !1 }))
                .join("")}</span>${p.length} na sala</span>`
            : '<span class="cprev">Vazia \xB7 toque para entrar</span>'
        }</div>
        <span class="joinhint">${f ? "Voc\xEA est\xE1 aqui" : "Entrar"}</span></div>`;
                  })
                  .join("");
          return `<button class="cat ${i ? "shut" : ""}" data-act="catToggle" data-type="${s}" aria-expanded="${!i}">${x("chevd")}${o}</button>${l}`;
        })
        .join("") +
      (a
        ? `<button class="chan" data-act="addChannel"><span class="cico">${x("plus", "sm")}</span><div class="cmain"><span class="cname" style="color:var(--fg2)">Criar canal</span></div></button>
    <button class="chan" data-act="manageChannels"><span class="cico">${x("edit", "sm")}</span><div class="cmain"><span class="cname" style="color:var(--fg2)">Editar canais</span><span class="cprev">Renomear, reordenar ou apagar</span></div></button>`
        : "")
    );
  }
  function T0(t) {
    let e = h.engine,
      a = (s) =>
        s === t.meta.owner ? 0 : (t.meta.admins || []).includes(s) ? 1 : 2,
      jr = (s) => {
        let o = jmRolesOf(t, s)[0];
        return o ? (t.meta.roles || []).indexOf(o) : 99;
      },
      n = [...t.meta.members].sort(
        (s, o) => a(s) - a(o) || jr(s) - jr(o) || ne(s).localeCompare(ne(o)),
      );
    return (
      `<div class="label">${n.length} ${n.length === 1 ? "membro" : "membros"}</div>` +
      n
        .map((s) => {
          let o =
              s === t.meta.owner
                ? "Dono"
                : (t.meta.admins || []).includes(s)
                  ? "Admin"
                  : "",
            r = e.contacts.get(s);
          let rr = jmRolesOf(t, s);
          return `<div class="mrow2" role="button" tabindex="0" data-act="memberMenu" data-pk="${s}" style="cursor:pointer">${pe(s, 44)}<div><div class="nmx"><span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;${rr[0] ? `color:${JM_ROLE_CORES[rr[0].c]}` : ""}">${M(ne(s))}</span>${o ? `<span class="rolepill">${o}</span>` : ""}${rr.map((q) => `<span class="rolepill cust" style="--rc:${JM_ROLE_CORES[q.c]}">${M(q.n)}</span>`).join("")}</div>
      <small>${s === e.pk ? "Voc\xEA" : r && r.status === "ok" ? "Nos seus contatos" : "Membro do grupo"}</small></div></div>`;
        })
        .join("") +
      '<p class="note">S\xF3 quem tem a chave do grupo l\xEA as mensagens. Quando o dono remove algu\xE9m, o grupo troca de chave.</p>'
    );
  }
  function Be() {
    if ((Xn(), !se)) return;
    let t = h.engine.groups.get(se.lid),
      e = J.find("group");
    if (!t || !e) return;
    let a = y("#gscroll", e.el),
      n = a ? a.scrollTop : 0;
    e.el.innerHTML = Gu(t);
    let s = y("#gscroll", e.el);
    s && (s.scrollTop = n);
  }
  b("gtab", (t) => {
    se.tab !== t.dataset.t && ((se.tab = t.dataset.t), Be(), qt(y("#gbody")));
  });
  b("catToggle", (t) => {
    (se.shut[t.dataset.type] = !se.shut[t.dataset.type]), Be();
  });
  b("openChannel", (t) => P.openChat(t.dataset.key));
  b("groupMute", async () => {
    let t = h.engine.groups.get(se.lid);
    t.mute = !t.mute;
    for (let e of h.engine.groupChats(t.lid))
      (e.muted = t.mute), await h.engine.saveChat(e);
    await h.engine.saveGroup(t),
      Be(),
      _(t.mute ? "Grupo silenciado" : "Notifica\xE7\xF5es ativadas");
  });
  var B0 = {
      amigos: { n: "Amigos", d: "Conversa e fotos" },
      estudos: { n: "Estudos", d: "Avisos e d\xFAvidas" },
      jogos: { n: "Jogos", d: "Clipes e partidas" },
      branco: { n: "Em branco", d: "S\xF3 o essencial" },
    },
    ko = "amigos";
  function ji() {
    (ko = "amigos"),
      ee(`<h2 class="sh-title">Novo grupo</h2><p class="sh-sub">Grupos t\xEAm canais de texto e salas de voz. A chave fica s\xF3 com quem voc\xEA convidar.</p>
    <label class="lbl" for="gname">Nome do grupo</label>
    <input class="inp" id="gname" maxlength="40" placeholder="Ex.: Pelada de quinta" autocomplete="off">
    <div class="errtx" id="gerr" hidden>D\xEA um nome ao grupo para continuar.</div>
    <span class="lbl">Modelo de canais</span>
    <div class="tpl">${Object.entries(B0)
      .map(
        ([t, e]) =>
          `<button class="tplb ${t === ko ? "on" : ""}" data-act="tpl" data-t="${t}">${e.n}<small>${e.d}</small></button>`,
      )
      .join("")}</div>
    <button class="btn" data-act="createGroup">Criar grupo</button>`),
      setTimeout(() => {
        let t = y("#gname");
        t && t.focus({ preventScroll: !0 });
      }, 420);
  }
  b("newGroup", () => ji());
  b("tpl", (t) => {
    (ko = t.dataset.t),
      oe(".tplb").forEach((e) => e.classList.toggle("on", e === t));
  });
  b("createGroup", async () => {
    let t = y("#gname"),
      e = t.value.trim();
    if (!e) {
      t.classList.add("err"), (y("#gerr").hidden = !1), t.focus();
      return;
    }
    let a = await h.engine.createGroup(e, ko);
    F(),
      P.refreshLists(),
      P.setTab("groups"),
      Oi(a.lid),
      _("Grupo criado. Agora convide pessoas."),
      setTimeout(() => qi(a.lid), 700);
  });
  document.addEventListener("input", (t) => {
    if (t.target.id === "gname") {
      t.target.classList.remove("err");
      let e = y("#gerr");
      e && (e.hidden = !0);
    }
  });
  function qi(t) {
    let e = h.engine,
      a = e.groups.get(t);
    if (!a) return;
    let n = [...e.contacts.values()]
      .filter((s) => s.status === "ok" && !a.meta.members.includes(s.pk))
      .sort((s, o) => ne(s.pk).localeCompare(ne(o.pk)));
    J.open(
      "invite",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Convidar para ${M(a.meta.name)}</h2></div>
    <div class="scroll" id="invlist">${n.length ? n.map((s) => `<label class="pick">${pe(s.pk, 44)}<span style="flex:1;min-width:0;font-weight:600">${M(ne(s.pk))}</span><input type="checkbox" value="${s.pk}"></label>`).join("") : '<div class="estate"><h3>Nenhum contato para convidar</h3><p>Adicione pessoas pelo c\xF3digo primeiro. Depois volte aqui.</p><button class="btn" data-act="addContact">Adicionar contato</button></div>'}</div>
    ${n.length ? `<div style="padding:10px 16px calc(14px + env(safe-area-inset-bottom,0px))"><button class="btn" data-act="sendInvites" data-lid="${t}" style="margin:0">Enviar convites</button></div>` : ""}</div>`,
    );
  }
  b("groupInvite", () => qi(se.lid));
  b("sendInvites", async (t) => {
    let e = oe("#invlist input:checked").map((n) => n.value);
    if (!e.length) return _("Escolha pelo menos uma pessoa");
    let a = t;
    a.classList.add("busy"), (a.textContent = "Enviando\u2026");
    for (let n of e) await h.engine.inviteToGroup(t.dataset.lid, n);
    J.pop(),
      _(e.length === 1 ? "Convite enviado" : `${e.length} convites enviados`);
  });
  b("memberMenu", (t) => {
    let e = h.engine,
      a = e.groups.get(se.lid),
      n = t.dataset.pk;
    if (n === e.pk) return;
    let s = a.meta.owner === e.pk,
      o = e.contacts.get(n);
    ee(`<div class="cinfo">${pe(n, 72)}<h3>${M(ne(n))}</h3></div><div class="mlist">
    <button class="mitem" data-act="memberMsg" data-pk="${n}">${x("chat")}${o && o.status === "ok" ? "Mandar mensagem" : "Adicionar e mandar mensagem"}</button>
    ${cn(a, e.pk) ? `<button class="mitem" data-act="memberRoles" data-pk="${n}">${x("sparkle")}<span>Cargos<small>${M(jmRolesOf(a, n).map((q) => q.n).join(", ") || "Nenhum cargo")}</small></span></button>` : ""}
    ${
      s
        ? `<button class="mitem" data-act="memberAdmin" data-pk="${n}">${x("shield")}${(a.meta.admins || []).includes(n) ? "Remover admin" : "Tornar admin"}</button>
    <button class="mitem danger" data-act="memberRemove" data-pk="${n}">${x("userx")}<span>Remover do grupo<small>O grupo troca de chave e essa pessoa n\xE3o l\xEA mais nada novo</small></span></button>`
        : ""
    }</div>`);
  });
  b("memberMsg", async (t) => {
    F();
    let e = h.engine,
      a = t.dataset.pk;
    (!(e.contacts.get(a) || {}).status || e.contacts.get(a).status !== "ok") &&
      (await e.addContact(a)),
      P.openChat("dm:" + a, { focus: !0 });
  });
  b("memberAdmin", async (t) => {
    F();
    let e = h.engine.groups.get(se.lid),
      a = t.dataset.pk;
    await h.engine.editGroup(e.lid, {
      admin: { pk: a, on: !(e.meta.admins || []).includes(a) },
    }),
      Be();
  });
  b("memberRemove", async (t) => {
    F();
    let e = t.dataset.pk;
    (await ge({
      title: "Remover " + ne(e) + "?",
      text: "O grupo ganha uma chave nova, entregue s\xF3 para quem fica.",
      ok: "Remover",
      danger: !0,
    })) &&
      (await h.engine.removeMember(se.lid, e),
      Be(),
      _("Removido. Chave do grupo trocada."));
  });
  b("groupMenu", () => {
    let t = h.engine,
      e = t.groups.get(se.lid),
      a = cn(e, t.pk);
    ee(`<div class="mlist">
    ${
      a
        ? `<button class="mitem" data-act="editGroup">${x("edit")}Editar nome e descri\xE7\xE3o</button>
    <button class="mitem" data-act="groupPhotoM">${x("camera")}${e.pic ? "Mudar foto do grupo" : "Colocar foto no grupo"}</button>
    <button class="mitem" data-act="manageChannelsM">${x("hash")}Editar canais</button>
    <button class="mitem" data-act="addChannel">${x("plus")}Criar canal</button>`
        : ""
    }
    <button class="mitem" data-act="groupInviteS">${x("userplus")}Convidar pessoas</button>
    <button class="mitem danger" data-act="leaveGroup">${x("logout")}Sair do grupo</button></div>`);
  });
  b("groupInviteS", () => {
    F(), qi(se.lid);
  });
  b("editGroup", () => {
    let t = h.engine.groups.get(se.lid);
    ee(`<h2 class="sh-title">Editar grupo</h2>
    <label class="lbl" for="egname">Nome</label><input class="inp" id="egname" maxlength="40" value="${M(t.meta.name)}">
    <label class="lbl" for="egdesc">Descri\xE7\xE3o</label><textarea class="inp" id="egdesc" maxlength="140" rows="3">${M(t.meta.desc || "")}</textarea>
    <button class="btn" data-act="saveGroup">Salvar</button>`);
  });
  b("saveGroup", async () => {
    await h.engine.editGroup(se.lid, {
      name: y("#egname").value.trim(),
      desc: y("#egdesc").value.trim(),
    }),
      F(),
      Be(),
      P.refreshLists(),
      _("Grupo atualizado");
  });
  var Pi = "text";
  b("addChannel", () => {
    (Pi = "text"),
      ee(`<h2 class="sh-title">Criar canal</h2>
    <div class="tpl" style="margin-top:14px"><button class="tplb on" data-act="chType" data-t="text">${x("hash", "sm")} Texto<small>Mensagens, fotos e \xE1udios</small></button><button class="tplb" data-act="chType" data-t="voice">${x("volume", "sm")} Voz<small>Sala para conversar ao vivo</small></button></div>
    <label class="lbl" for="chname">Nome do canal</label><input class="inp" id="chname" maxlength="24" placeholder="Ex.: avisos">
    <button class="btn" data-act="saveChannel">Criar canal</button>`);
  });
  b("chType", (t) => {
    (Pi = t.dataset.t),
      oe(".tplb").forEach((e) => e.classList.toggle("on", e === t));
  });
  b("saveChannel", async () => {
    let t = y("#chname").value.trim();
    if (!t) {
      y("#chname").classList.add("err");
      return;
    }
    await h.engine.editGroup(se.lid, { addChannel: { name: t, type: Pi } }),
      F(),
      (se.tab = "canais"),
      Be(),
      _("Canal criado");
  });
  b("leaveGroup", async () => {
    F();
    let t = h.engine.groups.get(se.lid);
    (await ge({
      title: "Sair de " + t.meta.name + "?",
      text: "As conversas do grupo ser\xE3o apagadas deste aparelho.",
      ok: "Sair do grupo",
      danger: !0,
    })) &&
      (P.inRoom && P.inRoomAny && P.inRoomAny(t.lid) && P.leaveRoom(),
      await h.engine.leaveGroup(t.lid),
      J.clear(),
      P.refreshLists(),
      _("Voc\xEA saiu do grupo"));
  });
  async function Ku(t) {
    let e = h.engine,
      a = e.groups.get(t);
    if (!a || !cn(a, e.pk)) return;
    let n = await ho({
      title: "Foto do grupo",
      hasPic: !!a.pic,
      shape: "sq",
      facing: "environment",
    });
    if (n !== null) {
      if (!(await e.setGroupPic(t, n)))
        return _("N\xE3o foi poss\xEDvel mudar a foto.");
      Be(),
        P.refreshLists(),
        _(n ? "Foto do grupo atualizada" : "Foto do grupo removida");
    }
  }
  b("groupPhoto", () => {
    let t = h.engine,
      e = se && t.groups.get(se.lid);
    if (!e) return;
    if (cn(e, t.pk)) return Ku(e.lid);
    if (!e.pic) return;
    let a = y("#viewer");
    (a.innerHTML = `<div class="vtop"><button class="ibtn" data-act="closeViewer" aria-label="Fechar">${x("back")}</button></div><img src="${e.pic}" alt="Foto do grupo" class="pixel">`),
      (a.hidden = !1);
  });
  b("groupPhotoM", () => {
    F(), setTimeout(() => Ku(se.lid), 260);
  });
  var Jn = null;
  function Fu(t) {
    (Jn = t),
      J.open(
        "channels",
        `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Editar canais</h2></div><div class="scroll" id="cmbody"></div></div>`,
        {
          onClose: () => {
            Jn = null;
          },
        },
      ),
      Xn();
  }
  function Xn() {
    let t = y("#cmbody");
    if (!t || !Jn) return;
    let e = h.engine.groups.get(Jn);
    if (!e) {
      J.pop();
      return;
    }
    let a = e.meta.channels.filter((n) => n.type === "text").length;
    t.innerHTML =
      [
        ["text", "Canais de texto"],
        ["voice", "Canais de voz"],
      ]
        .map(([n, s]) => {
          let o = e.meta.channels.filter((r) => r.type === n);
          return (
            `<div class="label">${s}</div>` +
            (o.length
              ? o
                  .map(
                    (r, i) => `<div class="cmrow">
      <span class="cico">${x(n === "text" ? "hash" : "volume", "sm")}</span>
      <div class="cmain" role="button" tabindex="0" data-act="chEdit" data-ch="${r.id}"><span class="cname">${M(r.name)}</span><span class="cprev">${r.topic ? M(r.topic) : "Sem descri\xE7\xE3o"}</span></div>
      <button class="ibtn sm" data-act="chMove" data-ch="${r.id}" data-dir="-1" ${i === 0 ? "disabled" : ""} aria-label="Subir ${M(r.name)}">${x("up", "sm")}</button>
      <button class="ibtn sm" data-act="chMove" data-ch="${r.id}" data-dir="1" ${i === o.length - 1 ? "disabled" : ""} aria-label="Descer ${M(r.name)}">${x("down", "sm")}</button>
      <button class="ibtn sm" data-act="chEdit" data-ch="${r.id}" aria-label="Editar ${M(r.name)}">${x("edit", "sm")}</button>
      <button class="ibtn sm" data-act="chDel" data-ch="${r.id}" ${n === "text" && a === 1 ? "disabled" : ""} aria-label="Apagar ${M(r.name)}">${x("trash", "sm")}</button></div>`,
                  )
                  .join("")
              : `<p class="note" style="padding:4px 18px">Nenhum canal de ${n === "text" ? "texto" : "voz"}.</p>`)
          );
        })
        .join("") +
      `<div style="padding:18px 16px 0"><button class="btn ghost" data-act="addChannel" style="margin:0">${x("plus", "sm")}&nbsp;Criar canal</button></div>
    <p class="note" style="padding:14px 18px 30px">As mudan\xE7as valem para todos do grupo. Apagar um canal de texto apaga as mensagens dele em todos os aparelhos. O grupo precisa de pelo menos um canal de texto.</p>`;
  }
  b("manageChannels", () => Fu(se.lid));
  b("manageChannelsM", () => {
    F(), Fu(se.lid);
  });
  var Vi = () => Jn || (se && se.lid);
  b("chMove", async (t) => {
    await h.engine.editGroup(Vi(), {
      moveChannel: { id: t.dataset.ch, dir: +t.dataset.dir },
    }),
      Xn(),
      Be();
  });
  b("chEdit", (t) => Wu(Vi(), t.dataset.ch));
  b("chDel", (t) => Yu(Vi(), t.dataset.ch));
  b("chanMenu", (t) => {
    let e = se && h.engine.groups.get(se.lid),
      a = e && e.meta.channels.find((s) => s.id === t.dataset.ch);
    if (!a || !cn(e, h.engine.pk)) return;
    let n = e.meta.channels.filter((s) => s.type === "text").length;
    ee(`<h2 class="sh-title">${a.type === "text" ? "#" : ""}${M(a.name)}</h2><div class="mlist">
    <button class="mitem" data-act="chEditS" data-ch="${a.id}">${x("edit")}Editar canal</button>
    <button class="mitem" data-act="chMoveS" data-ch="${a.id}" data-dir="-1">${x("up")}Mover para cima</button>
    <button class="mitem" data-act="chMoveS" data-ch="${a.id}" data-dir="1">${x("down")}Mover para baixo</button>
    ${a.type === "text" && n === 1 ? "" : `<button class="mitem danger" data-act="chDelS" data-ch="${a.id}">${x("trash")}Apagar canal</button>`}</div>`);
  });
  b("chEditS", (t) => Wu(se.lid, t.dataset.ch));
  b("chMoveS", async (t) => {
    F(),
      await h.engine.editGroup(se.lid, {
        moveChannel: { id: t.dataset.ch, dir: +t.dataset.dir },
      }),
      Be();
  });
  b("chDelS", (t) => {
    F(), setTimeout(() => Yu(se.lid, t.dataset.ch), 260);
  });
  function Wu(t, e) {
    let a = h.engine.groups.get(t),
      n = a && a.meta.channels.find((s) => s.id === e);
    n &&
      ee(`<h2 class="sh-title">Editar canal</h2>
    <label class="lbl" for="ecname">Nome</label><input class="inp" id="ecname" maxlength="24" value="${M(n.name)}">
    <div class="errtx" id="ecerr" hidden>O canal precisa de um nome.</div>
    <label class="lbl" for="ectopic">Descri\xE7\xE3o (opcional)</label><textarea class="inp" id="ectopic" maxlength="80" rows="2" placeholder="Sobre o que \xE9 este canal">${M(n.topic || "")}</textarea>
    <button class="btn" data-act="chSave" data-lid="${t}" data-ch="${n.id}">Salvar</button>`);
  }
  b("chSave", async (t) => {
    let e = y("#ecname").value.trim();
    if (!e) {
      y("#ecname").classList.add("err"), (y("#ecerr").hidden = !1);
      return;
    }
    await h.engine.editGroup(t.dataset.lid, {
      editChannel: {
        id: t.dataset.ch,
        name: e,
        topic: y("#ectopic").value.trim(),
      },
    }),
      F(),
      Xn(),
      Be(),
      P.refreshChatHead && P.refreshChatHead(),
      _("Canal atualizado");
  });
  async function Yu(t, e) {
    let a = h.engine,
      n = a.groups.get(t),
      s = n && n.meta.channels.find((r) => r.id === e);
    if (!s) return;
    if (
      s.type === "text" &&
      n.meta.channels.filter((r) => r.type === "text").length === 1
    )
      return _("O grupo precisa de pelo menos um canal de texto.");
    let o =
      s.type === "text"
        ? "As mensagens deste canal somem para todos do grupo."
        : "Quem estiver na sala de voz sai dela.";
    (await ge({
      title: `Apagar ${s.type === "text" ? "#" : ""}${s.name}?`,
      text: o,
      ok: "Apagar canal",
      danger: !0,
    })) &&
      (s.type === "voice" && P.inRoom && P.inRoom(t, e) && P.leaveRoom(),
      await a.editGroup(t, { removeChannel: e }),
      Xn(),
      Be(),
      P.refreshLists(),
      _("Canal apagado"));
  }
  var R0 = [
      {
        urls: ["stun:stun.l.google.com:19302", "stun:stun.cloudflare.com:3478"],
      },
    ],
    Eo = 6,
    H = null,
    z = null,
    Ju = new Set();
  function I0() {
    let t = h.engine.settings,
      e = [...R0];
    return (
      t.turn &&
        t.turn.url &&
        e.push({
          urls: t.turn.url.split(/[\s,]+/).filter(Boolean),
          username: t.turn.user || void 0,
          credential: t.turn.pass || void 0,
        }),
      e
    );
  }
  async function zi(t, e = "user") {
    return navigator.mediaDevices.getUserMedia({
      audio: $i,
      video: t
        ? {
            facingMode: e,
            width: { ideal: 640 },
            height: { ideal: 480 },
            frameRate: { ideal: 24, max: 30 },
          }
        : !1,
    });
  }
  var es = class {
      constructor(e, a) {
        (this.pk = e), (this.h = a);
        let n = h.engine.settings;
        (this.pc = new RTCPeerConnection({
          iceServers: I0(),
          iceTransportPolicy:
            n.forceRelay && n.turn && n.turn.url ? "relay" : "all",
          bundlePolicy: "max-bundle",
        })),
          (this.dc = this.pc.createDataChannel("jm", {
            negotiated: !0,
            id: 0,
          })),
          (this.dc.onmessage = (s) => {
            try {
              a.onData(JSON.parse(s.data));
            } catch {}
          }),
          (this.dc.onopen = () => a.onOpen && a.onOpen()),
          (this.remote = new MediaStream()),
          (this.pc.ontrack = (s) => {
            this.remote.getTracks().includes(s.track) ||
              this.remote.addTrack(s.track),
              a.onRemote(this.remote, s.track);
          }),
          (this.pc.onconnectionstatechange = () =>
            a.onState(this.pc.connectionState));
      }
      async gather() {
        this.pc.iceGatheringState !== "complete" &&
          (await new Promise((e) => {
            let a = setTimeout(e, 2600);
            this.pc.addEventListener("icegatheringstatechange", () => {
              this.pc.iceGatheringState === "complete" &&
                (clearTimeout(a), e());
            });
          }));
      }
      async makeOffer(e) {
        let a = e.getAudioTracks()[0],
          n = e.getVideoTracks()[0];
        return (
          (this.aT = this.pc.addTransceiver(a || "audio", {
            direction: "sendrecv",
            streams: [e],
          })),
          (this.vT = this.pc.addTransceiver(n || "video", {
            direction: "sendrecv",
            streams: [e],
          })),
          await this.pc.setLocalDescription(await this.pc.createOffer()),
          await this.gather(),
          this.pc.localDescription.sdp
        );
      }
      async acceptOffer(e, a) {
        await this.pc.setRemoteDescription({ type: "offer", sdp: e });
        let n = this.pc.getTransceivers();
        (this.aT = n.find(
          (r) => r.receiver.track && r.receiver.track.kind === "audio",
        )),
          (this.vT = n.find(
            (r) => r.receiver.track && r.receiver.track.kind === "video",
          ));
        let s = a.getAudioTracks()[0],
          o = a.getVideoTracks()[0];
        if (this.aT) {
          (this.aT.direction = "sendrecv"),
            await this.aT.sender.replaceTrack(s || null);
          try {
            this.aT.sender.setStreams(a);
          } catch {}
        }
        if (this.vT) {
          (this.vT.direction = "sendrecv"),
            await this.vT.sender.replaceTrack(o || null);
          try {
            this.vT.sender.setStreams(a);
          } catch {}
        }
        return (
          await this.pc.setLocalDescription(await this.pc.createAnswer()),
          await this.gather(),
          this.pc.localDescription.sdp
        );
      }
      async setAnswer(e) {
        this.pc.signalingState === "have-local-offer" &&
          (await this.pc.setRemoteDescription({ type: "answer", sdp: e }));
      }
      async setVideo(e) {
        this.vT && (await this.vT.sender.replaceTrack(e || null));
      }
      send(e) {
        try {
          this.dc.readyState === "open" && this.dc.send(JSON.stringify(e));
        } catch {}
      }
      close() {
        try {
          this.dc.close();
        } catch {}
        try {
          this.pc.close();
        } catch {}
      }
    },
    ts = new Map();
  function ns(t, e) {
    try {
      let a = Ve();
      if (!a || !e.getAudioTracks().length) return;
      let n = a.createMediaStreamSource(e),
        s = a.createAnalyser();
      (s.fftSize = 256),
        n.connect(s),
        ts.set(t, { an: s, buf: new Uint8Array(s.fftSize), src: n });
    } catch {}
  }
  var $o = null;
  function Gi() {
    clearInterval($o),
      ($o = setInterval(() => {
        for (let [t, e] of ts) {
          e.an.getByteTimeDomainData(e.buf);
          let a = 0;
          for (let s = 0; s < e.buf.length; s++)
            a = Math.max(a, Math.abs(e.buf[s] - 128));
          let n = y(`[data-sp="${t}"]`);
          n &&
            n.classList.toggle(
              "speaking",
              a > 14 && !(t === "me" && ((H && !H.mic) || (z && !z.mic))),
            );
        }
      }, 180));
  }
  function Xu() {
    clearInterval($o), ($o = null);
    for (let t of ts.values())
      try {
        t.src.disconnect();
      } catch {}
    ts.clear();
  }
  var Jt = () => y("#call");
  function Aa() {
    Jt().classList.add("open"), (y("#callpill").hidden = !0);
  }
  function Qu() {
    (!H && !z) ||
      (Jt().classList.remove("open"), (y("#callpill").hidden = !1), So());
  }
  function Ki() {
    Jt().classList.remove("open", "hasvid"),
      (y("#callpill").hidden = !0),
      setTimeout(() => {
        !H && !z && (Jt().innerHTML = "");
      }, 520);
  }
  function So() {
    let t = y("#callpill");
    t.hidden ||
      (H
        ? (t.innerHTML = `${x("phone", "xs")} ${M(Ue(H.pk))} \xB7 ${H.t0 ? Un((Date.now() - H.t0) / 1e3) : Fi()}`)
        : z &&
          (t.innerHTML = `${x("volume", "xs")} ${M(rf())} \xB7 ${z.peers.size + 1} na sala`));
  }
  b("callShow", () => Aa());
  b("callMin", () => Qu());
  var H0 = () => Jt().classList.contains("open");
  function Fi() {
    if (!H) return "";
    switch (H.state) {
      case "calling":
        return "Chamando\u2026";
      case "ringing":
        return "Tocando\u2026";
      case "incoming":
        return H.video
          ? "Chamada de v\xEDdeo recebida"
          : "Chamada de voz recebida";
      case "connecting":
        return "Conectando\u2026";
      case "reconnecting":
        return "Reconectando\u2026";
      case "active":
        return Un((Date.now() - H.t0) / 1e3);
      default:
        return "";
    }
  }
  function Co(t) {
    let e = (a, n, s, o, r = "") =>
      `<button class="cbtn ${r} ${n ? "act" : ""}" data-act="${a}" aria-pressed="${!!n}"><span class="cc">${x(s)}</span>${o}</button>`;
    return (
      e(
        "cMic",
        !t.mic,
        t.mic ? "mic" : "micoff",
        t.mic ? "Microfone" : "Mudo",
      ) +
      e("cCam", t.cam && !t.scr, t.cam && !t.scr ? "video" : "videooff", "C\xE2mera") +
      (X.available ? e("cScreen", !!t.scr, "share", t.scr ? "Parar" : "Tela") : "") +
      `<button class="cbtn sbtn" data-act="cSounds"><span class="cc">${x("wave")}</span>Sons</button>` +
      (t === H && H ? `<button class="cbtn" data-act="peerVol" data-pk="${H.pk}"><span class="cc">${x("volume")}</span>Volume</button>` : "") +
      (t.cam && !t.scr
        ? e("cFlip", !1, "flip", "Virar")
        : e("cSpk", t.spk, "volume", "Alto-falante"))
    );
  }
  function Sa() {
    if (!H) return;
    let t = H,
      e = Jt(),
      a = `<div class="ctop"><button class="ibtn" data-act="callMin" aria-label="Minimizar">${x("chevd")}</button><span class="enc">${x("lock", "xs")}Cifrada de ponta a ponta</span><button class="ibtn" id="rvidfit" data-act="rvidFit" aria-label="Ver o v\xEDdeo inteiro ou preenchendo a tela" ${t.remoteCam ? "" : "hidden"}>${x("scan")}</button></div>`,
      n = `<div class="cmid"><div class="bigav ${t.state === "active" ? "" : "calling"}" data-sp="remote"><i class="ring"></i><i class="ring r2"></i><i class="ring r3"></i>${pe(t.pk, 140, { badge: !1 })}</div>
    <h2 class="cname2">${M(ne(t.pk))}</h2><p class="cstat ${t.state === "active" ? "live" : ""}" id="cstat">${M(Fi())}</p><div id="cplay"></div></div>`,
      s = `<video class="rvid ${h.rvidFit ? "fit" : ""}" id="rvid" autoplay playsinline ${t.remoteCam ? "" : "hidden"}></video><video class="lvid ${t.front ? "" : "back"}" id="lvid" autoplay playsinline muted ${t.cam && t.stream ? "" : "hidden"}></video>`,
      o;
    t.state === "incoming"
      ? (o = `${t.video ? `<div class="ctrls"><button class="cbtn" data-act="cAcceptVoice"><span class="cc">${x("phone")}</span>Atender s\xF3 voz</button></div>` : ""}
      <div class="endwrap"><button class="endbtn" data-act="cDecline" aria-label="Recusar">${x("phone")}</button><button class="endbtn ans" data-act="cAccept" aria-label="Atender">${x(t.video ? "video" : "phone")}</button></div>`)
      : (o = `<div class="ctrls" id="ctrls">${Co(t)}</div><div class="endwrap"><button class="endbtn" data-act="cHangup" aria-label="Encerrar">${x("phone")}</button></div>`),
      (e.innerHTML = s + a + n + o),
      Wi();
  }
  function Wi() {
    let t = y("#rvid"),
      e = y("#lvid");
    H &&
      (t &&
        H.peer &&
        t.srcObject !== H.peer.remote &&
        ((t.srcObject = H.peer.remote),
        (t.volume = jmVolOf(H.pk)),
        t.play().catch(() => {}),
        (t.onresize = () => {
          // Vídeo deitado (tela do computador) aparece inteiro; câmera em pé preenche.
          h.rvidFit === void 0 && t.classList.toggle("fit", t.videoWidth > t.videoHeight * 1.15);
        })),
      e &&
        H.stream &&
        H.cam &&
        e.srcObject !== H.stream &&
        ((e.srcObject = H.stream), e.play().catch(() => {})),
      Jt().classList.toggle("hasvid", !!H.remoteCam),
      t && (t.hidden = !H.remoteCam),
      e && (e.hidden = !(H.cam && H.stream)));
    let jmFit = y("#rvidfit");
    jmFit && (jmFit.hidden = !H || !H.remoteCam);
  }
  function Qn() {
    let t = y("#cstat");
    if (!t || !H) return;
    (t.textContent = Fi()), t.classList.toggle("live", H.state === "active");
    let e = y(".bigav");
    e && e.classList.toggle("calling", H.state !== "active"), So();
  }
  function Ao(t, e, a) {
    let n = y("#cplay");
    if (!n) return;
    let s = a
      ? `<button class="cpsave" data-act="saveCallSnd" data-id="${a}" data-n="${M(e.n)}" data-e="${M(e.e)}" data-d="${e.d}">Salvar</button>`
      : "";
    (n.innerHTML = `<span class="cplay"><span class="eq"><i></i><i></i><i></i><i></i></span>${M(t)} tocou ${M(e.e)} ${M(e.n)}${s}</span>`),
      clearTimeout(Ao.t),
      (Ao.t = setTimeout(
        () => {
          let o = y("#cplay");
          o && (o.innerHTML = "");
        },
        Math.max(e.d * 1e3 + 600, a ? 5e3 : 0),
      ));
  }
  var ln = new Map(),
    Zu = new Map(),
    U0 = /^[\p{Extended_Pictographic}\p{Emoji_Component}\u200d\ufe0f]{1,8}$/u;
  async function D0(t, e) {
    if (t) {
      if (
        !e.builtin &&
        ((t.sentSnd = t.sentSnd || new Set()), !t.sentSnd.has(e.id))
      ) {
        let a = await uo(e.id);
        if (!a || a.length > Vn) return;
        let n = 12e3,
          s = Math.ceil(a.length / n);
        for (let o = 0; o < s; o++)
          t.send({
            t: "sndd",
            id: e.id,
            i: o,
            n: s,
            b: Fa.enc(a.subarray(o * n, (o + 1) * n)),
          });
        t.sentSnd.add(e.id);
      }
      t.send({ t: "snd", id: e.id, n: e.n, e: e.e, d: e.d });
    }
  }
  function ef(t, e) {
    if (
      !Xd.test(e.id) ||
      !Number.isInteger(e.i) ||
      !Number.isInteger(e.n) ||
      e.n < 1 ||
      e.n > 16 ||
      e.i < 0 ||
      e.i >= e.n ||
      typeof e.b != "string" ||
      e.b.length > 17e3
    )
      return;
    let a = t + ":" + e.id,
      n = ln.get(a);
    if (
      (n ||
        ((n = { parts: new Array(e.n), got: 0 }),
        ln.set(a, n),
        ln.size > 20 && ln.delete(ln.keys().next().value)),
      n.parts[e.i])
    )
      return;
    try {
      n.parts[e.i] = Fa.dec(e.b);
    } catch {
      return;
    }
    if (++n.got < e.n) return;
    ln.delete(a);
    let s = n.parts.reduce((i, c) => i + c.length, 0);
    if (s > Vn) return;
    let o = new Uint8Array(s),
      r = 0;
    for (let i of n.parts) o.set(i, r), (r += i.length);
    gi(o) === e.id && su(e.id, o);
  }
  function tf(t, e) {
    if (typeof e.id != "string" || e.id.length > 20) return;
    let a = Date.now(),
      n = (Zu.get(t) || []).filter((r) => a - r < 3e3);
    if (n.length >= 5) return;
    n.push(a), Zu.set(t, n);
    let s = kt(e.id),
      o = s || {
        n: ie(e.n, 24) || "Som",
        e: typeof e.e == "string" && U0.test(e.e) ? e.e : "\u{1F50A}",
        d: Math.max(0.5, Math.min(10, Number(e.d) || 2)),
      };
    Kt(e.id), Ao(Ue(t), o, !s && vi(e.id) ? e.id : "");
  }
  b("saveCallSnd", async (t) => {
    let e = vi(t.dataset.id);
    if (!e) return;
    let a = e[0] === 79 ? "audio/ogg" : "audio/webm";
    await an({
      bytes: e,
      mime: a,
      n: t.dataset.n,
      e: t.dataset.e,
      d: +t.dataset.d || 2,
    }),
      t.remove(),
      _(`"${t.dataset.n}" est\xE1 no seu soundboard`);
  });
  async function Yi(t, e) {
    let a = h.engine;
    if (z) return _("Saia da sala de voz para ligar");
    if (H) return H.pk === t ? Aa() : _("Voc\xEA j\xE1 est\xE1 em uma chamada");
    let n = a.contacts.get(t);
    if (!n || n.status !== "ok") return _("Aceite o contato antes de ligar.");
    if (!a.pool || a.pool.openCount() === 0)
      return _(
        "Sem conex\xE3o com os servidores. Tente quando a internet voltar.",
      );
    (H = {
      pk: t,
      cid: Ka(8),
      dir: "out",
      video: e,
      state: "calling",
      mic: !0,
      cam: e,
      spk: e,
      front: !0,
      ts: Date.now(),
    }),
      Sa(),
      Aa();
    try {
      H.stream = await zi(e);
    } catch (o) {
      _(ze(o)), Ge("error", !0);
      return;
    }
    if (!H) {
      H && H.stream.getTracks().forEach((o) => o.stop());
      return;
    }
    if (
      (af(),
      ns("me", H.stream),
      Gi(),
      X.callMode(!0, e),
      X.setSpeaker(e),
      du(),
      Sa(),
      (H.offer = await H.peer.makeOffer(H.stream)),
      !H)
    )
      return;
    let s = () =>
      H &&
      (H.state === "calling" || H.state === "ringing") &&
      a.signal(t, { cid: H.cid, a: "offer", sdp: H.offer, v: e ? 1 : 0 });
    s(),
      (H.offerT = setInterval(s, 4e3)),
      (H.timeout = setTimeout(() => {
        H &&
          (H.state === "calling" || H.state === "ringing") &&
          (a.missedCall(t, e),
          a.signal(t, { cid: H.cid, a: "end" }),
          Ge("noanswer"),
          _(Ue(t) + " n\xE3o atendeu"));
      }, 45e3));
  }
  function af() {
    let t = H;
    t.peer = new es(t.pk, {
      onRemote: (e, a) => {
        !H || H !== t || (a.kind === "audio" && ns("remote", e), Wi());
      },
      onData: (e) => N0(t, e),
      onOpen: () => {
        t.peer.send({ t: "cam", on: t.cam }),
          t.peer.send({ t: "mic", on: t.mic });
      },
      onState: (e) => {
        !H ||
          H !== t ||
          (e === "connected"
            ? (clearTimeout(t.dropT),
              t.state !== "active"
                ? ((t.state = "active"),
                  (t.t0 = t.t0 || Date.now()),
                  ka(),
                  X.vibrate(40),
                  (t.tick = setInterval(Qn, 1e3)))
                : (t.state = "active"),
              Qn())
            : e === "disconnected"
              ? (t.state === "active" && ((t.state = "reconnecting"), Qn()),
                clearTimeout(t.dropT),
                (t.dropT = setTimeout(() => {
                  H === t &&
                    t.peer.pc.connectionState !== "connected" &&
                    (Ge("failed"), _("A conex\xE3o caiu."));
                }, 1e4)))
              : e === "failed" &&
                (Ge("failed"),
                _(
                  "N\xE3o foi poss\xEDvel conectar a chamada. Redes muito restritas podem precisar de um servidor TURN (Ajustes \u2192 Rede).",
                  4500,
                )));
      },
    });
  }
  function N0(t, e) {
    e.t === "cam"
      ? ((t.remoteCam = !!e.on), Wi())
      : e.t === "sndd"
        ? ef(t.pk, e)
        : e.t === "snd" && tf(t.pk, e);
  }
  async function Zi(t) {
    if (!H || H.state !== "incoming") return;
    ka(),
      X.cancelIncoming(),
      (H.state = "connecting"),
      (H.cam = t),
      (H.spk = t),
      Sa();
    try {
      H.stream = await zi(t);
    } catch (a) {
      _(ze(a)),
        h.engine.signal(H.pk, { cid: H.cid, a: "decline" }),
        Ge("declined");
      return;
    }
    af(), ns("me", H.stream), Gi(), X.callMode(!0, t), X.setSpeaker(t);
    let e = await H.peer.acceptOffer(H.offer, H.stream);
    H && (h.engine.signal(H.pk, { cid: H.cid, a: "answer", sdp: e }), Sa());
  }
  b("cAccept", () => Zi(!!(H && H.video)));
  b("cAcceptVoice", () => Zi(!1));
  b("cDecline", () => {
    H && (h.engine.signal(H.pk, { cid: H.cid, a: "decline" }), Ge("declined"));
  });
  b("cHangup", () => {
    if (z) return ss();
    H &&
      (h.engine.signal(H.pk, { cid: H.cid, a: "end" }),
      Ge(H.t0 ? "ended" : "cancelled"));
  });
  function nf({ from: t, data: e }) {
    let a = h.engine;
    if (!(!e || typeof e.cid != "string")) {
      if (e.room) return j0(t, e);
      if (e.a === "offer") {
        if (Ju.has(e.cid) || (H && H.cid === e.cid)) return;
        let n = a.contacts.get(t);
        if (!n || n.status !== "ok" || a.settings.callsFrom === "nobody")
          return;
        if (H || z) {
          a.signal(t, { cid: e.cid, a: "busy" }),
            sf({
              pk: t,
              dir: "in",
              video: !!e.v,
              ts: Date.now(),
              status: "missed",
            });
          return;
        }
        if (typeof e.sdp != "string" || e.sdp.length > 2e4) return;
        (H = {
          pk: t,
          cid: e.cid,
          dir: "in",
          video: !!e.v,
          state: "incoming",
          offer: e.sdp,
          mic: !0,
          cam: !!e.v,
          spk: !!e.v,
          front: !0,
          ts: Date.now(),
        }),
          a.signal(t, { cid: e.cid, a: "ring" }),
          Sa(),
          Aa(),
          lu(),
          X.vibrate(600),
          h.visible || X.incomingCall({ name: ne(t), video: !!e.v }),
          (H.timeout = setTimeout(() => {
            H && H.cid === e.cid && H.state === "incoming" && Ge("missed");
          }, 45e3));
        return;
      }
      !H ||
        H.cid !== e.cid ||
        H.pk !== t ||
        (e.a === "ring" && H.state === "calling"
          ? ((H.state = "ringing"), Qn())
          : e.a === "answer" &&
              H.dir === "out" &&
              (H.state === "calling" || H.state === "ringing") &&
              typeof e.sdp == "string"
            ? (clearInterval(H.offerT),
              clearTimeout(H.timeout),
              ka(),
              (H.state = "connecting"),
              Qn(),
              H.peer.setAnswer(e.sdp).catch(() => {
                Ge("failed"), _("Falha ao conectar.");
              }))
            : e.a === "decline"
              ? (_(Ue(t) + " recusou a chamada"), Ge("declined"))
              : e.a === "busy"
                ? (_(Ue(t) + " est\xE1 em outra chamada"), Ge("busy"))
                : e.a === "end" &&
                  Ge(
                    H.state === "incoming"
                      ? "missed"
                      : H.t0
                        ? "ended"
                        : "cancelled",
                  ));
    }
  }
  function Ge(t) {
    if (!H) return;
    H.scr && (X.screenStop(), jmScr && ((jmScr.on = !1), jmScr.track.stop(), (jmScr = null)));
    let e = H;
    if (
      ((H = null),
      Ju.add(e.cid),
      clearInterval(e.offerT),
      clearTimeout(e.timeout),
      clearInterval(e.tick),
      clearTimeout(e.dropT),
      ka(),
      Xu(),
      X.cancelIncoming(),
      e.peer && e.peer.close(),
      e.stream && e.stream.getTracks().forEach((a) => a.stop()),
      X.callMode(!1, !1),
      X.setSpeaker(!1),
      t !== "error")
    ) {
      let a = e.t0
        ? "ok"
        : t === "declined"
          ? "declined"
          : t === "busy"
            ? "busy"
            : t === "cancelled"
              ? "cancelled"
              : "missed";
      sf({
        pk: e.pk,
        dir: e.dir,
        video: e.video,
        ts: e.ts,
        dur: e.t0 ? Math.round((Date.now() - e.t0) / 1e3) : 0,
        status: a,
      });
    }
    (e.t0 || e.dir === "out") && wi(),
      e.t0 && _("Chamada encerrada \xB7 " + Un((Date.now() - e.t0) / 1e3)),
      Ki();
  }
  async function sf(t) {
    (t.id = t.id || Ka(8)), await h.engine.db.put("calls", t), P.refreshLists();
  }
  async function _0() {
    let t = H || z;
    if (t) {
      if (t.cam) {
        t.cam = !1;
        for (let e of as())
          await e.setVideo(null), e.send({ t: "cam", on: !1 });
        t.stream.getVideoTracks().forEach((e) => {
          e.stop(), t.stream.removeTrack(e);
        });
      } else
        try {
          let a = (
            await navigator.mediaDevices.getUserMedia({
              video: {
                facingMode: t.front ? "user" : "environment",
                width: { ideal: 640 },
                height: { ideal: 480 },
                frameRate: { ideal: 24 },
              },
            })
          ).getVideoTracks()[0];
          t.stream.getVideoTracks().forEach((n) => {
            n.stop(), t.stream.removeTrack(n);
          }),
            t.stream.addTrack(a),
            (t.cam = !0);
          for (let n of as()) await n.setVideo(a), n.send({ t: "cam", on: !0 });
          X.callMode(!0, !0);
        } catch (e) {
          _(ze(e));
          return;
        }
      H ? Sa() : at();
    }
  }
  async function O0() {
    let t = H || z;
    if (!(!t || !t.cam)) {
      t.front = !t.front;
      try {
        t.stream.getVideoTracks().forEach((n) => {
          n.stop(), t.stream.removeTrack(n);
        });
        let a = (
          await navigator.mediaDevices.getUserMedia({
            video: {
              facingMode: t.front ? "user" : "environment",
              width: { ideal: 640 },
              height: { ideal: 480 },
            },
          })
        ).getVideoTracks()[0];
        t.stream.addTrack(a);
        for (let n of as()) await n.setVideo(a);
      } catch {
        _("N\xE3o foi poss\xEDvel trocar de c\xE2mera.");
      }
      H ? Sa() : at();
    }
  }
  function as() {
    return H
      ? H.peer
        ? [H.peer]
        : []
      : z
        ? [...z.peers.values()].map((t) => t.peer)
        : [];
  }
  b("cMic", () => {
    let t = H || z;
    if (!t) return;
    (t.mic = !t.mic),
      t.stream &&
        t.stream.getAudioTracks().forEach((a) => {
          a.enabled = t.mic;
        });
    for (let a of as()) a.send({ t: "mic", on: t.mic });
    let e = y("#ctrls");
    e && (e.innerHTML = Co(t)), z && at();
  });
  b("cCam", () => {
    let t = H || z;
    t && t.scr ? jmScreenOff().then(() => _0()) : _0();
  });
  // ---- Transmitir a tela do celular: o Android captura, a página desenha num
  // canvas e o canvas vira a trilha de vídeo (como a câmera).
  var jmScr = null;
  b("cScreen", () => {
    let t = H || z;
    t && (t.scr ? (X.screenStop(), jmScreenOff()) : (_("Escolha \u201CCome\xE7ar agora\u201D para transmitir"), X.screenStart()));
  });
  async function jmScreenOn() {
    let t = H || z;
    if (!t) return X.screenStop();
    let e = document.createElement("canvas");
    (e.width = 720), (e.height = 1280);
    let a = e.getContext("2d"),
      n = e.captureStream(15).getVideoTracks()[0],
      s = { on: !0, track: n };
    (jmScr = s),
      (async () => {
        for (; s.on; ) {
          try {
            let o = await fetch("/__screen.jpg?" + Date.now(), { cache: "no-store" });
            if (o.ok) {
              let r = await createImageBitmap(await o.blob());
              (e.width !== r.width || e.height !== r.height) && ((e.width = r.width), (e.height = r.height)), a.drawImage(r, 0, 0), r.close();
            }
          } catch {}
          await new Promise((o) => setTimeout(o, 70));
        }
      })(),
      t.stream.getVideoTracks().forEach((o) => {
        o.stop(), t.stream.removeTrack(o);
      }),
      t.stream.addTrack(n),
      (t.cam = !0),
      (t.scr = !0);
    for (let o of as()) await o.setVideo(n), o.send({ t: "cam", on: !0 });
    _("Transmitindo a sua tela"), H ? Sa() : at();
  }
  async function jmScreenOff() {
    let t = H || z,
      e = jmScr;
    if (((jmScr = null), e && ((e.on = !1), e.track.stop()), !t || !t.scr)) return;
    (t.scr = !1), (t.cam = !1), t.stream.getVideoTracks().forEach((a) => t.stream.removeTrack(a));
    for (let a of as()) await a.setVideo(null), a.send({ t: "cam", on: !1 });
    H ? Sa() : at();
  }
  b("cFlip", () => O0());
  b("cSpk", () => {
    let t = H || z;
    if (!t) return;
    (t.spk = !t.spk), X.setSpeaker(t.spk);
    let e = y("#ctrls");
    e && (e.innerHTML = Co(t)),
      _(t.spk ? "Alto-falante ligado" : "Alto-falante desligado");
  });
  b("cSounds", () => P.soundSheet("call"));
  async function of(t) {
    let e = kt(t);
    if (e) {
      Ao("Voc\xEA", e);
      for (let a of as()) await D0(a, e);
    }
  }
  var rf = () => {
      if (!z) return "";
      let t = h.engine.groups.get(z.lid),
        e = t && t.meta.channels.find((a) => a.id === z.ch);
      return e ? e.name : "Sala de voz";
    },
    cf = (t, e) => !!(z && z.lid === t && z.ch === e),
    lf = (t) => !!(z && z.lid === t);
  async function df(t, e) {
    let a = h.engine;
    if (H) return _("Termine a chamada atual primeiro");
    if (z) {
      if (z.lid === t && z.ch === e) return Aa();
      ss();
    }
    let n = a.groups.get(t);
    if (!n) return;
    let s = a
      .roomMembers(t, e)
      .filter((o) => o !== a.pk && n.meta.members.includes(o));
    if (s.length >= Eo)
      return _(`Sala cheia (at\xE9 ${Eo} pessoas nesta vers\xE3o beta)`);
    (z = {
      lid: t,
      ch: e,
      peers: new Map(),
      mic: !0,
      cam: !1,
      spk: !0,
      front: !0,
      t0: Date.now(),
    }),
      at(),
      Aa();
    try {
      z.stream = await zi(!1);
    } catch (o) {
      _(ze(o)), (z = null), Ki();
      return;
    }
    ns("me", z.stream),
      Gi(),
      X.callMode(!0, !1),
      X.setSpeaker(!0),
      a.roomPresence(t, e, "here"),
      (z.hb = setInterval(() => z && a.roomPresence(t, e, "here"), 8e3)),
      (z.tick = setInterval(() => {
        let o = y("#cstat");
        o &&
          z &&
          (o.textContent = "Conectado \xB7 " + Un((Date.now() - z.t0) / 1e3)),
          So();
      }, 1e3));
    for (let o of s) ff(o);
    at(), P.refreshGroup && P.refreshGroup();
  }
  function uf(t, e) {
    return {
      onRemote: (a, n) => {
        n.kind === "audio" && ns(t, a), at();
      },
      onData: (a) => {
        a.t === "cam"
          ? ((e.cam = !!a.on), at())
          : a.t === "mic"
            ? ((e.mic = !!a.on), at())
            : a.t === "sndd"
              ? ef(t, a)
              : a.t === "snd" && tf(t, a);
      },
      onOpen: () => {
        e.peer.send({ t: "cam", on: z && z.cam }),
          e.peer.send({ t: "mic", on: z && z.mic });
      },
      onState: (a) => {
        (e.state = a), (a === "failed" || a === "closed") && Ji(t), at();
      },
    };
  }
  async function ff(t) {
    let e = h.engine;
    if (
      !z ||
      z.peers.has(t) ||
      t === e.pk ||
      z.peers.size >= Eo - 1 ||
      !(e.pk < t)
    )
      return;
    let a = { state: "new", cam: !1, mic: !0 };
    (a.peer = new es(t, uf(t, a))), z.peers.set(t, a);
    let n = await a.peer.makeOffer(z.stream);
    !z ||
      !z.peers.has(t) ||
      e.signal(t, {
        cid: "room",
        a: "offer",
        sdp: n,
        room: { lid: z.lid, ch: z.ch },
      });
  }
  async function j0(t, e) {
    let a = h.engine;
    if (!z || !e.room || e.room.lid !== z.lid || e.room.ch !== z.ch) return;
    let n = a.groups.get(z.lid);
    if (!(!n || !n.meta.members.includes(t)))
      if (e.a === "offer" && typeof e.sdp == "string") {
        let s = z.peers.get(t);
        if ((s && (s.peer.close(), z.peers.delete(t)), z.peers.size >= Eo - 1))
          return;
        let o = { state: "new", cam: !1, mic: !0 };
        (o.peer = new es(t, uf(t, o))), z.peers.set(t, o);
        let r = await o.peer.acceptOffer(e.sdp, z.stream);
        if (!z) return;
        a.signal(t, {
          cid: "room",
          a: "answer",
          sdp: r,
          room: { lid: z.lid, ch: z.ch },
        }),
          at();
      } else if (e.a === "answer" && typeof e.sdp == "string") {
        let s = z.peers.get(t);
        s && s.peer.setAnswer(e.sdp).catch(() => {});
      } else e.a === "end" && Ji(t);
  }
  function Ji(t) {
    if (!z) return;
    let e = z.peers.get(t);
    e && (e.peer.close(), z.peers.delete(t), ts.delete(t), at());
  }
  function pf(t, e, a) {
    if (!(!z || z.lid !== t || z.ch !== e)) {
      for (let n of a) ff(n);
      for (let n of [...z.peers.keys()])
        if (!a.includes(n)) {
          let s = z.peers.get(n);
          s && s.state !== "connected" && Ji(n);
        }
    }
  }
  function hf() {
    if (!z) return;
    let t = h.engine.groups.get(z.lid);
    (!t || !t.meta.channels.some((e) => e.id === z.ch && e.type === "voice")) &&
      (ss(), _("A sala de voz foi apagada por um admin."));
  }
  function ss() {
    if (!z) return;
    z.scr && (X.screenStop(), jmScr && ((jmScr.on = !1), jmScr.track.stop(), (jmScr = null)));
    let t = z;
    z = null;
    let e = h.engine;
    e.roomPresence(t.lid, t.ch, "leave");
    for (let [a, n] of t.peers)
      e.signal(a, { cid: "room", a: "end", room: { lid: t.lid, ch: t.ch } }),
        n.peer.close();
    clearInterval(t.hb),
      clearInterval(t.tick),
      Xu(),
      t.stream && t.stream.getTracks().forEach((a) => a.stop()),
      X.callMode(!1, !1),
      X.setSpeaker(!1),
      wi(),
      Ki(),
      _("Voc\xEA saiu da sala de voz"),
      P.refreshGroup && P.refreshGroup(),
      P.refreshLists();
  }
  function at() {
    if (!z) return;
    let t = h.engine,
      e = t.groups.get(z.lid),
      a = `<div class="ctop"><button class="ibtn" data-act="callMin" aria-label="Minimizar">${x("chevd")}</button><span class="enc">${x("lock", "xs")}Cifrada de ponta a ponta</span><span style="width:40px"></span></div>`,
      n = `<div class="cmid compact"><h2 class="cname2 sm">${x("volume")}${M(rf())}</h2><p class="cstat live" id="cstat">${M(e ? e.meta.name : "")}</p><div id="cplay"></div></div>`,
      o = `<div class="pgrid" id="pgrid">${[
        ["me", t.pk, z.mic, z.cam, "connected"],
        ...[...z.peers.entries()].map(([c, l]) => [
          c,
          c,
          l.mic,
          l.cam,
          l.state,
        ]),
      ]
        .map(
          (
            [c, l, u, d, f],
            p,
          ) => `<div class="ptile" style="--i:${p}" data-sp="${c}" ${c !== "me" ? `data-act="peerVol" data-pk="${l}"` : ""}>
      ${d ? `<video data-vid="${c}" autoplay playsinline ${c === "me" ? "muted" : ""}></video>` : pe(l, 64, { badge: !1 })}
      ${c !== "me" && f !== "connected" ? '<span class="pst">conectando\u2026</span>' : ""}
      <span class="pn">${u ? "" : x("micoff")}${c === "me" ? "Voc\xEA" : M(Ue(l))}</span>
      ${c !== "me" && !d ? `<audio data-aud="${c}" autoplay></audio>` : ""}</div>`,
        )
        .join("")}</div>`,
      r = `<div class="ctrls" id="ctrls">${Co(z)}</div><div class="endwrap"><button class="endbtn" data-act="cHangup" aria-label="Sair da sala">${x("phone")}</button></div>`,
      i = Jt();
    i.classList.remove("hasvid"), (i.innerHTML = a + n + o + r);
    for (let c of oe("video[data-vid]", i)) {
      let l = c.dataset.vid;
      (c.srcObject =
        l === "me" ? z.stream : (z.peers.get(l) || {}).peer.remote),
        l !== "me" && (c.volume = jmVolOf(l)),
        c.play().catch(() => {});
    }
    for (let c of oe("audio[data-aud]", i)) {
      let l = z.peers.get(c.dataset.aud);
      l && ((c.srcObject = l.peer.remote), (c.volume = jmVolOf(c.dataset.aud)), c.play().catch(() => {}));
    }
    So();
  }
  function mf() {
    return H0() && (H || z) ? ((H && H.state === "incoming") || Qu(), !0) : !1;
  }
  function bf(t) {
    !H ||
      H.state !== "incoming" ||
      (Aa(),
      t === "accept"
        ? Zi(!!H.video)
        : t === "decline" &&
          (h.engine.signal(H.pk, { cid: H.cid, a: "decline" }),
          Ge("declined")));
  }
  function gf() {
    let e = [...h.engine.contacts.values()]
      .filter((a) => a.status === "ok")
      .sort((a, n) => ne(a.pk).localeCompare(ne(n.pk)));
    ee(
      `<h2 class="sh-title">Nova chamada</h2>${
        e.length
          ? `<div class="mlist">${e
              .map(
                (
                  a,
                ) => `<div class="mitem" style="gap:12px">${pe(a.pk, 40)}<span style="flex:1;min-width:0">${M(ne(a.pk))}</span>
    <button class="ibtn" data-act="pickCall" data-pk="${a.pk}" aria-label="Voz">${x("phone")}</button><button class="ibtn" data-act="pickCall" data-pk="${a.pk}" data-video="1" aria-label="V\xEDdeo">${x("video")}</button></div>`,
              )
              .join("")}</div>`
          : '<p class="sh-sub">Adicione contatos para poder ligar.</p><button class="btn" data-act="addContact">Adicionar contato</button>'
      }`,
    );
  }
  b("pickCall", (t) => {
    F(), Yi(t.dataset.pk, !!t.dataset.video);
  });
  var xf = () => !!(H || z);
  var le = (
    t,
  ) => `<div class="srow ${t.danger ? "danger" : ""}" role="button" tabindex="0" data-act="${t.act}" ${t.data || ""} ${t.disabled ? 'aria-disabled="true"' : ""}>
  ${t.i ? `<span class="sic">${x(t.i)}</span>` : ""}<span class="sm"><b>${t.n}</b>${t.s ? `<small>${t.s}</small>` : ""}</span>
  ${t.sw !== void 0 ? `<span class="sw" role="switch" aria-checked="${!!t.sw}"></span>` : t.val !== void 0 ? `<span class="val">${t.val}</span><span class="chev">${x("chev", "sm")}</span>` : t.dot ? '<span class="cdot"></span><span class="chev">' + x("chev", "sm") + "</span>" : t.danger ? "" : `<span class="chev">${x("chev", "sm")}</span>`}</div>`;
  async function Xt() {
    let t = h.engine;
    if (!t || !t.me) return;
    let e = await t.kv("wordsPending", !1),
      a = t.pool ? t.pool.openCount() : 0,
      n = t.settings.relays.length;
    y("#setList").innerHTML = `
    <div class="prof" role="button" tabindex="0" data-act="editProfile"><span class="pfwrap" role="button" tabindex="0" data-act="myPhoto" aria-label="Mudar foto do perfil">${pe(t.pk, 68, { ring: "var(--s1)" })}<i class="pfcam">${x("camera")}</i></span><div style="min-width:0;flex:1"><h2>${M(t.me.name)}</h2><p>${M(t.me.note || "Toque para editar seu perfil")}</p></div><span class="ibtn">${x("edit")}</span></div>
    ${e ? `<button class="reqhead" data-act="openRecovery" style="margin-top:12px">${x("key")}<b>Anote sua chave de recupera\xE7\xE3o</b>${x("chev", "sm")}</button>` : ""}
    <div class="label">Conta</div>
    ${le({ i: "qr", n: "Meu c\xF3digo", s: "QR e c\xF3digo para algu\xE9m te adicionar", act: "myCode" })}
    ${le({ i: "smile", n: "Status", s: (() => { let jmS = jmCleanStatus(h.engine.me.st); return jmS ? M(((jmS.e || "") + " " + (jmS.t || "")).trim() || "") + (jmS.dnd ? " \xB7 N\xE3o perturbe" : "") : "Mostre o que voc\xEA est\xE1 fazendo"; })(), act: "statusOpen" })}
    ${le({ i: "key", n: "Chave de recupera\xE7\xE3o", s: "As 12 palavras que recuperam sua conta", act: "openRecovery" })}
    ${le({ i: "link", n: "Aparelhos conectados", s: "Onde esta conta est\xE1 aberta", act: "devicesOpen" })}
    <div class="label">Privacidade e avisos</div>
    ${le({ i: "shield", n: "Privacidade e seguran\xE7a", s: "PIN, capturas de tela, recibos, bloqueados", act: "openPrivacy" })}
    ${le({ i: "bell", n: "Notifica\xE7\xF5es", s: t.settings.notifications ? "Ativadas" + (t.settings.background ? " \xB7 conectado em segundo plano" : "") : "Desativadas", act: "openNotifs" })}
    <div class="label">Aplicativo</div>
    ${le({ i: "drop", n: "Apar\xEAncia", s: 'Cores \xB7 destaque <span class="acc-name">' + M(mi()) + "</span>", act: "openAppearance", dot: !0 })}
    ${le({ i: "wave", n: "Soundboard", s: "Ouvir, adicionar e excluir os sons das chamadas", act: "openSoundsS" })}
    ${le({ i: "mic", n: "Voz e \xE1udio", s: "Microfone, ru\xEDdo e eco nas chamadas", act: "openAudio" })}
    ${le({ i: "server", n: "Rede e servidores", s: `Conectado a ${a} de ${n} servidores`, act: "openNetwork" })}
    ${le({ i: "db", n: "Armazenamento", s: "Espa\xE7o usado e limpeza de m\xEDdia", act: "openStorage" })}
    <div class="label">Sobre</div>
    ${le({ i: "sparkle", n: "Sobre o J-Menseger", s: `Vers\xE3o ${h.version} \xB7 como a privacidade funciona`, act: "openAbout" })}
    ${X.available ? le({ i: "download", n: jmUpd.info ? "Atualiza\xE7\xE3o dispon\xEDvel" : "Procurar atualiza\xE7\xF5es", s: jmUpd.info ? `Vers\xE3o ${M(jmUpd.info.versao)} \xB7 toque para instalar` : "O app procura sozinho a cada 6 horas", act: "updOpen", dot: !!jmUpd.info }) : ""}
    ${le({ i: "share", n: "Convidar amigos", s: "Compartilhe seu c\xF3digo", act: "shareMyCode" })}
    ${le({ i: "logout", n: "Sair e apagar deste aparelho", s: "Remove a conta e as conversas deste celular", act: "wipeAll", danger: !0 })}
    <div style="height:100px"></div>`;
  }
  b("soundPreview", () => P.soundSheet("preview"));
  b("openSoundsS", () => P.openSounds());
  b("myPhoto", async () => {
    let t = h.engine,
      e = await ho({
        title: "Foto do perfil",
        hasPic: !!t.me.pic,
        shape: "round",
        facing: "user",
      });
    e !== null &&
      (await t.updateProfile({ pic: e }),
      Xt(),
      P.refreshLists(),
      _(
        e
          ? "Foto atualizada. Seus contatos recebem a nova foto."
          : "Foto removida",
      ));
  });
  var dn = 0;
  b("editProfile", () => {
    let t = h.engine;
    (dn = t.me.t || 0),
      ee(`<h2 class="sh-title">Seu perfil</h2><p class="sh-sub">Seus contatos recebem a mudan\xE7a de forma cifrada. Nada fica p\xFAblico.</p>
    <div class="pavwrap" id="ppav">${et({ name: t.me.name, t: dn, size: 88, pic: t.me.pic })}</div>
    <div style="text-align:center;margin-top:-4px"><button class="linkbtn" data-act="myPhotoS">${x("camera", "sm")} ${t.me.pic ? "Mudar foto" : "Colocar foto"}</button></div>
    <div class="tones">${[0, 1, 2, 3, 4, 5].map((e) => `<button class="${e === dn ? "on" : ""}" data-act="profTone" data-t="${e}" aria-label="Cor ${e + 1}"><span class="av t${e}" style="--sz:34px"></span></button>`).join("")}</div>
    <label class="lbl" for="pname">Nome</label><input class="inp" id="pname" maxlength="40" value="${M(t.me.name)}">
    <label class="lbl" for="pnote">Recado</label><input class="inp" id="pnote" maxlength="80" value="${M(t.me.note || "")}" placeholder="Dispon\xEDvel para conversar">
    <button class="btn" data-act="saveProfile">Salvar</button>
    <div id="pextras">${jmMyExtras()}</div>`);
  });
  /** Banner, jogos favoritos e Spotify na folha "Seu perfil". */
  function jmMyExtras() {
    let t = h.engine,
      e = t.me.games || [],
      a = h.spotify || {},
      n = a.cur;
    return `<label class="lbl">Banner</label>
    ${t.me.banner ? `<img class="pbanner" src="${t.me.banner}" alt="">` : '<div class="pbanner empty"></div>'}
    <div class="two" style="margin-top:10px"><button class="btn ghost" data-act="myBanner" style="margin:0">${x("image", "sm")}&nbsp;${t.me.banner ? "Mudar banner" : "Colocar banner"}</button>${t.me.banner ? `<button class="btn ghost" data-act="myBannerDel" style="margin:0">${x("trash", "sm")}&nbsp;Remover</button>` : ""}</div>
    <label class="lbl">Jogos favoritos</label>
    <p class="note" style="padding:0 0 8px">Aparecem no seu perfil para os seus contatos, como no Discord. At\xE9 6.</p>
    <div class="games" style="padding:0">${e.length ? e.map((s, o) => `<span class="chip">${x("game", "xs")}${M(s)}<button data-act="gameDel" data-i="${o}" aria-label="Tirar">\xD7</button></span>`).join("") : '<span class="note">Nenhum jogo ainda.</span>'}</div>
    ${e.length < 6 ? '<div class="gameadd" style="padding:10px 0 0"><input class="inp" id="pgame" maxlength="30" placeholder="Nome do jogo" autocomplete="off"><button class="btn" data-act="gameAdd">Adicionar</button></div>' : ""}
    <label class="lbl">Atividade</label>
    <div class="srow" role="button" tabindex="0" data-act="spotifyTgl" style="margin:0 -18px"><span class="sic">${x("music")}</span><span class="sm"><b>Mostrar a m\xFAsica do Spotify</b><small>Seus contatos veem o que est\xE1 tocando, como no Discord</small></span><span class="sw" role="switch" aria-checked="${!!a.share}"></span></div>
    <p class="note" style="padding:6px 0 0">${n ? "Tocando agora: " + M(n.s) + (n.a ? " \u2014 " + M(n.a) : "") : "Nada tocando no Spotify agora."}</p>
    ${X.available ? `<p class="note" style="padding:6px 0 0">Para o J-Menseger ver a m\xFAsica, ligue no Spotify: <b>Configura\xE7\xF5es \u2192 Transmiss\xE3o do status do dispositivo</b> (\u201CDevice Broadcast Status\u201D).</p><button class="btn ghost" data-act="openSpotifyApp">${x("music", "sm")}&nbsp;Abrir o Spotify</button>` : ""}`;
  }
  function jmRedrawExtras() {
    let t = y("#pextras");
    t && (t.innerHTML = jmMyExtras());
  }
  b("myBanner", async () => {
    let t = h.engine;
    F(),
      setTimeout(async () => {
        let e = await ho({ title: "Banner do perfil", hasPic: !!t.me.banner, shape: "banner", facing: "environment" });
        if (e === null) return;
        try {
          await t.updateProfileExtras({ banner: e }),
            _(e ? "Banner atualizado. Seus contatos recebem o novo banner." : "Banner removido");
        } catch (a) {
          _(a.message);
        }
      }, 260);
  });
  b("myBannerDel", async () => {
    await h.engine.updateProfileExtras({ banner: "" }), jmRedrawExtras(), _("Banner removido");
  });
  b("gameAdd", async () => {
    let t = y("#pgame"),
      e = t && t.value.trim();
    if (!e) return;
    await h.engine.updateProfileExtras({ games: [...(h.engine.me.games || []), e] }), jmRedrawExtras();
    let a = y("#pgame");
    a && a.focus();
  });
  b("gameDel", async (t) => {
    let e = [...(h.engine.me.games || [])];
    e.splice(+t.dataset.i, 1), await h.engine.updateProfileExtras({ games: e }), jmRedrawExtras();
  });
  document.addEventListener("keydown", (t) => {
    t.key === "Enter" && t.target && t.target.id === "pgame" && (t.preventDefault(), vt.gameAdd && vt.gameAdd(t.target));
  });
  b("spotifyTgl", async () => {
    let t = (h.spotify = h.spotify || {});
    (t.share = !t.share), await h.engine.setKv("spotifyShare", t.share), jmSpotifyPush(), jmRedrawExtras();
  });
  b("openSpotifyApp", () => {
    try {
      window.JMNative && window.JMNative.openSpotify();
    } catch {}
  });
  /** Música do Spotify vista pelo Android (ou null) -> conta aos contatos. */
  function jmSpotifyFrom(t) {
    let e = (h.spotify = h.spotify || {});
    if (!t || !t.s) {
      (e.cur = null), jmSpotifyPush(), jmRedrawExtras();
      return;
    }
    let a = String(t.id || "").split(":"),
      n = a[0] === "spotify" && a[1] === "track" ? a[2] : void 0,
      s = Number(t.len) || 0,
      o = s > 2e4 ? Math.round(s / 1e3) : s || void 0;
    (e.cur && e.cur.s === t.s && e.cur.a === t.a) || (e.st = Date.now()),
      (e.cur = { s: t.s, a: t.a || "", al: t.al || "", id: n, d: o }),
      jmSpotifyPush(),
      jmRedrawExtras();
  }
  function jmSpotifyPush() {
    let t = h.engine,
      e = h.spotify || {};
    t && t.me && t.setListening(e.share && e.cur ? { ...e.cur, st: e.st } : null).catch(() => {});
  }
  async function jmSpotifyInit(t) {
    let e = (h.spotify = h.spotify || {});
    e.share = await t.kv("spotifyShare", !1);
    try {
      let a = window.JMNative && window.JMNative.nowPlaying && window.JMNative.nowPlaying();
      a && jmSpotifyFrom(JSON.parse(a));
    } catch {}
    setInterval(jmSpotifyPush, 6e4);
  }
  b("profTone", (t) => {
    (dn = +t.dataset.t),
      oe(".tones button").forEach((e) => e.classList.toggle("on", e === t)),
      (y("#ppav").innerHTML = et({
        name: y("#pname").value,
        t: dn,
        size: 88,
        pic: h.engine.me.pic,
      }));
  });
  b("myPhotoS", () => {
    F(), setTimeout(() => vt.myPhoto(), 260);
  });
  b("saveProfile", async () => {
    let t = y("#pname").value.trim();
    if (t.length < 2) {
      y("#pname").classList.add("err");
      return;
    }
    await h.engine.updateProfile({
      name: t,
      note: y("#pnote").value.trim(),
      t: dn,
    }),
      F(),
      Xt(),
      _("Perfil atualizado");
  });
  b("openRecovery", async () => {
    if ((await Bo()) && !(await Ro("Confirme seu PIN para ver a chave")))
      return;
    let t = await h.engine.exportKeys(),
      e = t.words ? t.words.split(" ") : null;
    J.open(
      "recovery",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Chave de recupera\xE7\xE3o</h2></div>
    <div class="scroll" style="padding:0 18px">
      <p class="sh-sub" style="margin-top:8px">Quem tiver estas palavras controla sua conta. Anote no papel, na ordem, e guarde longe do celular. Nunca envie para ningu\xE9m, nem para o suporte.</p>
      ${
        e
          ? `<div class="wgrid blur selectable" id="rwords">${e.map((a, n) => `<span style="--i:${n}"><i>${n + 1}</i>${M(a)}</span>`).join("")}</div>
      <button class="btn ghost" data-act="revealWords">${x("eye", "sm")}&nbsp;Mostrar palavras</button>`
          : '<p class="note" style="padding:12px 0">Esta conta foi restaurada pela chave secreta, sem palavras. Guarde a chave abaixo.</p>'
      }
      <label class="lbl">Chave secreta (avan\xE7ado)</label>
      <div class="idbox selectable"><span id="nsecbox">nsec1\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022</span><button class="ibtn sm" data-act="revealNsec" aria-label="Mostrar chave">${x("eye", "sm")}</button></div>
      <div class="label" style="padding-left:0">Usar em outro aparelho</div>
      <p class="sh-sub">No PC ou em outro celular, escolha \u201CJ\xE1 tenho uma conta\u201D e digite estas palavras. Com este aparelho aberto, o outro recebe seus contatos e grupos sozinho. As mensagens novas chegam nos dois; para levar o hist\xF3rico antigo, use Armazenamento \u2192 Fazer backup.</p>
      <button class="btn" data-act="wordsDone">J\xE1 anotei</button>
      <div style="height:30px"></div></div></div>`,
    ),
      b("revealWords", () => {
        let a = y("#rwords");
        a && a.classList.remove("blur");
      }),
      b("revealNsec", () => {
        y("#nsecbox").textContent = t.nsec;
      });
  });
  b("wordsDone", async () => {
    await h.engine.setKv("wordsPending", !1),
      J.pop(),
      Xt(),
      _("\xD3timo. Guarde em lugar seguro.");
  });
  async function P0() {
    let t = J.open(
      "privacy",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Privacidade e seguran\xE7a</h2></div><div class="scroll" id="privbody"></div></div>`,
    );
    return await Ca(), t;
  }
  async function Ca() {
    let t = h.engine,
      e = t.settings,
      a = await t.kv("lock", null),
      n = [...t.contacts.values()].filter((c) => c.status === "blocked").length,
      s =
        {
          full: "Nome e mensagem",
          name: "S\xF3 o nome",
          none: 'Nada (s\xF3 "nova mensagem")',
        }[e.notifPreview] || "S\xF3 o nome",
      o = {
        0: "Imediatamente",
        60: "Ap\xF3s 1 minuto",
        300: "Ap\xF3s 5 minutos",
        900: "Ap\xF3s 15 minutos",
      },
      r = !!(e.turn && e.turn.url),
      i = y("#privbody");
    i &&
      (i.innerHTML = `
    <div class="label">Bloqueio do app</div>
    ${le({ i: "lock", n: "PIN de bloqueio", s: a ? "Ligado \xB7 pede o PIN ao abrir o app" : "Desligado \xB7 o app abre sem senha", act: "pinToggle", sw: !!a })}
    ${a ? le({ n: "Trocar PIN", act: "pinSetup" }) + le({ n: "Pedir o PIN", s: "Depois de quanto tempo fora do app", act: "pinDelay", val: o[a.delay] || "Imediatamente" }) + le({ n: "Tirar o PIN", s: "O app volta a abrir sem senha", act: "pinRemove", danger: !0 }) : ""}
    ${le({ i: "eyeoff", n: "Bloquear capturas de tela", s: "Esconde o conte\xFAdo nas capturas e na tela de apps recentes", act: "tgl", data: 'data-k="lockScreenshots"', sw: e.lockScreenshots })}
    <div class="label">Mensagens</div>
    ${le({ i: "checks", n: "Confirma\xE7\xE3o de leitura", s: "Se desligar, voc\xEA tamb\xE9m n\xE3o v\xEA quando leram", act: "tgl", data: 'data-k="readReceipts"', sw: e.readReceipts })}
    ${le({ i: "share", n: "Pr\xE9via dos links que eu mando", s: "Seu celular abre o site para pegar t\xEDtulo e imagem; quem recebe n\xE3o acessa nada", act: "tgl", data: 'data-k="linkPreviews"', sw: e.linkPreviews !== !1 })}
    ${le({ i: "eye", n: "Mostrar quando estou online", s: "Se desligar, voc\xEA tamb\xE9m n\xE3o v\xEA quem est\xE1 online", act: "tgl", data: 'data-k="presence"', sw: e.presence !== !1 })}
    ${le({ i: "chat", n: "Mostrar quando estou digitando", act: "tgl", data: 'data-k="typing"', sw: e.typing })}
    ${le({ i: "timer", n: "Mensagens tempor\xE1rias para novas conversas", act: "ttlDefault", val: yt(e.ttlDefault) })}
    ${le({ i: "bell", n: "Pr\xE9via nas notifica\xE7\xF5es", act: "notifPreview", val: s })}
    <div class="label">Chamadas</div>
    ${le({ i: "phone", n: "Quem pode me ligar", act: "callsFrom", val: e.callsFrom === "nobody" ? "Ningu\xE9m" : "Meus contatos" })}
    ${le({ i: "shield", n: "Ocultar meu IP nas chamadas", s: r ? "Toda chamada passa pelo servidor TURN configurado" : "Precisa de um servidor TURN em Rede e servidores", act: "tgl", data: 'data-k="forceRelay"', sw: e.forceRelay && r, disabled: !r })}
    <div class="label">Contatos</div>
    ${le({ i: "ban", n: "Bloqueados", act: "openBlockedS", val: String(n) })}
    <p class="note">Tudo o que voc\xEA manda \xE9 cifrado no seu aparelho antes de sair. Os servidores s\xF3 guardam envelopes lacrados com destinat\xE1rio, sem remetente, conte\xFAdo nem hor\xE1rio real. N\xE3o h\xE1 telefone, e-mail, agenda de contatos nem an\xFAncios.</p>
    <div style="height:40px"></div>`);
  }
  b("openPrivacy", () => P0());
  b("openBlockedS", () => P.openBlocked());
  b("tgl", async (t) => {
    let e = t.dataset.k,
      a = h.engine;
    if (t.getAttribute("aria-disabled") === "true") return;
    let n = !a.settings[e];
    await a.setSetting(e, n);
    let s = y(".sw", t);
    s && s.setAttribute("aria-checked", n),
      e === "lockScreenshots" && X.setSecure(n),
      e === "presence" && jmPresTick(!0),
      e === "background" && X.setBackground(n && a.settings.notifications),
      e === "notifications" &&
        (n && X.requestNotifications(),
        X.setBackground(n && a.settings.background));
  });
  function To(t, e, a, n, s) {
    ee(
      `<h2 class="sh-title">${M(t)}</h2>${e ? `<p class="sh-sub">${M(e)}</p>` : ""}<div class="opts">${a.map(([o, r, i]) => `<button class="opt ${String(n) === String(o) ? "on" : ""}" data-act="pick" data-v="${o}"><span class="rad"></span><span>${M(r)}${i ? `<small>${M(i)}</small>` : ""}</span></button>`).join("")}</div>`,
    ),
      b("pick", async (o) => {
        F(), await s(o.dataset.v);
      });
  }
  b("ttlDefault", () =>
    To(
      "Mensagens tempor\xE1rias",
      "Vale para conversas que voc\xEA come\xE7ar a partir de agora.",
      [
        [0, "Desligadas"],
        [3600, "1 hora"],
        [86400, "1 dia"],
        [604800, "7 dias"],
      ],
      h.engine.settings.ttlDefault,
      async (t) => {
        await h.engine.setSetting("ttlDefault", +t), Ca();
      },
    ),
  );
  b("notifPreview", () =>
    To(
      "Pr\xE9via nas notifica\xE7\xF5es",
      "O que aparece na tela de bloqueio quando chega uma mensagem.",
      [
        ["full", "Nome e mensagem"],
        ["name", "S\xF3 o nome"],
        ["none", "Nada", 'Mostra apenas "Nova mensagem"'],
      ],
      h.engine.settings.notifPreview,
      async (t) => {
        await h.engine.setSetting("notifPreview", t), Ca();
      },
    ),
  );
  b("callsFrom", () =>
    To(
      "Quem pode me ligar",
      "",
      [
        ["contacts", "Meus contatos"],
        ["nobody", "Ningu\xE9m", "Chamadas recebidas s\xE3o ignoradas"],
      ],
      h.engine.settings.callsFrom,
      async (t) => {
        await h.engine.setSetting("callsFrom", t), Ca();
      },
    ),
  );
  async function Xi(t, e) {
    let a = await crypto.subtle.importKey(
        "raw",
        new TextEncoder().encode(t),
        "PBKDF2",
        !1,
        ["deriveBits"],
      ),
      n = await crypto.subtle.deriveBits(
        { name: "PBKDF2", hash: "SHA-256", salt: e, iterations: 15e4 },
        a,
        256,
      );
    return [...new Uint8Array(n)]
      .map((s) => s.toString(16).padStart(2, "0"))
      .join("");
  }
  var yf = (t) => new Uint8Array(t.match(/../g).map((e) => parseInt(e, 16)));
  async function Bo() {
    return !!(await h.engine.kv("lock", null));
  }
  function Qi(t, e) {
    return `<div style="display:grid;justify-items:center;gap:6px">${qe(56)}<h2 style="font:700 21px/1.2 var(--font-display);margin-top:12px;text-align:center">${M(t)}</h2>
    <p style="color:var(--fg2);font-size:14px;text-align:center" id="padsub">${M(e || "")}</p>
    <div class="pindots" id="pdots">${"<i></i>".repeat(6)}</div>
    <div class="keypad">${[1, 2, 3, 4, 5, 6, 7, 8, 9].map((a) => `<button class="key" data-act="pk" data-n="${a}">${a}</button>`).join("")}<span></span><button class="key" data-act="pk" data-n="0">0</button><button class="key ghost" data-act="pkDel" aria-label="Apagar">${x("back")}</button></div></div>`;
  }
  var fe = null;
  function Mo() {
    oe("#pdots i").forEach((t, e) =>
      t.classList.toggle("on", e < fe.buf.length),
    );
  }
  b("pk", (t) => {
    if (
      fe &&
      !(fe.buf.length >= 6) &&
      ((fe.buf += t.dataset.n), Mo(), X.vibrate(6), fe.buf.length >= fe.len)
    ) {
      let e = fe.buf;
      setTimeout(() => fe && fe.onDone(e), 120);
    }
  });
  b("pkDel", () => {
    fe && ((fe.buf = fe.buf.slice(0, -1)), Mo());
  });
  function Lo(t) {
    let e = y("#pdots");
    e &&
      (e.classList.remove("bad"),
      e.offsetWidth,
      e.classList.add("bad"),
      (fe.buf = ""),
      Mo(),
      X.vibrate(120),
      t && (y("#padsub").textContent = t));
  }
  function Ro(t) {
    return new Promise(async (e) => {
      let a = await h.engine.kv("lock", null);
      if (!a) return e(!0);
      ee(
        Qi("Digite seu PIN", t) +
          '<button class="linkbtn" data-act="pinForgotS" style="display:block;margin:6px auto 0">Esqueci o PIN</button>',
        {
          onClose: () => {
            (fe = null), e(!1);
          },
        },
      ),
        (fe = {
          buf: "",
          len: a.len || 4,
          onDone: async (n) => {
            (await Xi(n, yf(a.salt))) === a.hash
              ? ((fe = null), e(!0), F())
              : Lo("PIN incorreto. Tente de novo.");
          },
        });
    });
  }
  b("pinSetup", async () => {
    let t = h.engine;
    ((await Bo()) && !(await Ro("Confirme o PIN atual"))) ||
      setTimeout(() => {
        let e = "";
        ee(
          Qi(
            "Crie um PIN de 4 d\xEDgitos",
            "Ele \xE9 pedido ao abrir o app. Se esquecer, s\xF3 restaurando pelas 12 palavras.",
          ),
          {
            onClose: () => {
              fe = null;
            },
          },
        ),
          (fe = {
            buf: "",
            len: 4,
            onDone: async (a) => {
              if (!e) {
                (e = a),
                  (fe.buf = ""),
                  Mo(),
                  (y("#padsub").textContent = "Digite de novo para confirmar");
                return;
              }
              if (a !== e) {
                (e = ""), Lo("Os PINs n\xE3o batem. Comece de novo.");
                return;
              }
              let n = crypto.getRandomValues(new Uint8Array(16)),
                s = await Xi(a, n),
                o = await t.kv("lock", null);
              await t.setKv("lock", {
                salt: [...n]
                  .map((r) => r.toString(16).padStart(2, "0"))
                  .join(""),
                hash: s,
                len: 4,
                delay: o ? o.delay : 60,
              }),
                (fe = null),
                F(),
                _("PIN ativado"),
                Ca();
            },
          });
      }, 300);
  });
  b("pinRemove", async () => {
    (await Ro("Digite o PIN atual para tirar a senha")) &&
      (await h.engine.setKv("lock", null),
      _("PIN desligado. O app abre sem senha."),
      Ca());
  });
  b("pinToggle", async () => {
    (await Bo()) ? vt.pinRemove() : vt.pinSetup();
  });
  b("pinDelay", async () => {
    let t = await h.engine.kv("lock", null);
    t &&
      To(
        "Pedir o PIN",
        "",
        [
          [0, "Imediatamente"],
          [60, "Ap\xF3s 1 minuto fora do app"],
          [300, "Ap\xF3s 5 minutos"],
          [900, "Ap\xF3s 15 minutos"],
        ],
        t.delay || 0,
        async (e) => {
          (t.delay = +e), await h.engine.setKv("lock", t), Ca();
        },
      );
  });
  var os = !1;
  async function rs() {
    let t = await h.engine.kv("lock", null);
    if (!t || os) return !1;
    os = !0;
    let e = y("#lock");
    (e.innerHTML =
      Qi("J-Menseger bloqueado", "Digite seu PIN") +
      '<button class="linkbtn" data-act="pinForgot" style="margin-top:8px">Esqueci o PIN</button>'),
      (e.hidden = !1);
    let a = 0;
    return (
      (fe = {
        buf: "",
        len: t.len || 4,
        onDone: async (n) => {
          if ((await Xi(n, yf(t.salt))) === t.hash) {
            vf();
            return;
          }
          if ((a++, a >= 5)) {
            Lo("Muitas tentativas. Espere 30 segundos.");
            let s = fe;
            (fe = null),
              setTimeout(() => {
                (fe = s), (a = 3);
                let o = y("#padsub");
                o && (o.textContent = "Digite seu PIN");
              }, 3e4);
          } else Lo("PIN incorreto");
        },
      }),
      !0
    );
  }
  function vf() {
    (fe = null), (os = !1);
    let t = y("#lock");
    (t.hidden = !0), (t.innerHTML = "");
  }
  function V0() {
    return `<div class="forgot">${qe(48)}<h2>Esqueceu o PIN?</h2>
    <p>Digite as 12 palavras de recupera\xE7\xE3o desta conta (ou a chave que come\xE7a com nsec1). O PIN ser\xE1 desligado e o app abre normalmente.</p>
    <textarea class="inp mono" id="fgwords" rows="4" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="palavra1 palavra2 palavra3 \u2026"></textarea>
    <div class="errtx" id="fgerr" hidden></div>
    <button class="btn" data-act="pinForgotGo">Tirar o PIN e entrar</button>
    <button class="btn ghost" data-act="pinForgotBack">Voltar</button>
    <button class="linkbtn danger" data-act="pinForgotWipe">N\xE3o tenho as palavras</button></div>`;
  }
  b("pinForgot", () => {
    let t = y("#lock");
    (fe = null),
      (t.innerHTML = V0()),
      setTimeout(() => {
        let e = y("#fgwords");
        e && e.focus();
      }, 200);
  });
  b("pinForgotS", () => {
    F(), (os = !1), rs().then(() => vt.pinForgot());
  });
  b("pinForgotBack", async () => {
    (os = !1), await rs();
  });
  b("pinForgotGo", async () => {
    let t = y("#fgwords").value,
      e = y("#fgerr");
    if (!h.engine.verifyRecovery(t)) {
      (e.textContent =
        "Essas palavras n\xE3o s\xE3o desta conta. Confira a ordem e a grafia."),
        (e.hidden = !1);
      return;
    }
    await h.engine.setKv("lock", null),
      vf(),
      _(
        "PIN desligado. Se quiser, crie outro em Ajustes \u2192 Privacidade.",
        3600,
      );
  });
  b("pinForgotWipe", async () => {
    let t = y("#lock");
    t.innerHTML = `<div class="forgot">${qe(48)}<h2>Sem as palavras</h2>
    <p>Sem o PIN e sem as 12 palavras n\xE3o h\xE1 como abrir esta conta, nem mesmo para n\xF3s. A \xFAnica sa\xEDda \xE9 apagar o app deste aparelho e criar uma conta nova. As conversas daqui ser\xE3o perdidas.</p>
    <button class="btn danger" data-act="pinWipeGo">Apagar tudo deste aparelho</button>
    <button class="btn ghost" data-act="pinForgot">Voltar</button></div>`;
  });
  b("pinWipeGo", async (t) => {
    t.classList.add("busy"),
      (t.textContent = "Apagando\u2026"),
      X.setBackground(!1),
      await h.engine.wipe();
    try {
      let e = localStorage.getItem("jm.theme");
      localStorage.clear(), e && localStorage.setItem("jm.theme", e);
    } catch {}
    location.reload();
  });
  async function wf() {
    let t = await h.engine.kv("lock", null);
    !t ||
      !h.hiddenAt ||
      (Date.now() - h.hiddenAt >= (t.delay || 0) * 1e3 && rs());
  }
  b("openNotifs", () => {
    J.open(
      "notifs",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Notifica\xE7\xF5es</h2></div><div class="scroll" id="notbody"></div></div>`,
    ),
      kf();
  });
  function kf() {
    let t = h.engine.settings,
      e = y("#notbody");
    e &&
      (e.innerHTML = `<div style="height:8px"></div>
    ${le({ i: "bell", n: "Notifica\xE7\xF5es", s: "Avisos de mensagens e chamadas", act: "tglN", data: 'data-k="notifications"', sw: t.notifications })}
    ${le({ i: "refresh", n: "Manter conectado em segundo plano", s: "Recebe mensagens e chamadas com o app fechado. Mostra um aviso fixo discreto.", act: "tglN", data: 'data-k="background"', sw: t.background })}
    ${le({ i: "volume", n: "Sons no app", act: "tglN", data: 'data-k="sounds"', sw: t.sounds })}
    ${le({ i: "phone", n: "Vibrar", act: "tglN", data: 'data-k="vibrate"', sw: t.vibrate })}
    <div class="label">Android</div>
    ${le({ i: "battery", n: "Permitir uso em segundo plano", s: "Evita que o sistema pause o app para economizar bateria", act: "batterySettings" })}
    ${jmFullScreenOk() ? "" : le({ i: "phone", n: "Chamadas em tela cheia", s: "Desligado: com o celular bloqueado, a chamada chega s\xF3 como notifica\xE7\xE3o. Toque para permitir.", act: "fullScreenSettings", dot: !0 })}
    ${le({ i: "sliders", n: "Configura\xE7\xF5es do app no Android", s: "Permiss\xF5es de c\xE2mera, microfone e notifica\xE7\xF5es", act: "appSettings" })}
    <p class="note">Sem servidores de notifica\xE7\xE3o do Google: o pr\xF3prio app mant\xE9m a conex\xE3o cifrada quando a op\xE7\xE3o de segundo plano est\xE1 ligada.</p>`);
  }
  b("tglN", async (t) => {
    let e = t.dataset.k,
      a = h.engine,
      n = !a.settings[e];
    await a.setSetting(e, n),
      e === "notifications" && n && X.requestNotifications(),
      X.setBackground(a.settings.notifications && a.settings.background),
      kf();
  });
  b("batterySettings", () => X.batterySettings());
  function jmFullScreenOk() {
    try {
      return !window.JMNative || !window.JMNative.fullScreenAllowed || window.JMNative.fullScreenAllowed();
    } catch {
      return !0;
    }
  }
  b("fullScreenSettings", () => {
    try {
      window.JMNative.fullScreenSettings();
    } catch {}
  });
  /* ---------- Voz e áudio: microfone e ajustes de som das chamadas ---------- */
  var jmAudio = { deviceId: "", ec: !0, ns: !0, agc: !0 },
    jmMic = null;
  function jmApplyAudio() {
    ($i.echoCancellation = jmAudio.ec), ($i.noiseSuppression = jmAudio.ns), ($i.autoGainControl = jmAudio.agc);
    jmAudio.deviceId ? ($i.deviceId = { ideal: jmAudio.deviceId }) : delete $i.deviceId;
  }
  async function jmAudioInit(t) {
    Object.assign(jmAudio, (await t.kv("audioPrefs", null)) || {}), jmApplyAudio();
  }
  async function jmAudioSave() {
    jmApplyAudio(), await h.engine.setKv("audioPrefs", jmAudio);
  }
  function jmMicStop() {
    jmMic && (cancelAnimationFrame(jmMic.raf), jmMic.stream.getTracks().forEach((t) => t.stop()), jmMic.ctx.close().catch(() => {}), jmMic.audio && (jmMic.audio.srcObject = null)), (jmMic = null);
  }
  async function jmAudioPage() {
    let t = y("#audbody");
    if (!t) return;
    let e = [];
    try {
      e = (await navigator.mediaDevices.enumerateDevices()).filter((n) => n.kind === "audioinput");
    } catch {}
    let a = e.some((n) => n.label);
    t.innerHTML = `<div style="height:8px"></div>
    <div class="label">Microfone</div>
    <div class="opts" style="padding:0 18px">${[{ deviceId: "", label: "Padr\xE3o do sistema" }, ...e.filter((n) => n.deviceId && n.deviceId !== "default")]
      .map((n, s) => `<button class="opt ${jmAudio.deviceId === n.deviceId ? "on" : ""}" data-act="audDev" data-id="${M(n.deviceId)}"><span class="rad"></span>${M(n.label || "Microfone " + s)}</button>`)
      .join("")}</div>
    ${a ? "" : '<p class="note">Os nomes dos microfones aparecem depois do primeiro teste (o Android pede permiss\xE3o).</p>'}
    <div class="label">Teste</div>
    <div style="padding:0 18px"><div class="meter"><i id="audlvl"></i></div>
    <div class="two" style="margin-top:12px"><button class="btn ${jmMic ? "" : "ghost"}" data-act="audTest" style="margin:0">${x("mic", "sm")}&nbsp;${jmMic ? "Parar teste" : "Testar microfone"}</button><button class="btn ghost" data-act="audEcho" style="margin:0" ${jmMic ? "" : "disabled"}>${x("volume", "sm")}&nbsp;${jmMic && jmMic.audio ? "Parar de me ouvir" : "Ouvir minha voz"}</button></div>
    <p class="note" style="padding:8px 0 0" id="audmsg">Fale algo: a barra deve se mexer. Para se ouvir, use fone.</p></div>
    <div class="label">Processamento da voz</div>
    ${le({ i: "wave", n: "Redu\xE7\xE3o de ru\xEDdo", s: "Diminui barulho de fundo", act: "audTgl", data: 'data-k="ns"', sw: jmAudio.ns })}
    ${le({ i: "volume", n: "Cancelamento de eco", s: "Evita que o outro lado se ou\xE7a pelo seu alto-falante", act: "audTgl", data: 'data-k="ec"', sw: jmAudio.ec })}
    ${le({ i: "sliders", n: "Ajuste autom\xE1tico de volume", s: "Equilibra voz baixa e alta", act: "audTgl", data: 'data-k="agc"', sw: jmAudio.agc })}
    <p class="note">Valem para as pr\xF3ximas chamadas e salas de voz. Se o outro lado n\xE3o te ouve: teste aqui, escolha outro microfone e confira a permiss\xE3o de microfone do app.</p>
    ${le({ i: "sliders", n: "Permiss\xF5es do app no Android", s: "Microfone, c\xE2mera e notifica\xE7\xF5es", act: "appSettings" })}
    <div style="height:40px"></div>`;
  }
  b("openAudio", () => {
    J.open(
      "audio",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Voz e \xE1udio</h2></div><div class="scroll" id="audbody"></div></div>`,
      { onClose: jmMicStop },
    ),
      jmAudioPage();
  });
  b("audDev", async (t) => {
    (jmAudio.deviceId = t.dataset.id || ""), await jmAudioSave(), jmMic && (jmMicStop(), await vt.audTest()), jmAudioPage();
  });
  b("audTgl", async (t) => {
    let e = t.dataset.k;
    (jmAudio[e] = !jmAudio[e]), await jmAudioSave(), jmAudioPage();
  });
  b("audTest", async () => {
    if (jmMic) {
      jmMicStop(), jmAudioPage();
      return;
    }
    try {
      let t = await navigator.mediaDevices.getUserMedia({ audio: { ...$i } }),
        e = new AudioContext(),
        a = e.createAnalyser();
      (a.fftSize = 512), e.createMediaStreamSource(t).connect(a);
      let n = new Uint8Array(a.fftSize);
      jmMic = { stream: t, ctx: e, raf: 0, audio: null };
      let s = () => {
        if (!jmMic) return;
        a.getByteTimeDomainData(n);
        let o = 0;
        for (let i of n) o = Math.max(o, Math.abs(i - 128));
        let r = y("#audlvl");
        r && (r.style.width = Math.min(100, (o / 128) * 160).toFixed(0) + "%"), (jmMic.raf = requestAnimationFrame(s));
      };
      s(), await jmAudioPage();
    } catch (t) {
      jmMic = null;
      let e = y("#audmsg");
      e && ((e.textContent = ze(t)), e.classList.add("errtx"));
    }
  });
  b("audEcho", () => {
    if (jmMic) {
      if (jmMic.audio) (jmMic.audio.srcObject = null), (jmMic.audio = null);
      else {
        let t = new Audio();
        (t.srcObject = jmMic.stream), t.play().catch(() => {}), (jmMic.audio = t);
      }
      jmAudioPage();
    }
  });
  b("appSettings", () => X.appSettings());
  b("openAppearance", () => {
    J.open(
      "appearance",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Apar\xEAncia</h2></div>
    <div class="scroll">${io(!0)}<button class="btn ghost reset" data-act="resetColors">Voltar ao padr\xE3o (verde e preto)</button></div></div>`,
      { onClose: Xt },
    ),
      va();
  });
  b("resetColors", () => {
    Zd(), _("Cores restauradas");
  });
  b("openNetwork", () => {
    J.open(
      "network",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Rede e servidores</h2></div><div class="scroll" id="netbody"></div></div>`,
      { onClose: Xt },
    ),
      un();
  });
  function un() {
    let t = h.engine,
      e = y("#netbody");
    if (!e) return;
    let a = t.pool ? t.pool.status() : [],
      n = {
        open: "Conectado",
        connecting: "Conectando\u2026",
        closed: "Sem conex\xE3o",
        idle: "Aguardando",
      },
      s = t.settings.turn || {};
    e.innerHTML = `<p class="note" style="padding-top:4px">As mensagens viajam por servidores p\xFAblicos da rede Nostr, sempre cifradas. Usar v\xE1rios ao mesmo tempo garante entrega mesmo se um cair. Seus contatos precisam ter pelo menos um servidor em comum com voc\xEA.</p>
    <div class="label">Servidores de mensagens</div>
    ${a.map((o) => `<div class="srow" style="cursor:default"><span class="rdot ${o.status}"></span><span class="sm"><b style="font-size:14.5px;word-break:break-all">${M(o.url.replace("wss://", ""))}</b><small>${n[o.status] || o.status}</small></span><button class="ibtn sm" data-act="relayDel" data-u="${M(o.url)}" aria-label="Remover servidor">${x("x", "sm")}</button></div>`).join("")}
    <div style="padding:6px 18px 0;display:flex;gap:8px"><input class="inp mono" id="relayin" placeholder="wss://seu-servidor" autocapitalize="off" spellcheck="false" style="height:46px"><button class="mini" data-act="relayAdd">Adicionar</button></div>
    <div style="padding:0 18px"><button class="btn ghost" data-act="relayReset">Restaurar servidores padr\xE3o</button></div>
    <div class="label">Chamadas (TURN, opcional)</div>
    <p class="note" style="padding-top:0">Chamadas v\xE3o direto entre os aparelhos. Em algumas redes m\xF3veis isso falha; um servidor TURN resolve e tamb\xE9m permite ocultar seu IP. O \xE1udio e o v\xEDdeo continuam cifrados.</p>
    <div style="padding:8px 18px 0;display:grid;gap:8px">
      <input class="inp mono" id="turnurl" placeholder="turn:servidor.com:3478" value="${M(s.url || "")}" autocapitalize="off" spellcheck="false">
      <div class="two"><input class="inp" id="turnuser" placeholder="Usu\xE1rio" value="${M(s.user || "")}" autocapitalize="off"><input class="inp" id="turnpass" type="password" placeholder="Senha" value="${M(s.pass || "")}"></div>
      <button class="btn" data-act="turnSave" style="margin-top:4px">Salvar TURN</button></div>
    <div style="height:40px"></div>`;
  }
  b("relayDel", async (t) => {
    let e = h.engine,
      a = e.settings.relays.filter((n) => n !== t.dataset.u);
    if (!a.length) return _("Mantenha pelo menos um servidor");
    await e.setSetting("relays", a), un();
  });
  b("relayAdd", async () => {
    let t = ha(y("#relayin").value);
    if (!t || !t.startsWith("wss://"))
      return (
        y("#relayin").classList.add("err"),
        _("Use um endere\xE7o wss:// v\xE1lido")
      );
    let e = h.engine;
    e.settings.relays.includes(t) ||
      (await e.setSetting("relays", [...e.settings.relays, t].slice(0, 10))),
      un(),
      _("Servidor adicionado");
  });
  b("relayReset", async () => {
    await h.engine.setSetting("relays", li),
      un(),
      _("Servidores padr\xE3o restaurados");
  });
  b("turnSave", async () => {
    let t = y("#turnurl").value.trim();
    if (t && !/^turns?:/i.test(t))
      return _("O endere\xE7o deve come\xE7ar com turn: ou turns:");
    await h.engine.setSetting("turn", {
      url: t,
      user: y("#turnuser").value.trim(),
      pass: y("#turnpass").value,
    }),
      t || (await h.engine.setSetting("forceRelay", !1)),
      _(t ? "TURN salvo" : "TURN removido");
  });
  b("openStorage", async () => {
    let t = "";
    try {
      t =
        ((await navigator.storage.estimate()).usage / 1048576).toFixed(1) +
        " MB";
    } catch {
      t = "\u2014";
    }
    let a = (await h.engine.db.all("media"))
      .filter((n) => n.blob)
      .reduce((n, s) => n + (s.blob.size || 0), 0);
    J.open(
      "storage",
      `<div class="sc-top"><div class="subhead"><button class="ibtn" data-act="back" aria-label="Voltar">${x("back")}</button><h2>Armazenamento</h2></div>
    <div class="scroll"><div class="label">Backup</div>
      ${le({ i: "download", n: "Fazer backup das conversas", s: "Arquivo cifrado com a chave da sua conta: s\xF3 abre com as suas palavras. Fotos e \xE1udios n\xE3o entram.", act: "backupExport" })}
      ${le({ i: "refresh", n: "Restaurar backup", s: "Junta mensagens, contatos e grupos do arquivo com os daqui. Nada \xE9 apagado.", act: "backupImport" })}
      <div class="label">Neste aparelho</div>
      ${le({ i: "db", n: "Espa\xE7o usado pelo app", act: "noop", val: t })}
      ${le({ i: "image", n: "Fotos e \xE1udios", act: "noop", val: (a / 1048576).toFixed(1) + " MB" })}
      <div style="padding:0 18px"><button class="btn ghost" data-act="clearMedia">Apagar fotos e \xE1udios deste aparelho</button></div>
      <p class="note">As mensagens de texto continuam. Fotos e \xE1udios apagados n\xE3o podem ser baixados de novo.</p></div></div>`,
    );
  });
  b("noop", () => {});
  b("backupExport", async () => {
    _("Preparando o backup\u2026", 6e3);
    try {
      let t = await h.engine.exportBackup(),
        e = `J-Menseger backup ${new Date().toISOString().slice(0, 10)}.jmbackup`,
        a = X.saveFile(e, Fa.enc(t));
      if (a === void 0) {
        let n = document.createElement("a");
        (n.href = URL.createObjectURL(new Blob([t], { type: "application/octet-stream" }))), (n.download = e), n.click(), setTimeout(() => URL.revokeObjectURL(n.href), 5e3), _("Backup salvo");
      } else a ? ee(`<h2 class="sh-title">Backup salvo</h2><p class="sh-sub">Est\xE1 em <b>${M(a)}</b>. Guarde junto com as suas palavras de recupera\xE7\xE3o: sem elas o arquivo n\xE3o abre.</p><button class="btn" data-act="closeSheet">Entendi</button>`) : _("N\xE3o foi poss\xEDvel salvar o backup.");
    } catch (t) {
      _(t.message || "N\xE3o foi poss\xEDvel fazer o backup.");
    }
  });
  b("backupImport", async () => {
    let t = await Kn(".jmbackup,application/octet-stream,*/*");
    if (t) {
      _("Restaurando\u2026", 8e3);
      try {
        let e = await h.engine.importBackup(new Uint8Array(await t.arrayBuffer()));
        _(`Backup restaurado: ${e.messages} mensagens, ${e.contacts} contatos e ${e.groups} grupos novos`, 4e3), Ae();
      } catch (e) {
        _(e.message || "N\xE3o foi poss\xEDvel restaurar.", 4e3);
      }
    }
  });
  b("clearMedia", async () => {
    if (
      !(await ge({
        title: "Apagar fotos e \xE1udios?",
        text: "Os textos continuam. A m\xEDdia n\xE3o volta.",
        ok: "Apagar",
        danger: !0,
      }))
    )
      return;
    let t = h.engine;
    for (let e of await t.db.all("media")) await t.db.del("media", e.mid);
    for (let e of await t.db.all("msgs"))
      e.media &&
        e.media.ready &&
        ((e.media.ready = !1), (e.media.err = !0), await t.db.put("msgs", e));
    J.pop(), _("M\xEDdia apagada");
  });
  // ---- Atualização automática (o download e a instalação ficam no lado Android) ----
  var jmUpd = { info: null, prog: -1, manual: !1, avisada: "" };
  function jmUpdSheet() {
    let t = jmUpd.info;
    if (!t) return;
    let e = t.tamanho ? ` \xB7 ${(t.tamanho / 1048576).toFixed(1)} MB` : "";
    ee(`<h2 class="sh-title">Atualiza\xE7\xE3o dispon\xEDvel</h2><p class="sh-sub">Vers\xE3o ${M(t.versao)}${e} (voc\xEA tem a ${M(h.version)})</p>
    ${t.novidades ? `<p class="sh-sub" style="white-space:pre-line">${M(t.novidades)}</p>` : ""}
    <div class="upd-bar" id="updbar" ${jmUpd.prog < 0 ? "hidden" : ""}><i style="width:${Math.max(0, jmUpd.prog)}%"></i></div>
    <p class="sh-sub" id="updtxt">${jmUpd.prog < 0 ? "O arquivo \xE9 conferido antes de instalar. O Android vai pedir para voc\xEA confirmar." : jmUpd.prog >= 100 ? "Abrindo o instalador\u2026" : `Baixando\u2026 ${jmUpd.prog}%`}</p>
    <div class="two"><button class="btn ghost" data-act="updLater" style="margin:0">Depois</button><button class="btn" data-act="updGo" id="updgo" style="margin:0" ${jmUpd.prog >= 0 && jmUpd.prog < 100 ? "disabled" : ""}>${x("download", "sm")}&nbsp;Atualizar</button></div>`);
  }
  function jmUpdFound(t) {
    let e = jmUpd.manual;
    if (((jmUpd.manual = !1), (jmUpd.info = t || null), !t)) {
      e && _("Voc\xEA j\xE1 tem a vers\xE3o mais nova");
      return;
    }
    (e || jmUpd.avisada !== t.versao) && ((jmUpd.avisada = t.versao), jmUpdSheet());
  }
  function jmUpdProgress(t) {
    jmUpd.prog = t;
    let e = y("#updbar"),
      a = y("#updtxt"),
      n = y("#updgo");
    e && ((e.hidden = !1), (e.firstElementChild.style.width = t + "%")),
      a && (a.textContent = t >= 100 ? "Abrindo o instalador\u2026" : `Baixando\u2026 ${t}%`),
      n && (n.disabled = t < 100),
      t >= 100 && setTimeout(() => (jmUpd.prog = -1), 3e3);
  }
  function jmUpdError(t) {
    jmUpd.prog = -1;
    let e = y("#updgo"),
      a = y("#updtxt");
    e && (e.disabled = !1), a && (a.textContent = t), _(t, 4e3);
  }
  b("updOpen", () => {
    jmUpd.info ? jmUpdSheet() : ((jmUpd.manual = !0), _("Procurando atualiza\xE7\xF5es\u2026"), X.updateCheck());
  });
  b("updLater", () => F());
  b("updGo", () => {
    jmUpd.prog < 0 && ((jmUpd.prog = 0), jmUpdProgress(0)), X.updateDownload();
  });
  b("openAbout", () => {
    ee(`<div class="about">${qe(96)}<h3><span>J-</span>Menseger</h3><p>Vers\xE3o ${M(h.version)} \xB7 beta para testes</p>
    <div class="sizes">${qe(64)}${qe(44)}${qe(30)}</div></div>
    <div class="label" style="padding-left:0">Como sua privacidade \xE9 protegida</div>
    <p class="sh-sub">\u2022 Sua conta \xE9 um par de chaves criado no seu aparelho. N\xE3o pedimos telefone, e-mail nem agenda.<br>
    \u2022 Cada mensagem \xE9 cifrada (NIP-44) e lacrada em um envelope com chave descart\xE1vel (NIP-59). Os servidores n\xE3o sabem quem enviou nem o que diz.<br>
    \u2022 Chamadas v\xE3o direto entre os aparelhos, cifradas com DTLS-SRTP.<br>
    \u2022 Fotos perdem os metadados (como localiza\xE7\xE3o) antes do envio.<br>
    \u2022 A chave privada fica guardada com o Android Keystore. Sem an\xFAncios e sem rastreamento.</p>
    <div class="label" style="padding-left:0">C\xF3digo aberto usado</div>
    <p class="sh-sub">nostr-tools, @noble/curves, @noble/hashes, @noble/ciphers e @scure (MIT) \xB7 qrcode-generator (MIT) \xB7 jsQR (Apache 2.0) \xB7 fontes Bricolage Grotesque e Figtree (OFL).</p>
    <div class="label" style="padding-left:0">Sons do soundboard</div>
    <p class="sh-sub">Feitos com grava\xE7\xF5es reais de instrumentos e plateia dos bancos MuseScore General (S. Christian Collins) e FluidR3 GM (Frank Wen), licen\xE7a MIT, com amostras CC0 do VSCO e do MuseScore Drumline. O som de grilos \xE9 sintetizado.</p>`);
  });
  b("wipeAll", async () => {
    let t = await h.engine.kv("wordsPending", !1);
    if (
      (await ge({
        title: "Sair e apagar tudo?",
        text:
          (t
            ? "Voc\xEA ainda n\xE3o confirmou que anotou suas 12 palavras. "
            : "") +
          "A conta e as conversas somem deste aparelho. S\xF3 d\xE1 para voltar com as 12 palavras de recupera\xE7\xE3o.",
        ok: "Apagar tudo",
        danger: !0,
      })) &&
      !((await Bo()) && !(await Ro("Confirme o PIN para apagar")))
    ) {
      P.busyCall && P.busyCall() && P.leaveRoom && P.leaveRoom(),
        X.setBackground(!1),
        await h.engine.wipe();
      try {
        let a = localStorage.getItem("jm.theme");
        localStorage.clear(), a && localStorage.setItem("jm.theme", a);
      } catch {}
      location.reload();
    }
  });
  var Q = {
      flow: "create",
      step: "welcome",
      name: "",
      note: "",
      t: Math.floor(Math.random() * 6),
      words: "",
      restore: "",
      saved: !1,
    },
    G0 = {
      create: ["name", "colors", "words", "notify"],
      restore: ["restore", "rname", "notify"],
    },
    Ef = {
      name: "welcome",
      colors: "name",
      words: "colors",
      restore: "welcome",
      rname: "restore",
    };
  function $f() {
    let t = y("#auth");
    (t.hidden = !1),
      t.classList.add("entering"),
      setTimeout(() => t.classList.remove("entering"), 700),
      it("welcome", 0);
  }
  function K0(t) {
    return t === "welcome"
      ? `<div class="abody">${qe(112, !0)}
      <h1 class="wmark"><span>J-</span>Menseger</h1>
      <p class="wtag">Conversas, grupos e salas de voz em um s\xF3 lugar, cifrados de ponta a ponta.</p>
      <ul class="wfeat">
        <li style="--i:0"><span class="fi">${x("shield")}</span>Sem telefone nem e-mail: sua conta \xE9 uma chave que fica no seu aparelho</li>
        <li style="--i:1"><span class="fi">${x("users")}</span>Grupos com canais de texto e salas de voz</li>
        <li style="--i:2"><span class="fi">${x("wave")}</span>Chamadas com v\xEDdeo e soundboard</li>
      </ul></div>
    <div class="afoot"><button class="btn" data-act="auCreate">Criar minha conta</button>
      <button class="lnkbtn" data-act="auRestore">J\xE1 tenho uma conta? <b>Restaurar</b></button>
      <p class="afine">Vers\xE3o beta para testes. Nenhum dado pessoal \xE9 pedido nem enviado para servidores.</p></div>`
      : t === "name"
        ? `<div class="abody"><h1 class="ah">Como podemos te chamar?</h1>
      <p class="as">Esse nome s\xF3 aparece para quem voc\xEA adicionar e nos grupos em que voc\xEA entrar.</p>
      <div class="pavwrap" id="pavw">${et({ name: Q.name || "?", t: Q.t, size: 104 })}</div>
      <div class="tones">${[0, 1, 2, 3, 4, 5].map((e) => `<button class="t${e} ${e === Q.t ? "on" : ""}" style="--g:var(--g)" data-act="auTone" data-t="${e}" aria-label="Cor do avatar ${e + 1}"><span class="av t${e}" style="--sz:34px"></span></button>`).join("")}</div>
      <label class="lbl" for="aname">Seu nome</label>
      <input class="inp" id="aname" data-focus maxlength="40" placeholder="Ex.: Jo\xE3o Palhares" autocomplete="name" value="${M(Q.name)}">
      <label class="lbl" for="anote">Recado (opcional)</label>
      <input class="inp" id="anote" maxlength="80" placeholder="Dispon\xEDvel para conversar" value="${M(Q.note)}"></div>
    <div class="afoot"><button class="btn" id="anext" data-act="auNext" ${Q.name.trim().length >= 2 ? "" : "disabled"}>Continuar</button></div>`
        : t === "colors"
          ? `<div class="abody"><h1 class="ah">Deixe com a sua cara</h1>
      <p class="as">Escolha as cores do aplicativo. Tudo pode ser trocado depois.</p>
      ${io(!1)}<div style="height:12px"></div></div>
    <div class="afoot"><button class="btn" data-act="auNext">Continuar</button></div>`
          : t === "words"
            ? `<div class="abody"><h1 class="ah">Sua chave de recupera\xE7\xE3o</h1>
      <p class="as">Sem telefone nem e-mail, <b>estas 12 palavras s\xE3o a \xFAnica forma de recuperar sua conta</b> se voc\xEA trocar de celular. Anote no papel, na ordem. N\xE3o mande para ningu\xE9m.</p>
      <div class="wgrid selectable" id="wordsbox">${Q.words
        .split(" ")
        .map((a, n) => `<span style="--i:${n}"><i>${n + 1}</i>${M(a)}</span>`)
        .join("")}</div>
      <label class="check"><input type="checkbox" id="wsaved" ${Q.saved ? "checked" : ""}><span>Anotei as 12 palavras em um lugar seguro.</span></label></div>
    <div class="afoot"><button class="btn" id="anext" data-act="auNext" ${Q.saved ? "" : "disabled"}>Criar minha conta</button>
      <button class="lnkbtn" data-act="auSkipWords">Anotar depois (fica em Ajustes \u2192 Conta)</button></div>`
            : t === "restore"
              ? `<div class="abody"><h1 class="ah">Restaurar conta</h1>
      <p class="as">Digite as 12 palavras de recupera\xE7\xE3o, separadas por espa\xE7o. Tamb\xE9m aceitamos a chave secreta que come\xE7a com <b>nsec1</b>.</p>
      <label class="lbl" for="arestore">Palavras ou chave</label>
      <textarea class="inp mono" id="arestore" data-focus rows="4" spellcheck="false" autocapitalize="off" autocomplete="off" placeholder="abacate abaixo abalar\u2026">${M(Q.restore)}</textarea>
      <div class="errtx" id="aerr" hidden></div>
      <p class="note" style="padding:12px 0 0">As conversas guardadas nos servidores (at\xE9 cerca de um m\xEAs, enquanto n\xE3o expiram) voltam para este aparelho. Fotos e \xE1udios antigos podem n\xE3o voltar.</p></div>
    <div class="afoot"><button class="btn" id="anext" data-act="auNext">Continuar</button></div>`
              : t === "rname"
                ? `<div class="abody"><h1 class="ah">Confirme seu nome</h1>
      <p class="as">Chave reconhecida. Como voc\xEA quer aparecer para seus contatos?</p>
      <div class="pavwrap" id="pavw">${et({ name: Q.name || "?", t: Q.t, size: 104 })}</div>
      <label class="lbl" for="aname">Seu nome</label>
      <input class="inp" id="aname" data-focus maxlength="40" placeholder="Seu nome" value="${M(Q.name)}">
      <label class="lbl" for="anote">Recado (opcional)</label>
      <input class="inp" id="anote" maxlength="80" placeholder="Dispon\xEDvel para conversar" value="${M(Q.note)}"></div>
    <div class="afoot"><button class="btn" id="anext" data-act="auNext" ${Q.name.trim().length >= 2 ? "" : "disabled"}>Restaurar conta</button></div>`
                : t === "notify"
                  ? `<div class="abody"><div class="permcard"><span class="big">${x("bell")}</span>
      <h1 class="ah" style="margin-top:6px">Receber avisos?</h1>
      <p class="as" style="margin-left:auto;margin-right:auto">Para saber de novas mensagens e chamadas com o app fechado. Por padr\xE3o o aviso mostra s\xF3 o nome de quem escreveu, nunca o texto.</p></div></div>
    <div class="afoot"><button class="btn" data-act="auNotifyYes">Ativar notifica\xE7\xF5es</button><button class="lnkbtn" data-act="auNotifyNo">Agora n\xE3o</button></div>`
                  : "";
  }
  function it(t, e) {
    let a = y("#asteps"),
      n = y(".astep", a),
      s = document.createElement("div");
    (s.className = "astep " + t + (e > 0 ? " from-r" : e < 0 ? " from-l" : "")),
      (s.innerHTML = K0(t)),
      a.appendChild(s),
      n &&
        (n.classList.add(e >= 0 ? "to-l" : "to-r"),
        setTimeout(() => n.remove(), 460)),
      (Q.step = t);
    let o = G0[Q.flow],
      r = o.indexOf(t);
    y("#auth").classList.toggle("nobar", r < 0),
      (y("#aprog").innerHTML = o
        .map((i, c) => `<i class="${c <= r ? "on" : ""}"></i>`)
        .join("")),
      t === "colors" && va(),
      setTimeout(() => {
        let i = s.querySelector("[data-focus]");
        i && Q.step === t && i.focus({ preventScroll: !0 });
      }, 480);
  }
  b("auCreate", () => {
    (Q.flow = "create"),
      Q.words || (Q.words = h.engine.newWords()),
      it("name", 1);
  });
  b("auRestore", () => {
    (Q.flow = "restore"), it("restore", 1);
  });
  b("authBack", () => {
    let t = Ef[Q.step];
    t && it(t, -1);
  });
  b("auTone", (t) => {
    (Q.t = +t.dataset.t),
      oe(".tones button").forEach((a) => a.classList.toggle("on", a === t));
    let e = y("#pavw");
    e && (e.innerHTML = et({ name: Q.name || "?", t: Q.t, size: 104 }));
  });
  b("auSkipWords", async () => {
    (Q.saved = !1), await Af();
  });
  b("auNext", async (t) => {
    if (t.disabled || t.classList.contains("busy")) return;
    let e = Q.step;
    if (e === "name" || e === "rname")
      return Q.name.trim().length < 2
        ? void 0
        : e === "rname"
          ? F0(t)
          : it("colors", 1);
    if (e === "colors") return it("words", 1);
    if (e === "words") return Af(t);
    if (e === "restore") {
      let a = y("#arestore").value.trim();
      Q.restore = a;
      try {
        h.engine.skFromInput(a);
      } catch {
        let s = y("#aerr");
        (s.textContent =
          "N\xE3o reconhecemos essas palavras. Confira a ordem e a grafia (12 palavras)."),
          (s.hidden = !1),
          y("#arestore").classList.add("err"),
          setTimeout(
            () => y("#arestore") && y("#arestore").classList.remove("err"),
            500,
          );
        return;
      }
      return it("rname", 1);
    }
  });
  async function Af(t) {
    t && (t.classList.add("busy"), (t.textContent = "Criando\u2026")),
      await h.engine.createIdentity({
        name: Q.name.trim(),
        note: Q.note.trim(),
        words: Q.words,
        t: Q.t,
      }),
      Q.saved || (await h.engine.setKv("wordsPending", !0)),
      it("notify", 1);
  }
  async function F0(t) {
    t.classList.add("busy"), (t.textContent = "Restaurando\u2026");
    try {
      await h.engine.restoreIdentity(Q.restore, {
        name: Q.name.trim(),
        note: Q.note.trim(),
      }),
        (h.engine.me.t = Q.t),
        await h.engine.setKv("me", h.engine.me),
        (Q.restored = !0),
        it("notify", 1);
    } catch (e) {
      t.classList.remove("busy"),
        (t.textContent = "Restaurar conta"),
        _(e.message);
    }
  }
  b("auNotifyYes", () => {
    X.requestNotifications(), h.engine.setSetting("notifications", !0), Sf();
  });
  b("auNotifyNo", () => {
    h.engine.setSetting("notifications", !1), Sf();
  });
  function Sf() {
    let t = y("#auth"),
      e = y("#screen-main");
    t.classList.add("leaving"),
      e.classList.add("intro"),
      P.afterLogin({ restored: !!Q.restored }),
      setTimeout(() => {
        (t.hidden = !0),
          t.classList.remove("leaving"),
          t.classList.add("nobar"),
          e.classList.remove("intro"),
          (y("#asteps").innerHTML = "");
        let a = h.engine.me.name.split(" ")[0];
        _(
          Q.restored
            ? "Conta restaurada. Buscando suas conversas\u2026"
            : `Bem-vindo, ${a}`,
          3200,
        ),
          Object.assign(Q, {
            flow: "create",
            step: "welcome",
            name: "",
            note: "",
            words: "",
            restore: "",
            saved: !1,
            restored: !1,
          });
      }, 700);
  }
  document.addEventListener("input", (t) => {
    let e = t.target;
    if (e.id === "aname") {
      Q.name = e.value;
      let a = y("#pavw");
      a && (a.innerHTML = et({ name: Q.name || "?", t: Q.t, size: 104 }));
      let n = y("#anext");
      n && (n.disabled = Q.name.trim().length < 2);
    } else if (e.id === "anote") Q.note = e.value;
    else if (e.id === "arestore") {
      let a = y("#aerr");
      a && (a.hidden = !0);
    }
  });
  document.addEventListener("change", (t) => {
    if (t.target.id === "wsaved") {
      Q.saved = t.target.checked;
      let e = y("#anext");
      e && (e.disabled = !Q.saved);
    }
  });
  document.addEventListener("keydown", (t) => {
    if (
      t.key === "Enter" &&
      (t.target.id === "aname" || t.target.id === "anote")
    ) {
      let e = y("#anext");
      e && !e.disabled && e.click();
    }
  });
  function Cf() {
    if (y("#auth").hidden) return !1;
    let t = Ef[Q.step];
    return t ? (it(t, -1), !0) : !1;
  }
  Object.assign(P, {
    openChat: Zn,
    refreshChatHead: sn,
    currentChat: yo,
    ttlSheet: Di,
    soundSheet: Eu,
    openSounds: Li,
    refreshLists: Ae,
    setTab: Ea,
    saveInvites: go,
    newChat: qu,
    addContact: () => on(),
    myCode: Ni,
    openProfile: rn,
    openRequests: zu,
    openBlocked: _i,
    openGroup: Oi,
    newGroup: ji,
    refreshGroup: Be,
    groupAvatar: Pe,
    startCall: Yi,
    joinRoom: df,
    leaveRoom: ss,
    newCall: gf,
    callSound: of,
    inRoom: cf,
    inRoomAny: lf,
    busyCall: xf,
    renderSettings: Xt,
    afterLogin: (t) => Tf(t),
  });
  b("back", () => J.pop());
  b("closeSheet", () => F());
  var Lf;
  function W0(t, e, a, n) {
    let s = y("#inapp");
    (s.innerHTML = `${n}<div><b>${M(e)}</b><small>${M(a)}</small></div>`),
      (s.dataset.key = t),
      (s.hidden = !1),
      (s.style.animation = "none"),
      s.offsetWidth,
      (s.style.animation = ""),
      clearTimeout(Lf),
      (Lf = setTimeout(() => {
        s.hidden = !0;
      }, 3800));
  }
  y("#inapp").addEventListener("click", () => {
    let t = y("#inapp").dataset.key;
    (y("#inapp").hidden = !0), t && Zn(t);
  });
  function Y0(t, e, a) {
    let n = h.engine,
      s = n.settings;
    if (a && h.visible) return;
    let o = n.chats.get(t),
      jmMen = !!o && o.kind === "group" && e.kind === "text" && e.from !== n.pk && jmMentionsMe(e.text, o, e.from);
    if (!o || (o.muted && !jmMen)) return;
    let r, i, c;
    if (o.kind === "dm") {
      let d = n.contacts.get(o.pk) || {};
      if (d.status === "blocked") return;
      (i = ne(o.pk)),
        (c = pe(o.pk, 40, { badge: !1 })),
        (r = d.status === "request" ? "Novo pedido de mensagem" : i);
    } else {
      let d = n.groups.get(o.lid);
      if (!d || (d.mute && !jmMen)) return;
      let f = d.meta.channels.find((p) => p.id === o.ch);
      (i = ne(e.from)),
        (r = `${i} \xB7 ${d.meta.name}${f ? " #" + f.name : ""}`),
        (c = Pe(d, 40));
    }
    let l = n.preview(e);
    if (h.visible) {
      s.sounds && cu(),
        s.vibrate && X.vibrate(30),
        W0(t, r, s.notifPreview === "full" ? l : "Nova mensagem", c);
      return;
    }
    if (!s.notifications) return;
    let jmMeuSt = jmCleanStatus(n.me && n.me.st);
    if (jmMeuSt && jmMeuSt.dnd) return;
    let u = s.notifPreview;
    X.notify({
      tag: t,
      chat: t,
      title: u === "none" ? "J-Menseger" : r,
      body:
        u === "full"
          ? l
          : u === "name"
            ? "Nova mensagem"
            : "Voc\xEA tem uma nova mensagem",
      silent: !s.sounds,
      vibrate: !!s.vibrate,
      // Responder pela notificação: não com PIN (seria um atalho por cima do bloqueio).
      reply: !n.hasPin && (o.kind === "group" || (n.contacts.get(o.pk) || {}).status === "ok"),
      read: !n.hasPin,
    });
  }
  function Z0(t) {
    jmSpotifyInit(t), jmAudioInit(t);
    t.on("msg", (e, a) => {
      Iu(e, a),
        a.kind === "invite" &&
          a.from !== t.pk &&
          a.inv &&
          !t.groups.has(a.inv.lid) &&
          !h.invites.some((n) => n.msgId === a.id) &&
          (h.invites.push({
            msgId: a.id,
            chat: e,
            from: a.from,
            lid: a.inv.lid,
            meta: a.inv.meta,
          }),
          go()),
        Ae();
    }),
      t.on("msgUpdate", (e, a) => {
        Hu(e, a), Ae();
      }),
      t.on("msgDelete", (e, a) => {
        Uu(e, a), Ae();
      }),
      t.on("chat", () => Ae()),
      t.on("chats", () => Ae()),
      t.on("contact", () => {
        Ae(), sn();
      }),
      t.on("group", () => {
        Ae(), Be(), sn(), hf();
      }),
      t.on("chatGone", (e) => {
        let a = yo();
        a &&
          a.key === e &&
          (J.popTo("chat"), J.pop(), _("Este canal foi apagado por um admin."));
      }),
      t.on("synced", (e) => {
        _(`Seu outro aparelho mandou ${e.contacts} contatos e ${e.groups} grupos`, 3500), Ae(), Be();
      }),
      t.on("remoteLogout", async () => {
        P.busyCall && P.busyCall() && P.leaveRoom && P.leaveRoom(), X.setBackground(!1), await h.engine.wipe();
        try {
          let a = localStorage.getItem("jm.theme");
          localStorage.clear(), a && localStorage.setItem("jm.theme", a);
        } catch {}
        location.reload();
      }),
      t.on("pins", (e) => {
        C && C.key === e && jmPinBar();
      }),
      t.on("presence", (e) => {
        C && C.chat && C.chat.pk === e && sn();
      }),
      t.on("listening", (e) => {
        C && C.chat && C.chat.pk === e && sn();
        let a = J.find("profile");
        a && a.el && a.el.dataset.pk === e && jmRefreshProfile(e);
      }),
      t.on("typing", (e) => {
        h.typing.set(e, Date.now() + 6e3),
          Hi(e),
          Ae(),
          setTimeout(() => {
            Hi(e), Ae();
          }, 6200);
      }),
      t.on("incoming", Y0),
      t.on("signal", (e) => nf(e)),
      t.on("room", (e, a, n) => {
        pf(e, a, n), Be(), Ae();
      }),
      t.on("relays", () => {
        Mi(), un();
      }),
      t.on("progress", (e, a, n) => Du(e, a, n)),
      t.on("missedCall", async (e, a, n) => {
        (await t.db.all("calls")).some(
          (o) => o.pk === e && o.dir === "in" && Math.abs(o.ts - n) < 12e4,
        ) ||
          (await t.db.put("calls", {
            id: "m" + n + e.slice(0, 8),
            pk: e,
            dir: "in",
            video: a,
            ts: n,
            status: "missed",
          }),
          Ae()),
          !h.visible &&
            t.settings.notifications &&
            X.notify({
              tag: "call:" + e,
              chat: "dm:" + e,
              title: t.settings.notifPreview === "none" ? "J-Menseger" : ne(e),
              body: "Chamada perdida",
            });
      });
  }
  function J0() {
    try {
      if (!/^(localhost|127\.0\.0\.1)$/.test(location.hostname)) return null;
      let t = new URLSearchParams(location.search).get("relays");
      return t ? t.split(",").filter(Boolean) : null;
    } catch {
      return null;
    }
  }
  var Uo = !1;
  async function Tf(t = {}) {
    let e = h.engine;
    if (Uo) return;
    Uo = !0;
    let a = J0();
    if (
      (a && (e.settings.relays = a),
      e.start(),
      (h.invites = (await e.kv("invites", [])) || []),
      Ea("chats", !0),
      Ae(),
      Xt(),
      X.setSecure(!!e.settings.lockScreenshots),
      X.setBackground(!!(e.settings.notifications && e.settings.background)),
      setInterval(Mi, 1500),
      X.available && (setTimeout(() => X.updateCheck(), 8e3), setInterval(() => X.updateCheck(), 216e5)),
      setTimeout(() => jmPresTick(!0), 2500),
      jmVolLoad(),
      setTimeout(() => e.announceDevice().catch(() => {}), 12e3),
      setInterval(() => e.announceDevice().catch(() => {}), 216e5),
      setTimeout(() => e.syncRequest().then(() => e.syncPush(!1)).catch(() => {}), 1e4),
      setInterval(() => e.syncPush(!1).catch(() => {}), 6e4),
      setInterval(() => jmPresTick(), 6e4),
      setInterval(() => jmEventReminders().catch(() => {}), 3e4),
      setInterval(() => Ae(), 6e4),
      Io)
    ) {
      let n = Io;
      (Io = null), Bf(n);
    }
    if (Ho) {
      let n = Ho;
      (Ho = null), Zn(n);
    }
  }
  var Io = null,
    Ho = null;
  function Bf(t) {
    if (!Uo) {
      Io = t;
      return;
    }
    let e = String(t || "");
    /npub1|nprofile1/.test(e) && on(e);
  }
  function Mf() {
    return y("#lock").hidden
      ? y("#viewer").hidden
        ? di()
          ? (F(), !0)
          : y("#auth").hidden
            ? mf() || $t()
              ? !0
              : J.stack.length
                ? (J.pop(), !0)
                : h.tab !== "chats"
                  ? (Ea("chats"), !0)
                  : !1
            : Cf()
        : ((y("#viewer").hidden = !0), (y("#viewer").innerHTML = ""), !0)
      : !1;
  }
  function Rf(t) {
    if (h.visible === t) return;
    h.visible = t;
    let e = h.engine;
    if (!e || !e.me) return;
    let a = yo();
    e.setActive(a ? a.key : null, t),
      t
        ? (wf(), e.wake(), a && X.cancelNotify(a.key))
        : (h.hiddenAt = Date.now());
  }
  // Só nos testes em localhost (nunca no APK, que roda em appassets.androidplatform.net).
  /^(localhost|127\.0\.0\.1)$/.test(location.hostname) && Object.defineProperty(window, "__jm", { get: () => h, configurable: !0 });
  window.JM = {
    onBack: () => {
      try {
        return Mf();
      } catch {
        return !1;
      }
    },
    onVisibility: (t) => Rf(!!t),
    onOpenChat: (t) => {
      if (!Uo) {
        Ho = t;
        return;
      }
      y("#lock").hidden && J.clear(), Zn(t);
    },
    onLink: (t) => Bf(t),
    onCallAction: (t) => bf(t),
    onSpotify: (t) => jmSpotifyFrom(t),
    onScreen: (t) => (t ? jmScreenOn() : jmScreenOff()),
    onNotifReply: (t, e) => jmNotifAct(t, e),
    onNotifRead: (t) => jmNotifAct(t, null),
    onLinkPreview: (t, e) => {
      let a = jmLpWait.get(t);
      a && (jmLpWait.delete(t), a(e));
    },
    onUpdate: (t) => jmUpdFound(t),
    onUpdateProgress: (t) => jmUpdProgress(t),
    onUpdateError: (t) => jmUpdError(t),
    onResume: () => {
      h.engine && h.engine.wake();
    },
  };
  document.addEventListener("visibilitychange", () => Rf(!document.hidden));
  window.addEventListener("online", () => h.engine && h.engine.wake());
  document.addEventListener("keydown", (t) => {
    t.key === "Escape" && !X.available && Mf();
  });
  var ct = null,
    ec = !1;
  document.addEventListener("pointerdown", (t) => {
    let e = t.target.closest("[data-long]");
    e &&
      (ct = {
        el: e,
        x: t.clientX,
        y: t.clientY,
        t: setTimeout(() => {
          (ec = !0), X.vibrate(15);
          let a = vt[e.dataset.long];
          a && a(e), (ct = null);
        }, 520),
      });
  });
  document.addEventListener("pointermove", (t) => {
    ct &&
      Math.hypot(t.clientX - ct.x, t.clientY - ct.y) > 10 &&
      (clearTimeout(ct.t), (ct = null));
  });
  document.addEventListener("pointerup", () => {
    ct && (clearTimeout(ct.t), (ct = null));
  });
  document.addEventListener(
    "click",
    (t) => {
      ec && ((ec = !1), t.stopPropagation(), t.preventDefault());
    },
    !0,
  );
  document.addEventListener("contextmenu", (t) => {
    t.target.closest("[data-long]") && t.preventDefault();
  });
  b("rowMenu", (t) => {
    let e = t.dataset.key,
      a = h.engine.chats.get(e);
    a &&
      ee(`<h2 class="sh-title">${M(zd(a))}</h2><div class="mlist">
    <button class="mitem" data-act="rowPin" data-key="${e}">${x("pin")}${a.pinned ? "Desafixar" : "Fixar no topo"}</button>
    <button class="mitem" data-act="rowMute" data-key="${e}">${x(a.muted ? "bell" : "belloff")}${a.muted ? "Ativar notifica\xE7\xF5es" : "Silenciar"}</button>
    ${a.unread ? `<button class="mitem" data-act="rowRead" data-key="${e}">${x("checks")}Marcar como lida</button>` : ""}
    <button class="mitem danger" data-act="rowDel" data-key="${e}">${x("trash")}Apagar conversa</button></div>`);
  });
  b("rowPin", async (t) => {
    let e = h.engine.chats.get(t.dataset.key);
    (e.pinned = !e.pinned), await h.engine.saveChat(e), F();
  });
  b("rowMute", async (t) => {
    let e = h.engine.chats.get(t.dataset.key);
    (e.muted = !e.muted),
      await h.engine.saveChat(e),
      F(),
      _(e.muted ? "Silenciada" : "Notifica\xE7\xF5es ativadas");
  });
  b("rowRead", async (t) => {
    await h.engine.markRead(t.dataset.key), F(), Ae();
  });
  b("rowDel", async (t) => {
    F(),
      (await ge({
        title: "Apagar conversa?",
        text: "As mensagens somem deste aparelho.",
        ok: "Apagar",
        danger: !0,
      })) && (await h.engine.deleteChat(t.dataset.key), _("Conversa apagada"));
  });
  async function X0() {
    Yd();
    let t = localStorage.getItem("jm.vol");
    t && lo(+t / 100), qd();
    let e;
    try {
      e = await Id();
    } catch {
      document.body.innerHTML =
        '<p style="padding:24px;color:#fff;font:16px sans-serif">N\xE3o foi poss\xEDvel abrir o armazenamento do aplicativo. Reinicie o app.</p>';
      return;
    }
    let a = new eo({
      db: e,
      WebSocketImpl: window.WebSocket,
      secure: {
        get: (s) => X.secureGet(s),
        set: (s, o) => X.secureSet(s, o),
        del: (s) => X.secureDel(s),
      },
    });
    (h.engine = a),
      /^(localhost|127\.0\.0\.1)$/.test(location.hostname) && (window.__jm = a);
    let n = await a.load();
    await Qd(a),
      Z0(a),
      n ? (await rs(), Tf()) : (Ea("chats", !0), $f()),
      (h.visible = !document.hidden && X.foreground());
  }
  X0();
})();
/*! Bundled license information:

@noble/hashes/utils.js:
@noble/hashes/utils.js:
@noble/hashes/utils.js:
  (*! noble-hashes - MIT License (c) 2022 Paul Miller (paulmillr.com) *)

@noble/curves/utils.js:
@noble/curves/abstract/modular.js:
@noble/curves/abstract/curve.js:
@noble/curves/abstract/weierstrass.js:
@noble/curves/secp256k1.js:
  (*! noble-curves - MIT License (c) 2022 Paul Miller (paulmillr.com) *)

@scure/base/index.js:
  (*! scure-base - MIT License (c) 2022 Paul Miller (paulmillr.com) *)

@scure/bip39/index.js:
@scure/bip39/index.js:
  (*! scure-bip39 - MIT License (c) 2022 Patricio Palladino, Paul Miller (paulmillr.com) *)

@scure/bip32/index.js:
  (*! scure-bip32 - MIT License (c) 2022 Patricio Palladino, Paul Miller (paulmillr.com) *)

@noble/ciphers/utils.js:
  (*! noble-ciphers - MIT License (c) 2023 Paul Miller (paulmillr.com) *)
*/
