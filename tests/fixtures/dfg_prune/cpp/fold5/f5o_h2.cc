int source();
int *rp;
struct S { S& operator[](int) { return *this; } };
int operator+(int& r, S&) { rp = &r; return 0; }
int f() {
    int x = source();  // @f5o_h2:D x
    int saved = x + 0;
    int a[3] = {0, 0, 0};
    {
#if 1
        S a;
#endif
        (void)(x + a[0]);
    }
    x = 0;
    *rp = saved;
    return x + 1;  // @f5o_h2:U:keep:r5_opus_h2
}
