function f(res, inp) {
  const t = inp.value; // @SRC
  res.send("fixed"); observe(t); // @SINK
}
