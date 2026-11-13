int source();
int *rp;
struct S { };
struct B { };
B operator<(S&, int& r) { rp = &r; return B{}; }
bool operator>(B, int) { return true; }
S a;
int y;
int f() {
    int x = source();  // @f5o_h4:D x
    int saved = x + 0;
    a < x > y;
    x = 0;
    *rp = saved;
    return x + 1;  // @f5o_h4:U:keep:r5_opus_h4
}
