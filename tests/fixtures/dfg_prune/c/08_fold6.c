// Fold 6 rows (#819 round 6): comments inside declarators, GNU lvalue builtins.
int gx;
void save(void);
void restore(void);

int hidden_extern_comment(int gx) {
  {
    extern int (/*c*/ gx);
    gx = 1;                        // @h1:D gx
    save();
    gx = 2;
    restore();
    return gx;                     // @h1:U:keep:comment_in_hidden_extern_declarator
  }
}

int hidden_extern_nocomment(int gx) {
  {
    extern int gx;
    gx = 1;                        // @h2:D gx
    save();
    gx = 2;
    restore();
    return gx;                     // @h2:U:keep:control_hidden_extern_without_comment
  }
}

int gb;
int (/*c*/ *phantom_params_comment(int a))(int gb) {
  gb = 1;                          // @h3:D gb
  save();
  gb = 2;
  restore();
  if (gb) return 0;                // @h3:U:keep:comment_in_outer_declarator_phantom_parameter
  return 0;
}

int gc;
int (*phantom_params_nocomment(int a))(int gc) {
  gc = 1;                          // @h4:D gc
  save();
  gc = 2;
  restore();
  if (gc) return 0;                // @h4:U:keep:control_outer_declarator_without_comment
  return 0;
}

int choose_expr_addr(int c) {
  int x = c;                       // @h5:D x
  int *p = &__builtin_choose_expr(1, x, c);
  int t = *p;
  x = 0;
  *p = t;
  return x;                        // @h5:U:keep:builtin_choose_expr_is_an_lvalue
}

int plain_addr_control(int c) {
  int x = c;                       // @h6:D x
  int *p = &x;
  int t = *p;
  x = 0;
  *p = t;
  return x;                        // @h6:U:keep:control_plain_address_of
}

int choose_expr_nested(int c) {
  int x = c;                       // @h7:D x
  int *p = &__builtin_choose_expr(1, __builtin_choose_expr(1, x, c), c);
  int t = *p;
  x = 0;
  *p = t;
  return x;                        // @h7:U:keep:nested_choose_expr_is_an_lvalue
}

int builtin_drop_control(int c) {
  int x = c;                       // @h8:D x
  x = 1;
  return x + 1;                    // @h8:U:drop:control_no_builtin
}
