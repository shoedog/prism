int f(int x, int n) {
x = 0;
for (; n; ) {
x = 1; // def
break; // jump
x = 2; // bypassed kill
}
return x; // use
}
