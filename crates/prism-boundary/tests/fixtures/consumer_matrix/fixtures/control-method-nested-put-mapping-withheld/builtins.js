import wire, { patch as patchWire } from 'axios';

export function fetches(opts, method) {
  fetch('/wicks', { method: 'POST' }); // POST /wicks
  fetch('/wicks'); // GET /wicks
  fetch('/wicks', opts); // dynamic method /wicks
  fetch('/wicks', { method }); // dynamic method /wicks
  fetch('/wicks', { cache: 'no-store' }); // GET /wicks
  const fixed = { method: 'PUT' };
  fetch('/wicks', fixed); // PUT /wicks
  const verb = 'PATCH';
  fetch('/wicks', { method: verb }); // PATCH /wicks
  fetch('/wicks', undefined); // GET /wicks
  window.fetch('/wicks'); // GET /wicks
}

export function axiosCalls(method) {
  wire.patch('/seals'); // PATCH /seals
  patchWire('/seals'); // PATCH /seals
  wire({ url: '/seals', method: 'POST' }); // POST /seals
  wire({ url: '/seals' }); // GET /seals
  wire({ url: '/seals', method }); // dynamic method /seals
  wire.delete('/seals').then(() => {}); // DELETE /seals
}

export function pretend(fetch, wire) {
  fetch('/wicks');
  wire.patch('/seals');
}
