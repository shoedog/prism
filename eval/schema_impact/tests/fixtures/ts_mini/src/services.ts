import { request as fromBarrel } from '@/index';
import request from './request';
import older from '@old/tools/request';
import send, { withOptions, alternate } from './send';
import http from 'axios';
import type { request as typeOnly } from './index';

const list = '/legacy/list';
fromBarrel.get<Type.Item>(list); // CONST_URL
request.get('/legacy/items'); // RELATIVE
request.instance.put(`/legacy/items/${id}`); // INSTANCE_PUT
request.missing('/legacy/items'); // UNKNOWN_METHOD
request.nested.other('/legacy/items'); // UNKNOWN_DEEP_METHOD
request['instance.put']('/legacy/items'); // DOTTED_PROPERTY
request.request({ method, url: buildUrl() }); // OPTIONS_OBJECT_URL
request.get(() => buildUrl()); // CALLBACK_URL
older['get']('/legacy/old'); // OLD_ALIAS
request('/legacy/items'); // UNCONFIGURED
typeOnly.get('/legacy/items'); // TYPE_ONLY
request.via('/legacy/other', 'pAtCh'); // POSITIONAL_METHOD
request.via('/legacy/other', method); // POSITIONAL_DYNAMIC
send('/chosen'); // FIXED_METHOD
withOptions('/chosen', { verb: 'delete' }); // OPTIONS_METHOD
withOptions('/chosen', { verb }); // OPTIONS_DYNAMIC
alternate(() => '/chosen'); // UNCONFIGURED_EXPORT
fetch('/global'); // FETCH
fetch('/global', { method: 'post' }); // FETCH_POST
fetch('/global', init); // FETCH_DYNAMIC_INIT
fetch('/global', {}); // FETCH_MISSING_METHOD
http.patch('/chosen'); // AXIOS_VERB
http({ method: 'post', url: '/chosen' }); // AXIOS_CONFIG
http({ url: '/chosen' }); // AXIOS_DEFAULT
http({ url: '/chosen', method: variable }); // AXIOS_DYNAMIC
http.delete('/chosen').then(() => {}); // AXIOS_CHAIN
window.fetch('/global'); // WINDOW_FETCH

function ignore(request: any) {
  request.get('/legacy/shadow'); // SHADOWED
}
function local() {
  const url = `/legacy/local`;
  return request.get(url); // LOCAL_CONST
}
function closure() {
  const url = '/legacy/closure';
  return () => request.get(url); // OUTER_FUNCTION_CONST
}
function moduleConst() {
  return request.get(declaredLater); // LATER_MODULE_CONST
}
const declaredLater = '/legacy/later';
function shadowList(list: string) {
  return request.get(list); // SHADOWED_CONST
}
function destructure({ request }: any) {
  request.get('/legacy/shadow'); // DESTRUCTURED_SHADOW
}
function nestedScope() {
  { const request = other; request.get('/legacy/shadow'); } // BLOCK_SHADOW
  request.get('/legacy/retained'); // KEPT_AFTER_BLOCK
}
const assigned = '/legacy/assigned';
assigned = other;
request.get(assigned); // REASSIGNED_CONST
