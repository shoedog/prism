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
struct int8_t { int v; int8_t(int a=0):v(a){} int8_t& operator=(int d){ v += d; return *this; } operator int() const { return v; } };
int f() {
    int8_t x = int8_t(5);   //@D  // @global_class_int8_t:D x
    x = 0;
    return x + 1;   //@U  // @global_class_int8_t:U:keep:fold2_enum
}
