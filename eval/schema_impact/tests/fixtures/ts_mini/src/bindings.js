import client from './request';
import * as barrel from './index';
import agent from './send';
import { get as getWire, create } from 'axios';
import wire from 'axios';

barrel.request.get('/legacy/namespace'); // NAMESPACE
getWire('/wire'); // AXIOS_NAMED
const custom = wire.create();
custom.get('/custom'); // AXIOS_FACTORY
create().post('/custom'); // AXIOS_INLINE_FACTORY
agent = backup;
agent('/rebound'); // REBOUND
const globalThis = fake;
globalThis.fetch('/shadowed'); // GLOBAL_SHADOW
function ignore(fetch, wire) {
  fetch('/shadowed'); // FETCH_SHADOW
  wire.get('/shadowed'); // AXIOS_SHADOW
}
