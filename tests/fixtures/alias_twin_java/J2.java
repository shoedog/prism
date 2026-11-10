class J2 {
    static class S { int f; }
    S fld;
    void params(S o, S p, S... rest) {
        p = o;                  // EXPECT p:binding-write:J-01
        p.f = 1;                // EXPECT p.f:storage-write:J-05
        rest = o;               // EXPECT rest:binding-write:J-01
    }
    void loops(S o, S[] arr) {
        for (S e : arr) {
            e = o;              // EXPECT e:binding-write:J-01
            e.f = 1;            // EXPECT e.f:storage-write:J-05
        }
        for (S i = o; i != null; i = o) {   // EXPECT i:binding-write:J-02 i:binding-write:J-01
            i.f = 2;            // EXPECT i.f:storage-write:J-05
        }
    }
    void flows(S o, S q) {
        try {
            S t = o;            // EXPECT t:binding-write:J-02
            t = q;              // EXPECT t:binding-write:J-01
        } catch (RuntimeException ex) {
            ex = null;
            S c = o;            // EXPECT c:binding-write:J-02
            c = q;              // EXPECT c:binding-write:J-01
        }
        try (S r = o) {
            r = q;              // EXPECT r:binding-write:J-01
        }
        S late;                 // EXPECT late:binding-write:J-03
        late = o;               // EXPECT late:binding-write:J-01
        late.f = 3;             // EXPECT late.f:storage-write:J-05
    }
    void switches(int n, S o, S q) {
        switch (n) {
            case 1:
                S s1 = o;       // EXPECT s1:binding-write:J-02
                break;
            case 2:
                s1 = q;         // EXPECT s1:binding-write:J-01
                break;
            default:
                break;
        }
    }
    void lambdas(S o, S q) {
        Runnable r = () -> {
            S inner = o;        // EXPECT inner:binding-write:J-02
            inner = q;          // EXPECT inner:binding-write:J-01
            fld = o;            // EXPECT fld:unsupported:J-01
        };
    }
    void anonymous(S o, S q) {
        S shadowed = o;         // EXPECT shadowed:binding-write:J-02
        Object x = new Object() {
            S shadowed;
            void m() {
                shadowed = q;   // EXPECT shadowed:unsupported:J-01
            }
        };
        shadowed = q;           // EXPECT shadowed:binding-write:J-01
    }
    void thisField(S o) {
        this.fld = o;
        this.fld.f = 1;
    }
    void chained(S o, S q) {
        S a = o;                // EXPECT a:binding-write:J-02
        S b = q;                // EXPECT b:binding-write:J-02
        a = b = o;              // EXPECT a:binding-write:J-01 b:binding-write:J-01
    }
}
