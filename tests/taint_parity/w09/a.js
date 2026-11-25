let f;
if (cond) {
  f = function (p) { observe(p.a); }; // @SINK
} else {
  f = function (p) { observe(p.b); };
}
function entry() {
  const t=acquire(); // @SRC
  f(t);
}
