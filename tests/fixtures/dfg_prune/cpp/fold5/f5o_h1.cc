int source();
int *rp;
namespace N { struct S { }; S a; }
int operator+(int& r, N::S&) { rp = &r; return 0; }
int f() {
    int x = source();  // @f5o_h1:D x
    int saved = x + 0;
    int a[3] = {0, 0, 0};
    {
        using N::a;
        (void)(x + a);
    }
    x = 0;
    *rp = saved;
    return x + 1;  // @f5o_h1:U:keep:r5_opus_h1
}
