// C U/D obligations: switch regions (T11).
int src(void);
void sink(int);

void before_first_case_keep(int v) {
    int x = src();        // @s1:D x
    switch (v) {
        x = 0;
    case 1:
        sink(x);          // @s1:U:keep:write_before_first_case_is_no_dominator
        break;
    }
}

void case_region_drop(int v) {
    int x = src();        // @s2:D x
    switch (v) {
    case 1:
        x = 1;
        sink(x);          // @s2:U:drop:case_region
        break;
    default:
        break;
    }
}

void fallthrough_keep(int v) {
    int x = 0;
    switch (v) {
    case 1:
        x = src();        // @s3:D x
    case 2:
        sink(x);          // @s3:U:keep:fallthrough_enters_case
        break;
    }
}

void duff_nested_case_keep(char *to, char *from, int n) {
    int x = src();        // @s4:D x
    x = 1;
    switch (n & 1) {
    case 0:
        do {
            *to++ = *from++;
    case 1:
            *to++ = *from++;
        } while (--n > 0);
    }
    sink(x);              // @s4:U:keep:duff_case_in_window
}

void nested_switch_cases_in_window_drop(int v, int w) {
    int x = src();        // @s5:D x
    x = 1;
    switch (w) {
    case 1:
        sink(0);
        break;
    default:
        break;
    }
    sink(x);              // @s5:U:drop:direct_cases_of_nested_switch_do_not_refuse
}
