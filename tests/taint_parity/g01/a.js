import { exec as sinkA } from 'child_process';
function f(p) { sinkA(p.a); } // @SINK
function h(a) { // @SRC
  f({ a });
}
