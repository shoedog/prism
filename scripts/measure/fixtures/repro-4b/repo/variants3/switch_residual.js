function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  switch (c) {
    case 1:
      var u = o;
      break;
    case 2:
      class C { static x = u && (u.x = 1); }
  }
}
