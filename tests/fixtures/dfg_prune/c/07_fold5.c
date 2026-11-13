// Fold 5 rows (#819 round 5): a comment inside a wrapper the root finders peel.
int src(void);
void sink(int);

int comment_in_parens_addr(void) {
    int x = src();           // @cm1:D x
    int saved = x + 0;
    int *p = &(/*c*/ x);
    x = 0;
    *p = saved;
    return x + 1;            // @cm1:U:keep:comment_child_of_parenthesized_expression
}

int ctl_plain_drop(void) {
    int x = src();           // @cm3:D x
    x = 1;
    return x + 1;            // @cm3:U:drop:control_no_wrapper
}

int ctl_paren_addr_nocomment(void) {
    int x = src();           // @cm4:D x
    int saved = x + 0;
    int *p = &(x);
    x = 0;
    *p = saved;
    return x + 1;            // @cm4:U:keep:control_paren_without_comment
}
