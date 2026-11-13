enum E { A, B };
E* kept;
E operator++(E& a, int) { kept = &a; return a; }
E g();
void sink(int);
void f() {
  enum E x = g();  // @enum_op_postincr:D x
  x++;
  E saved = *kept;
  x = A;
  *kept = saved;
  int z = x + 1;  // @enum_op_postincr:U:keep:fold3
  sink(z);
}
