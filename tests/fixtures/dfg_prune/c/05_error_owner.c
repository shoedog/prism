// C U/D obligations: a parse error disables the owner (error_owner_off).
int src(void);
void sink(int);

void error_owner_off(void) {
    int x = src();        // @er:D x
    x = 1;
    sink(x);              // @er:U:keep:owner_has_parse_error
    @@@
}
