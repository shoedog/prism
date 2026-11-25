import quay from './quay';
export const read = () => quay.get('/legacy/item'); // GET /api/item
