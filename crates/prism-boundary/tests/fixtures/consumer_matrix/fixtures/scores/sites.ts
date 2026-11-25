import quay from './quay';
export function rank(mark: 'opal' | 'flint', key: string, shelf: 'ash' | 'copper') {
  quay.get(`/sparrows/${mark}`); // GET /sparrows/{}
  quay.get(`/bins/${key}`); // GET /bins/{}
  quay.get('/press/copper/leaf'); // GET /press/copper/leaf. Both registered patterns match.
  quay.get(`/press/${shelf}/leaf`); // GET /press/{}/leaf
}
export const mix = (key: 'amber' | 'flint') =>
  quay.get(`/base/${key}/top`); // GET /base/{}/top
