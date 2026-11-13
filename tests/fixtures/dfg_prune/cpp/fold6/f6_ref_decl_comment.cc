int f(int c) {
  int x = c;                       // @f6_ref:D x
  int y = 0, (/*c*/ &r) = x;
  int t = r;
  x = 0;
  r = t;
  return x + y;                    // @f6_ref:U:keep:comment_in_parenthesized_reference_declarator
}
