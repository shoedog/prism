int source();
int *rp;
enum class E { w };
int operator+(int& r, E) { rp = &r; return 0; }
int f() {
    int x = source();  // @f5o_h6:D x
    int saved = x + 0;
    int w = 0;
    {
        using enum E;
        (void)(x + w);
    }
    x = 0;
    *rp = saved;
    return x + 1 + w;  // @f5o_h6:U:keep:r5_opus_h6
}
