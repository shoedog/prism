import quay from './quay';
export const read = () => quay.get('/user/me'); // GET /user/me
