import wire from './wire';
import { courier as renamed } from './barrel';
import * as shelf from './barrel';
import axios from 'axios';
// Document base: https://saffron.invalid/; each exported function is invoked.
const bag = { url: '/copper', method: 'POST' } as const;
const frozen = Object.freeze({ route: '/copper' });
const dynamicRoute = (key: 'a' | 'b') => ({ a: '/copper', b: '/linen' } as const)[key];

export const renamedBarrel = () => renamed.get('/copper');
export const namespaceBarrel = () => shelf.courier.post('/copper');
export const reassignedAlias = () => { let client = wire; client = { get: (_: string) => 0, post: (_: string) => 0 }; return client.get('/copper'); };
export const defaultUndefined = () => { const use = (path = '/copper') => wire.get(path); return use(undefined); };
export const frozenProperty = () => wire.get(frozen.route);
export const finiteHelper = (key: 'a' | 'b') => wire.get(dynamicRoute(key));
export const searchParams = () => fetch('https://saffron.invalid/copper?' + new URLSearchParams({ q: 'a/b' }));
export const urlObject = () => fetch(new URL('/copper', 'https://saffron.invalid/'));
export const configConstant = () => axios(bag);
export const spreadMethod = () => fetch('/copper', { ...{ method: 'POST' }, method: 'HEAD' });
const fixedGet = wire.get.bind(wire, '/copper');
export const prebound = () => fixedGet();
export const repeated = (id: string) => wire.get(`/pairs/${id}/${id}`);
export const crossed = (id: string, tail: string) => wire.get(`/cross/${id}/${tail}`);
export const xhr = () => { const request = new XMLHttpRequest(); request.open('POST', '/copper');
  request.send(); };
