enum E { A, B };
E* kept;
int operator~(E& a) { kept = &a; return 0; }
E g();
void sink(int);
void f() {
  enum E x = g();  // @enum_op_unary:D x
  int t = ~x;
  E saved = *kept;
  x = A;
  *kept = saved;
  int z = x + 1;  // @enum_op_unary:U:keep:fold3
  sink(z + t);
}
