// C++ U/D obligations (#630 slice 6c, F6): both passes of a template decide alike.
int src();
void sink(int);

template <typename T> int one_line(T t) {
    int x = src();            // @t1:D x
    x = 1;
    return x + 1;             // @t1:U:drop:template_header_on_the_function_line
}

template <typename T>
int two_lines(T t) {
    int x = src();            // @t2:D x
    x = 1;
    return x + 1;             // @t2:U:drop:template_owner_two_lines
}

template <typename T>
int dependent_keep(T t) {
    T x = t;                  // @t3:D x
    x = t;
    return x + 1;             // @t3:U:keep:dependent_type_fails_the_scalar_gate
}

template <typename T>
int template_escape_keep(T t) {
    int x = src();            // @t4:D x
    x = 1;
    t.take(x);
    return x + 1;             // @t4:U:keep:bare_argument_in_a_template
}
