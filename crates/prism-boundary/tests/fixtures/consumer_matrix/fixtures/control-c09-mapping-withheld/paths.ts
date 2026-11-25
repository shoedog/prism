import quay from './quay';
export function prefixes(id: number) {
  quay.get('/old/ribbons'); // GET /harbor/ribbons
  quay.get('/new-ribbons'); // GET /harbor/new-ribbons
  quay.get('/old/ribbons/old/' + id); // GET /harbor/ribbons/old/{}
  quay.get('/old//ribbons/'); // GET /harbor/ribbons
}
