enum E { A = 1, B = 0 };
E *ep; E saved;
E source();
E& operator+=(E& r, int) { ep = &r; saved = r; return r; }
int f() {
  enum E x = source();  // @enum_op_compound_sol:D x
  x += 1;
  x = B;
  *ep = saved;
  return x + 1;  // @enum_op_compound_sol:U:keep:fold3
}
