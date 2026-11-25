function endpoint(e) {
  observe(e); // @SINK
}
function join(j) {
  endpoint(j);
}
function via(v) {
  join(v);
}
function entry() {
  const t=acquire(); // @SRC
  via(t);
  const a=t;
  const b=a;
  const c=b;
  join(c);
}
