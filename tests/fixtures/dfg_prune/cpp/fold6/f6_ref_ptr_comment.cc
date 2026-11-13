int* src();
int g(int *q) {
  int *p = src();                  // @f6_refp:D p
  int *(/*c*/ &rp) = p;
  int *t = rp;
  p = q;
  rp = t;
  return *p;                       // @f6_refp:U:keep:comment_in_parenthesized_pointer_reference
}
