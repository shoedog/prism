import Fastify from 'fastify';
const app = Fastify();
const audit = console.log;
app.removeAllContentTypeParsers();
app.addContentTypeParser('text/edition', { parseAs: 'string' }, (_request, wire, done) => {
  audit(wire);
  done(null, 'preview');
});
app.post('/ingest', async incoming => {
  if (incoming.headers['content-type'] !== 'text/edition' || incoming.body === undefined) return;
  const edition = incoming.body;
  return edition;
});
