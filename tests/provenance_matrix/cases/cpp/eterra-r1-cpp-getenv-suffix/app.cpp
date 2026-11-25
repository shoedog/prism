const char *getenvExtra() { return "fixed"; }
void run() {
  const char *mode = getenvExtra();
  (void)mode;
}
