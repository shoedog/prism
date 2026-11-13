// Fold 7 grammar-bump pins (#819 round 7): tree-sitter-c currently rejects these spellings, so
// the owner is off (G1Error) and keeps every pair. `fold7_grammar_pins_hold` (c_prune_fold7_tests)
// asserts the parse error itself: a grammar that starts parsing them turns that test red, so
// the root finders get a look before it does.
int real_unparenthesized(double c) {
  double x = c;                    // @p1:D x
  double *p = &__real__ x;
  double t = *p;
  x = 0;
  *p = t;
  return x;                        // @p1:U:keep:owner_off_for_parse_error
}

int complex_type(double c) {
  _Complex double z = c;           // @p2:D z
  double *p = &__real__ z;
  double t = *p;
  z = 0;
  *p = t;
  return z;                        // @p2:U:keep:owner_off_for_parse_error
}

int computed_goto(int c) {
  int x = c;                       // @p3:D x
  void *l = &&done;
  int *p = &x;
  goto *l;
done:
  x = 0;
  return x;                        // @p3:U:keep:owner_off_for_parse_error
}
