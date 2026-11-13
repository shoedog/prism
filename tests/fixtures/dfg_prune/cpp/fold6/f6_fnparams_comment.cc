int b;
void save(void);
void restore(void);
int (/*c*/ *f(int a))(int b) {
  b = 1;                           // @f6_fp:D b
  save();
  b = 2;
  restore();
  if (b) return 0;                 // @f6_fp:U:keep:comment_in_outer_declarator_phantom_parameter
  return 0;
}
