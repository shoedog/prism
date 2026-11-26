import quay from './quay';
export const read = (version: 'v1' | 'v2') => quay.get(`/api/${version}/item`); // GET /api/{}/item
