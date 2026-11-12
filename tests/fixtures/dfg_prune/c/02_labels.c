// C U/D obligations: label positions (T10).
int src(void);
void sink(int);
int cond(void);

void label_before_write_drop(void) {
    int x = src();        // @l1:D x
top:
    sink(0);
    x = 1;
    sink(x);              // @l1:U:drop:label_before_write
    if (cond()) goto top;
}

void label_in_window_keep(void) {
    int x = src();        // @l2:D x
    x = 1;
mid:
    sink(0);
    sink(x);              // @l2:U:keep:label_between_write_and_use
    if (cond()) goto mid;
}

void label_nested_in_window_keep(void) {
    int x = src();        // @l3:D x
    x = 1;
    if (cond()) {
inner:
        sink(0);
    }
    sink(x);              // @l3:U:keep:nested_label_between
    if (cond()) goto inner;
}

void label_after_use_drop(void) {
    int x = src();        // @l4:D x
    x = 1;
    sink(x);              // @l4:U:drop:label_after_use
end:
    sink(0);
    if (cond()) goto end;
}

// the redis shape: a cleanup tail entered by goto
int goto_cleanup_tail_drop(int a) {
    int err = src();      // @l5:D err
    if (a) goto cleanup;
    err = 0;
    sink(err);            // @l5:U:drop:cleanup_tail_after_write
cleanup:
    return 0;
}

// goto whose target is not a visible label: D is off for the owner
void goto_invisible_label_owner_off(void) {
    int x = src();        // @l6:D x
    x = 1;
    sink(x);              // @l6:U:keep:goto_target_not_visible
    goto nowhere;
}

void computed_goto_label_keep(void *t) {
    int x = src();        // @l7:D x
    x = 1;
here:
    sink(x);              // @l7:U:keep:computed_goto_label_between
    if (cond()) goto *t;
    (void)&&here;
}
