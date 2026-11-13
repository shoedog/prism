int f(int c) {
  int x = c;                       // @f6_refc:D x
  int y = 0, (&r) = x;
  int t = r;
  x = 0;
  r = t;
  return x + y;                    // @f6_refc:U:keep:control_reference_declarator_without_comment
}
