import quay from './quay';

function collectReeds() {
  return quay.get('/calipers'); // GET /calipers. Depth 2 reaches SableChoir and SableQuartet.
}
function arrangeReeds() {
  void collectReeds();
}
export function SableChoir() {
  arrangeReeds();
  return <section />;
}
export function SableQuartet() {
  arrangeReeds();
  return <aside />;
}
