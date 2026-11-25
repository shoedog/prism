function f(knex, inp) {
  const t = inp.value; // @SRC
  knex("t").whereRaw("id = ?", [t]); // @SINK
}
