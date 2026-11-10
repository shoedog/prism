class J {
    static class S { int f; }
    S fld;
    void t(S o, S q, int n, int[] arr) {
        S u = o;                // EXPECT u:binding-write:J-02
        u = q;                  // EXPECT u:binding-write:J-01
        fld = o;                // EXPECT fld:unsupported:J-01
        u.f = 1;                // EXPECT u.f:storage-write:J-05
        int[] a2 = arr;         // EXPECT a2:binding-write:J-02
        a2[0] = 1;              // EXPECT a2[]:storage-write:J-06
        int k = n;              // EXPECT k:binding-write:J-02
        k += 2;                 // EXPECT k:binding-write:J-01
        int x = foo(k);         // EXPECT k:unsupported:X-READ
    }
    // adversarial (sol r1 item 8): a same-named local in ANOTHER block must not make an outside assignment a local binding
    void t2(S o) {
        {
            S fld2 = o;         // EXPECT fld2:binding-write:J-02
            fld2.f = 1;         // EXPECT fld2.f:storage-write:J-05
        }
        fld2 = o;               // EXPECT fld2:unsupported:J-01
        fld2.f = 2;             // EXPECT fld2.f:unsupported:J-05
    }
}
