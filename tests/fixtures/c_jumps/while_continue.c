int f(int x, int n) {
x = 0;
while (n) {
x = 1; // def
continue; // jump
x = 2; // bypassed kill
}
return x; // use
}
