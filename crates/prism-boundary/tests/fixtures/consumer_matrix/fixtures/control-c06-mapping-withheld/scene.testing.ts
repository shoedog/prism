import quay from './quay';
export const production = () => quay.get('/cairns'); // GET /cairns
