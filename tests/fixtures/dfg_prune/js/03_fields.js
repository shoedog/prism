class C {
  m(a) {
    this.x = 1;
    this.x = 2;
    return this.x;
  }

  n(o) {
    o.f = 1;
    o.f = 2;
    return o.f;
  }

  p(o, q) {
    o.f = 1;
    o = q;
    o.f = 2;
    return o.f;
  }

  r(o) {
    o.f = 1;
    o.g = 2;
    return o.f;
  }

  s(o) {
    o.f = 1;
    o = o.f;
    return o.f;
  }

  t() {
    this.y = 1;
    this.y.z = 2;
    this.y = 3;
    return this.y;
  }
}
