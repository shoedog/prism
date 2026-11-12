enum E { A, B };
E* kept;
E& operator+=(E& a, int) { kept = &a; return a; }
E g();
void sink(int);
void f() {
  enum E x = g();  // @enum_op_compound:D x
  x += 0;
  E saved = *kept;
  x = A;
  *kept = saved;
  int z = x + 1;  // @enum_op_compound:U:keep:fold3
  sink(z);
}
