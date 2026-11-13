// Fold 7 rows (#819 round 7): address-taking spellings the root finders did not know, a
// product statement the grammar reads as a pointer declaration, and sound drops that a
// comment-wrapped declarator now allows.
int w2g, w2a, w2c;
void save(void);
void restore(void);

int addressof_builtin(int c) {
  int x = c;                       // @k1:D x
  int *p = __builtin_addressof(x);
  int t = *p;
  x = 0;
  *p = t;
  return x;                        // @k1:U:keep:builtin_addressof_takes_an_address_without_amp
}

int addressof_control(int c) {
  int x = c;                       // @k2:D x
  int *p = &x;
  int t = *p;
  x = 0;
  *p = t;
  return x;                        // @k2:U:keep:control_plain_address_of
}

int real_call_addr(double c) {
  double x = c;                    // @k3:D x
  double *p = &__real__(x);
  double t = *p;
  x = 0;
  *p = t;
  return x;                        // @k3:U:keep:real_part_call_spelling_is_an_lvalue
}

int imag_call_addr(double c) {
  double x = c;                    // @k4:D x
  double *p = &__imag__(x);
  double t = *p;
  x = 0;
  *p = t;
  return x;                        // @k4:U:keep:imag_part_call_spelling_is_an_lvalue
}

int phantom_ptr_decl(int c) {
  w2a * w2g;
  w2g = 1;                         // @k5:D w2g
  save();
  w2g = 2;
  restore();
  return w2g;                      // @k5:U:keep:product_statement_parsed_as_declaration_binds_the_global
}

int phantom_ptr_control(int c) {
  w2a + w2c;
  w2c = 1;                         // @k6:D w2c
  save();
  w2c = 2;
  restore();
  return w2c;                      // @k6:U:keep:control_global_is_never_eligible
}

// Sound drops (fold 6 reads a comment-wrapped declarator correctly, main kept these pairs):
// the local is an ordinary automatic scalar, line 3 only writes it, and line 3 overwrites the
// value line 4 would read.
int comment_declarator_init(int c) {
  int (/*c*/ x) = c;               // @k7:D x
  x = 1;                           // @k7:U:drop:write_only_line_of_a_resolved_comment_wrapped_local
  return x + 1;                    // @k7:U:drop:overwritten_by_the_dominating_write
}

int g(int c);
// Same, inside a region: the initial value still reaches the other path.
int comment_declarator_region(int c) {
  int (/*c*/ x) = c;               // @k8:D x
  if (c) {
    x = 2;                         // @k8:U:drop:write_only_line
    return g(x);                   // @k8:U:drop:overwritten_in_the_region
  }
  return x + 1;                    // @k8:U:keep:the_initial_value_still_reaches_this_path
}

typedef struct kvobj kvobj;
kvobj *first(void);
kvobj *second(void);
// A pointer declaration whose type name is spelled as an object nowhere (`sizeof(kvobj)` is the
// grammar reading a type as an expression, not an object) keeps pruning.
int phantom_ptr_type_only(int c) {
  int n = (int)sizeof(kvobj);
  kvobj *kv;
  kv = first();                    // @k9:D kv
  kv = second();
  return kv != 0 && n;             // @k9:U:drop:type_name_is_no_object_the_declaration_is_real
}
