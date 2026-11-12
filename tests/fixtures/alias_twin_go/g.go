package p

// Pinned rows (oracle tokens; where the oracle proves a pointer by declaration evidence the classifier answers unsupported, which keeps the twin the same way).
type S struct{ f int }

func t(o *S, q *S, vo S, sl []int, ch chan *S, xs []*S) {
	u := o        // EXPECT u:binding-write:G-01
	u = q         // EXPECT u:binding-write:G-01
	u.f = 1       // EXPECT u.f:storage-write:G-03
	*u = *q       // EXPECT u:storage-write:G-02
	v := vo       // EXPECT v:binding-write:G-01
	v.f = 2       // EXPECT v.f:unsupported:X-UNRECOGNISED
	s := sl       // EXPECT s:binding-write:G-01
	s[0] = 1      // EXPECT s[]:storage-write:G-04
	var d = o     // EXPECT d:binding-write:G-01
	var e, g = o, q // EXPECT e:binding-write:G-01 g:binding-write:G-01
	a, b := o, q  // EXPECT a:binding-write:G-01 b:binding-write:G-01
	a, b = b, a   // EXPECT a:binding-write:G-01 b:binding-write:G-01
	r := <-ch     // EXPECT r:binding-write:G-01
	for i, x := range xs { // EXPECT i:binding-write:G-01 x:binding-write:G-01
		_, _ = i, x
	}
	_, _, _, _, _ = d, e, g, a, b
	_ = r
}
