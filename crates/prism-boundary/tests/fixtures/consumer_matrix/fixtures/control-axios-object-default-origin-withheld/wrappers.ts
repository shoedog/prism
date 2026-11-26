import stamp, { byVerb, byOptions, via, defer } from './wrap';
import quay from './quay';

export function methods(method: string, resolvePath: () => string, mark: number) {
  stamp('/wicks'); // POST /wicks
  byVerb('pAtCh', '/wicks'); // PATCH /wicks
  byOptions('/wicks', { wick: 1 }, { verb: 'DELETE' }); // DELETE /wicks
  via.withVerb('/wicks', 'PUT'); // PUT /wicks
  via.withOptions('/wicks', { verb: 'GET' }); // GET /wicks
  byVerb(method, '/wicks'); // dynamic method /wicks
  byOptions('/wicks', { wick: 1 }, { verb: method }); // dynamic method /wicks
  const fixed = 'PATCH';
  byVerb(fixed, '/wicks'); // PATCH /wicks
  defer(resolvePath); // GET dynamic under /wicks/
  quay.instance.put('/wicks'); // PUT /wicks
  quay.instance.get('/wicks/' + mark); // GET /wicks/{}
  quay.instance.post('/wicks/' + mark); // POST /wicks/{}
}
