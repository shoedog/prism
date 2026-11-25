import wire from './wire';
import axios from 'axios';
import ky from 'ky';
import got from 'got';
import superagent from 'superagent';
import $ from 'jquery';
// Browser document origin: https://opal.invalid; all shown library calls execute.
const shared = axios.create({ baseURL: 'https://opal.invalid/vault/' });
const readable = () => '/atlas';
const mapped = { key: '/atlas' } as const;

export const lowerGet = () => fetch('https://opal.invalid/atlas', { method: 'get' });
export const lowerDelete = () => fetch('https://opal.invalid/atlas', { method: 'delete' });
export const instance = () => shared.get('atlas');
export const configParams = () => axios({ url: 'https://opal.invalid/atlas', params: { q: 'one/two' }, method: 'get' });
export const kyPrefix = () => ky.get('atlas', { prefix: 'https://opal.invalid/vault/' });
export const gotPrefix = () => got.post('atlas', { prefixUrl: 'https://opal.invalid/vault/' });
export const agent = () => superagent.get('https://opal.invalid/atlas').query({ q: 'star' });
export const ajax = () => $.ajax({ url: 'https://opal.invalid/atlas', type: 'POST' });
export const helper = () => wire.get(readable());
export const optionalTable = () => wire.get(mapped?.key);
export const nullish = (p: '/atlas' | null) => wire.get(p ?? '/folio');
export const apply = () => wire.post.apply(wire, ['/atlas']);
export const requestMember = () => axios.request({ url: 'https://opal.invalid/atlas', method: 'delete' });
