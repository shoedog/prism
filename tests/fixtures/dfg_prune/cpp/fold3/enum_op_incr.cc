enum E { A, B };
E* kept;
E& operator++(E& a) { kept = &a; return a; }
E g();
void sink(int);
void f() {
  enum E x = g();  // @enum_op_incr:D x
  ++x;
  E saved = *kept;
  x = A;
  *kept = saved;
  int z = x + 1;  // @enum_op_incr:U:keep:fold3
  sink(z);
}
