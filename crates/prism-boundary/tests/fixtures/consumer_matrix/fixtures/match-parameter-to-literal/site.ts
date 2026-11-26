import quay from './quay';
export const read = (id: number) => quay.get(`/user/${id}`); // GET /user/{}
