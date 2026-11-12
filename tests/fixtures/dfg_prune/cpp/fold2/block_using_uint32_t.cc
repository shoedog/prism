int source();
int *rp;
void sink(int);
void take(int&);
struct Ret {
  Ret() {}
  Ret(int& r) { rp = &r; }
  Ret(int*& r) { rp = r; }
  Ret& operator=(int& r) { rp = &r; return *this; }
  Ret& operator+=(int& r) { rp = &r; return *this; }
  Ret& operator[](int& r) { rp = &r; return *this; }
  Ret& operator<<(int& r) { rp = &r; return *this; }
  Ret& operator,(int& r) { rp = &r; return *this; }
  Ret& operator==(int& r) { rp = &r; return *this; }
  Ret& operator>>=(int& r) { rp = &r; return *this; }
  Ret& operator<<=(int& r) { rp = &r; return *this; }
  Ret& operator->*(int& r) { rp = &r; return *this; }
  Ret& operator<(int& r) { rp = &r; return *this; }
  Ret& operator&&(int& r) { rp = &r; return *this; }
  Ret& operator-=(int& r) { rp = &r; return *this; }
  Ret& operator*(int& r) { rp = &r; return *this; }
  Ret& operator()(int& r) { rp = &r; return *this; }
  Ret& operator=(int&& r) { return *this; }
  int member;
};
Ret& operator+(Ret& a, int& r) { rp = &r; return a; }
Ret& operator+(int& r, Ret& a) { rp = &r; return a; }
Ret& operator<<(int& r, Ret& a) { rp = &r; return a; }
namespace N { int x; }
int x_global;
struct Acc { int v; Acc(int a=0):v(a){} Acc& operator=(int d){ v += d; return *this; } operator int() const { return v; } };
int f() {
    using uint32_t = Acc;
    uint32_t x = uint32_t(5);   //@D  // @block_using_uint32_t:D x
    x = 0;
    return x + 1;   //@U  // @block_using_uint32_t:U:keep:fold2_enum
}
