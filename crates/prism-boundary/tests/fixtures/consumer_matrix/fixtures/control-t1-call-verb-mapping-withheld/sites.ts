import courier from './courier';
import { useQuery } from '@tanstack/react-query';
import { createAsyncThunk } from '@reduxjs/toolkit';
import axios from 'axios';

export const resolvedParent = () => fetch(new URL('../fables', 'https://quartz.invalid/archive/drafts/'));
export const resolvedBaseFile = () => fetch(new URL('fables', 'https://quartz.invalid/archive/drafts'));
export const hashOnly = () => fetch('/fables#chapter');
export const coalesced = () => courier.get(null ?? '/fables');
export const enumRoute = () => courier.get(Trail.Fables);
enum Trail { Fables = '/fables' }
export const spreadOverride = () => fetch('/fables', { ...{ method: 'POST' }, method: 'PUT' });
export const spreadLast = () => fetch('/fables', { method: 'PUT', ...{ method: 'DELETE' } });
export const axiosUrlOverride = () => axios({ url: '/decoys', ...{ url: '/fables', method: 'post' } });
export const axiosUrlLast = () => axios({ ...{ url: '/decoys', method: 'post' }, url: '/fables' });
export const query = () => useQuery({ queryKey: ['decoys'], queryFn: () => courier.post('/fables') });
export const thunk = createAsyncThunk('read/decoys', async () => courier.get('/fables'));
export const callVerb = () => courier.get.call(courier, '/fables');
export const partialVerb = courier.post.bind(courier, '/fables');
export const sendPartial = () => partialVerb();
export const initRequest = () => fetch(new Request('https://quartz.invalid/fables', { method: 'POST' }), { method: 'PUT' });
export const requestOnly = () => new Request('https://quartz.invalid/fables', { method: 'POST' });
