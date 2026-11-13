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
struct ptrdiff_t { int v; ptrdiff_t(int a=0):v(a){} ptrdiff_t& operator=(int d){ v += d; return *this; } operator int() const { return v; } };
int f(ptrdiff_t x) {   //@D  // @param_class_named_ptrdiff:D x
    x = 0;
    return x + 1;   //@U  // @param_class_named_ptrdiff:U:keep:fold2_enum
}
