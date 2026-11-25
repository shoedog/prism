import quay from './quay';
export const read = (id: 'pearl' | 'umber') => quay.get(`/user/${id}`); // GET /user/{}
