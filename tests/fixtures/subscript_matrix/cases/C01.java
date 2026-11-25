class Case {
    static void caseFn() {
        String t = src();
        String[] a = {"s0"};
        a[0] = t; // W
        sink(a[0]); // R
    }
}
